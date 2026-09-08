use std::future::{self, Future};
use std::net::TcpListener as StdTcpListener;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::task::{Context, Poll};

use dashmap::DashMap;
use futures_util::future::Either;
use mlua::{ExternalResult, FromLuaMulti, Function, IntoLuaMulti, Lua, Result, Table, Value};
use rustc_hash::FxBuildHasher;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot::{self, Receiver};

use crate::async_pool::{ObjectPool, PER_WORKER_POOL_SIZE};

type FutureId = u64;

// Link between future id and the corresponding receiver (used to signal when the future is ready)
static FUTURE_RX_MAP: OnceLock<DashMap<FutureId, Receiver<()>, FxBuildHasher>> = OnceLock::new();

pub use crate::runtime::runtime;

fn get_notification_listener() -> &'static StdTcpListener {
    static NOTIFICATION_LISTENER: OnceLock<StdTcpListener> = OnceLock::new();
    NOTIFICATION_LISTENER.get_or_init(|| {
        StdTcpListener::bind("127.0.0.1:0").expect("failed to bind to a local port")
    })
}

fn get_notification_port() -> u16 {
    get_notification_listener()
        .local_addr()
        .expect("failed to get local address")
        .port()
}

fn get_rx_by_future_id(future_id: FutureId) -> Option<Receiver<()>> {
    FUTURE_RX_MAP.get()?.remove(&future_id).map(|(_, rx)| rx)
}

fn set_rx_by_future_id(future_id: FutureId, rx: Receiver<()>) {
    FUTURE_RX_MAP
        .get_or_init(|| DashMap::with_capacity_and_hasher(256, FxBuildHasher))
        .insert(future_id, rx);
}

// Returns a next future id (and starts the notification task if it's not running yet)
fn get_future_id() -> FutureId {
    static WATCHER: OnceLock<()> = OnceLock::new();
    WATCHER.get_or_init(|| {
        let listener = get_notification_listener()
            .try_clone()
            .expect("failed to clone notification listener");
        listener
            .set_nonblocking(true)
            .expect("failed to configure notification listener");
        let listener = TcpListener::from_std(listener)
            .expect("failed to configure async notification listener");

        // Spawn notification task (it responds to subscribe requests and signal when the future is ready)
        runtime().spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::task::spawn(async move {
                    let (reader, mut writer) = stream.split();
                    let reader = BufReader::new(reader);
                    let mut lines = reader.lines();
                    // Read future id from the stream and wait for the future to be ready
                    while let Ok(Some(line)) = lines.next_line().await {
                        let line = line.trim();
                        if line == "PING" {
                            if writer.write_all(b"PONG\n").await.is_err() {
                                break;
                            }
                            continue;
                        }
                        if let Ok(future_id) = line.parse::<FutureId>() {
                            // Wait for the future to be ready before sending the signal
                            let resp: &[u8] = match get_rx_by_future_id(future_id) {
                                Some(rx) => {
                                    _ = rx.await;
                                    b"READY\n"
                                }
                                None => b"ERR\n",
                            };
                            if writer.write_all(resp).await.is_err() {
                                break;
                            }
                        }
                    }
                });
            }
        });
    });

    // Future id generator
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("future id space exhausted")
}

/// Creates a new async function that can be used in HAProxy configuration.
///
/// Tokio runtime is automatically configured to use multiple threads.
pub fn create_async_function<F, A, R, FR>(lua: &Lua, func: F) -> Result<Function>
where
    F: Fn(A) -> FR + 'static,
    A: FromLuaMulti + 'static,
    R: IntoLuaMulti + Send + 'static,
    FR: Future<Output = Result<R>> + Send + 'static,
{
    let port = get_notification_port();
    let yield_fixup = YieldFixUp::new(lua, port)?;
    lua.create_async_function(move |lua, args| {
        let _ = &yield_fixup;
        // New future id must be generated on each invocation
        let future_id = get_future_id();

        // Spawn the future in background
        let _guard = runtime().enter();
        let args = match A::from_lua_multi(args, &lua) {
            Ok(args) => args,
            Err(err) => return Either::Left(future::ready(Err(err))),
        };
        let (tx, rx) = oneshot::channel();
        set_rx_by_future_id(future_id, rx);
        let fut = func(args);
        let result = tokio::task::spawn(async move {
            let result = fut.await;
            // Signal that the future is ready
            let _ = tx.send(());
            result
        });

        Either::Right(HaproxyFuture {
            lua,
            id: future_id,
            fut: async move { result.await.into_lua_err()? },
        })
    })
}

