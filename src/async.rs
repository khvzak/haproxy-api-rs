use std::future::{self, Future};
use std::net::TcpListener as StdTcpListener;
use std::pin::Pin;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::OnceLock;
use std::task::{Context, Poll};

use dashmap::DashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::sync::oneshot::{self, Receiver};

use futures_util::future::Either;
use mlua::{
    ExternalResult, FromLuaMulti, Function, IntoLuaMulti, Lua, RegistryKey, Result, Table,
    UserData, UserDataMethods, Value,
};
use rustc_hash::FxBuildHasher;

// Using `u16` will give us max 65536 receivers to store.
// If for any reason future was not picked up by the notification listener,
// receiver will be overwritten on the counter reset (and memory released).
type FutureId = u16;

// Number of open connections to the notification server
const PER_WORKER_POOL_SIZE: usize = 512;

// Link between future id and the corresponding receiver (used to signal when the future is ready)
static FUTURE_RX_MAP: OnceLock<DashMap<FutureId, Receiver<()>, FxBuildHasher>> = OnceLock::new();

/// Returns the global tokio runtime.
pub fn runtime() -> &'static runtime::Runtime {
    static RUNTIME: OnceLock<runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

// Find first free port
fn get_notification_port() -> u16 {
    static NOTIFICATION_PORT: OnceLock<u16> = OnceLock::new();
    *NOTIFICATION_PORT.get_or_init(|| {
        StdTcpListener::bind("127.0.0.1:0")
            .expect("failed to bind to a local port")
            .local_addr()
            .expect("failed to get local address")
            .port()
    })
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
        let port = get_notification_port();

        // Spawn notification task (it responds to subscribe requests and signal when the future is ready)
        runtime().spawn(async move {
            let listener = TcpListener::bind(("127.0.0.1", port))
                .await
                .unwrap_or_else(|err| panic!("failed to bind to a port {port}: {err}"));

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
    static NEXT_ID: AtomicU16 = AtomicU16::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Creates a new async function that can be used in HAProxy configuration.
///
/// Tokio runtime is automatically configured to use multiple threads.
pub fn create_async_function<'lua, A, R, F, FR>(lua: &'lua Lua, func: F) -> Result<Function<'lua>>
where
    A: FromLuaMulti<'lua> + 'static,
    R: IntoLuaMulti<'lua> + Send + 'static,
    F: Fn(A) -> FR + 'static,
    FR: Future<Output = Result<R>> + Send + 'static,
{
    let port = get_notification_port();
    let _yield_fixup = YieldFixUp::new(lua, port)?;
    lua.create_async_function(move |lua, args| {
        // New future id must be generated on each invocation
        let future_id = get_future_id();

        // Spawn the future in background
        let _guard = runtime().enter();
        let args = match A::from_lua_multi(args, lua) {
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

struct YieldFixUp<'lua>(&'lua Lua, Function<'lua>);

impl<'lua> YieldFixUp<'lua> {
    fn new(lua: &'lua Lua, port: u16) -> Result<Self> {
        let connection_pool =
            match lua.named_registry_value::<Value>("__HAPROXY_CONNECTION_POOL")? {
                Value::Nil => {
                    let connection_pool = ObjectPool::new(PER_WORKER_POOL_SIZE)?;
                    let connection_pool = lua.create_userdata(connection_pool)?;
                    lua.set_named_registry_value("__HAPROXY_CONNECTION_POOL", &connection_pool)?;
                    Value::UserData(connection_pool)
                }
                connection_pool => connection_pool,
            };

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
        coroutine.set("yield", new_yield)?;
        Ok(YieldFixUp(lua, orig_yield))
    }
}

impl<'lua> Drop for YieldFixUp<'lua> {
    fn drop(&mut self) {
        if let Err(e) = (|| {
            let coroutine: Table = self.0.globals().get("coroutine")?;
            coroutine.set("yield", &self.1)
        })() {
            println!("Error in YieldFixUp destructor: {}", e);
        }
    }
}

struct ObjectPool(Vec<RegistryKey>);

impl ObjectPool {
    fn new(capacity: usize) -> Result<Self> {
        Ok(ObjectPool(Vec::with_capacity(capacity)))
    }
}

impl UserData for ObjectPool {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("get", |_, this, ()| Ok(this.0.pop()));

        methods.add_method_mut("put", |_, this, obj: RegistryKey| {
            if this.0.len() == PER_WORKER_POOL_SIZE {
                return Ok(false);
            }
            this.0.push(obj);
            Ok(true)
        });
    }
}

pin_project_lite::pin_project! {
    struct HaproxyFuture<'lua, F> {
        lua: &'lua Lua,
        id: FutureId,
        #[pin]
        fut: F,
    }
}

impl<F, R> Future for HaproxyFuture<'_, F>
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