const YIELD_ORIGINAL_KEY: &str = "__HAPROXY_ASYNC_ORIGINAL_YIELD";
const YIELD_WRAPPER_KEY: &str = "__HAPROXY_ASYNC_YIELD_WRAPPER";
const YIELD_USERS_KEY: &str = "__HAPROXY_ASYNC_YIELD_USERS";

struct YieldFixUp(Lua);

impl YieldFixUp {
    fn new(lua: &Lua, port: u16) -> Result<Self> {
        let connection_pool =
            match lua.named_registry_value::<Value>("__HAPROXY_CONNECTION_POOL")? {
                Value::Nil => {
                    let connection_pool = ObjectPool::new(PER_WORKER_POOL_SIZE);
                    let connection_pool = lua.create_userdata(connection_pool)?;
                    lua.set_named_registry_value("__HAPROXY_CONNECTION_POOL", &connection_pool)?;
                    Value::UserData(connection_pool)
                }
                connection_pool => connection_pool,
            };

        let active: Option<u64> = lua.named_registry_value(YIELD_USERS_KEY)?;
        if let Some(active) = active {
            let active = active.checked_add(1).ok_or_else(|| {
                mlua::Error::RuntimeError("async yield user count exhausted".into())
            })?;
            lua.set_named_registry_value(YIELD_USERS_KEY, active)?;
            return Ok(Self(lua.clone()));
        }

        let coroutine: Table = lua.globals().get("coroutine")?;
        let orig_yield: Function = coroutine.get("yield")?;
        let new_yield: Function = lua
            .load(
                r#"
                local port, connection_pool = ...
                local msleep = core.msleep
                return function()
                    -- It's important to cache the future id before first yielding point
                    local future_id = __RUST_ACTIVE_FUTURE_ID
                    local ok, err

                    -- Get new or existing connection from the pool
                    local sock = connection_pool:get()
                    if not sock then
                        sock = core.tcp()
                        ok, err = sock:connect("127.0.0.1", port)
                        if err ~= nil then
                            msleep(1)
                            return
                        end
                    end

                    -- Subscribe to the future updates
                    ok, err = sock:send(future_id .. "\n")
                    if err ~= nil then
                        sock:close()
                        msleep(1)
                        return
                    end

                    -- Wait for the future to be ready
                    ok, err = sock:receive("*l")
                    if err ~= nil then
                        sock:close()
                        msleep(1)
                        return
                    end
                    if ok ~= "READY" then
                        msleep(1)
                    end

                    ok = connection_pool:put(sock)
                    if not ok then
                        sock:close()
                    end
                end
            "#,
            )
            .call((port, connection_pool))?;
        lua.set_named_registry_value(YIELD_ORIGINAL_KEY, &orig_yield)?;
        lua.set_named_registry_value(YIELD_WRAPPER_KEY, &new_yield)?;
        lua.set_named_registry_value(YIELD_USERS_KEY, 1u64)?;
        if let Err(error) = coroutine.set("yield", new_yield) {
            let _ = lua.set_named_registry_value(YIELD_USERS_KEY, Value::Nil);
            let _ = lua.set_named_registry_value(YIELD_WRAPPER_KEY, Value::Nil);
            let _ = lua.set_named_registry_value(YIELD_ORIGINAL_KEY, Value::Nil);
            return Err(error);
        }
        Ok(YieldFixUp(lua.clone()))
    }
}

impl Drop for YieldFixUp {
    fn drop(&mut self) {
        if let Err(e) = (|| {
            let active: Option<u64> = self.0.named_registry_value(YIELD_USERS_KEY)?;
            let Some(active) = active else {
                return Ok(());
            };
            if active > 1 {
                self.0
                    .set_named_registry_value(YIELD_USERS_KEY, active - 1)?;
                return Ok(());
            }

            let coroutine: Table = self.0.globals().get("coroutine")?;
            let current: Value = coroutine.get("yield")?;
            let wrapper: Function = self.0.named_registry_value(YIELD_WRAPPER_KEY)?;
            if let Value::Function(current) = current {
                if current.to_pointer() == wrapper.to_pointer() {
                    let original: Function = self.0.named_registry_value(YIELD_ORIGINAL_KEY)?;
                    coroutine.set("yield", original)?;
                }
            }
            self.0
                .set_named_registry_value(YIELD_USERS_KEY, Value::Nil)?;
            self.0
                .set_named_registry_value(YIELD_WRAPPER_KEY, Value::Nil)?;
            self.0
                .set_named_registry_value(YIELD_ORIGINAL_KEY, Value::Nil)
        })() {
            eprintln!("Error in YieldFixUp destructor: {e}");
        }
    }
}

pin_project_lite::pin_project! {
    struct HaproxyFuture<F> {
        lua: Lua,
        id: FutureId,
        #[pin]
        fut: F,
    }
}

impl<F, R> Future for HaproxyFuture<F>
where
    F: Future<Output = Result<R>>,
{
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.fut.poll(cx) {
            Poll::Ready(res) => Poll::Ready(res),
            Poll::Pending => {
                // Set the active future id so the mlua async helper will be able to wait on it
                let _ = (this.lua.globals()).raw_set("__RUST_ACTIVE_FUTURE_ID", *this.id);
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn async_function_keeps_yield_fixup_alive() {
        let lua = Lua::new();
        let core = lua.create_table().unwrap();
        core.set("msleep", lua.create_function(|_, _: u64| Ok(())).unwrap())
            .unwrap();
        lua.globals().set("core", core).unwrap();
        lua.load("yield_before = coroutine.yield").exec().unwrap();

        let _function =
            create_async_function(&lua, |()| async { Ok::<_, mlua::Error>(()) }).unwrap();
        let replaced: bool = lua
            .load("return coroutine.yield ~= yield_before")
            .eval()
            .unwrap();

        assert!(replaced);
    }

    #[test]
    fn async_yield_fixup_survives_multiple_registrations() {
        let lua = Lua::new();
        let core = lua.create_table().unwrap();
        core.set("msleep", lua.create_function(|_, _: u64| Ok(())).unwrap())
            .unwrap();
        lua.globals().set("core", core).unwrap();

        let original: Function = lua.load("return coroutine.yield").eval().unwrap();
        let first = create_async_function(&lua, |()| async { Ok::<_, mlua::Error>(()) }).unwrap();
        let wrapper: Function = lua
            .globals()
            .get::<Table>("coroutine")
            .unwrap()
            .get("yield")
            .unwrap();
        let second = create_async_function(&lua, |()| async { Ok::<_, mlua::Error>(()) }).unwrap();

        drop(first);
        lua.gc_collect().unwrap();
        let after_first: Function = lua
            .globals()
            .get::<Table>("coroutine")
            .unwrap()
            .get("yield")
            .unwrap();
        assert_eq!(after_first.to_pointer(), wrapper.to_pointer());

        drop(second);
        lua.gc_collect().unwrap();
        let after_last: Function = lua
            .globals()
            .get::<Table>("coroutine")
            .unwrap()
            .get("yield")
            .unwrap();
        assert_eq!(after_last.to_pointer(), original.to_pointer());
    }

    #[tokio::test]
    async fn async_hook_yield_preserves_stack() -> Result<()> {
        use std::future::{poll_fn, Future};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::task::Poll;

        let lua = Lua::new();
        let thread = lua.create_thread(
            lua.load(
                r#"
                local x = 40
                local y = 2
                return x + y
            "#,
            )
            .into_function()?,
        )?;

        let yielded = std::sync::Arc::new(AtomicBool::new(false));
        let yielded2 = yielded.clone();
        thread.set_hook(mlua::HookTriggers::EVERY_LINE, move |lua, debug| {
            if debug.current_line() == Some(4) && !yielded2.swap(true, Ordering::Relaxed) {
                lua.remove_hook();
                return Ok(mlua::VmState::Yield);
            }
            Ok(mlua::VmState::Continue)
        })?;

        let mut thread = Box::pin(thread.into_async::<i32>(())?);
        poll_fn(|cx| {
            assert!(thread.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert!(yielded.load(Ordering::Relaxed));
        lua.gc_collect()?;
        assert_eq!(thread.await?, 42);

        Ok(())
    }
}
