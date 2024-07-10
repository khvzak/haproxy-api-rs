use std::collections::HashMap;
use std::future::{self, Future};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::task::{Context, Poll};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::sync::{mpsc, oneshot};

use futures_util::future::Either;
use mlua::{
    ExternalResult, FromLuaMulti, Function, IntoLuaMulti, Lua, RegistryKey, Result, Table,
    TableExt, Value,
};
use rustc_hash::{FxBuildHasher, FxHashMap};

type FutureId = u32;

// Channel to send future id to the socket
#[derive(Clone, Debug)]
struct FutureNotifier(mpsc::Sender<FutureId>);

impl Deref for FutureNotifier {
    type Target = mpsc::Sender<FutureId>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Max size of the pool of channels
const POOL_MAX_SIZE: usize = 512;

// Pool of channels
struct Pool(Vec<RegistryKey>);

struct FutureChannelMap(FxHashMap<FutureId, RegistryKey>);

// Future id generator
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

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

// Starts a tokio runtime and spawns a background task to receive "ready "notifications
// from futures and re-send them to the socket
fn get_or_init_notifier(lua: &Lua) -> FutureNotifier {
    if let Some(sender) = lua.app_data_ref::<FutureNotifier>() {
        return sender.clone();
    }

    let (port_tx, port_rx) = oneshot::channel::<u16>();
    // Spawn notification task (it sends data received from a future via channel to the socket)
    let (tx, mut rx) = mpsc::channel::<FutureId>(1024);
    runtime().spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind to a port");
        let port = listener
            .local_addr()
            .expect("failed to get local address")
            .port();
        port_tx.send(port).expect("failed to send port information");

        while let Ok((mut stream, _)) = listener.accept().await {
            while let Some(future_id) = rx.recv().await {
                // Send the future id to the socket
                // When haproxy receive it, it will find an associated coroutine and wake it up
                if (stream.write_all(format!("{future_id}\n").as_bytes()).await).is_err() {
                    // If the socket is closed, wait for a new connection
                    break;
                }
            }
        }
    });

    // Wait for the port to be bound
    let port = port_rx
        .blocking_recv()
        .expect("failed to receive port information");

    // Start haproxy task on this worker thread to receive notifications
    // send over the socket (see above)
    (|| -> Result<()> {
        let future_wake_up = lua.create_function(|lua, future_id: Value| {
            if let Ok(future_id) = lua.unpack::<FutureId>(future_id) {
                let future2channel = lua.app_data_ref::<FutureChannelMap>().unwrap();
                if let Some(channel_key) = future2channel.0.get(&future_id) {
                    lua.registry_value::<Table>(channel_key)?
                        .call_method::<_, ()>("push", true)?;
                }
            }
            Ok(())
        })?;

        lua.load(
            r#"
            local port, future_wake_up = ...
            core.register_task(function()
                while true do
                    local socket = core.tcp()
                    local ok = socket:connect("127.0.0.1", port)
                    if not ok then
                        core.Alert("Failed to connect to the notification socket")
                        return
                    end
                    while true do
                        local future_id = socket:receive("*l")
                        future_wake_up(future_id)
                    end
                end
            end)
        "#,
        )
        .call::<_, ()>((port, future_wake_up))
    })()
    .expect("failed to register a worker task");

    let notifier = FutureNotifier(tx);
    lua.set_app_data(notifier.clone());
    notifier
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
    let _yield_fixup = YieldFixUp::new(lua)?;
    lua.create_async_function(move |lua, args| {
        let notifier = get_or_init_notifier(lua);
        let (future_id, channel) = (|| -> Result<_> {
            // Try to get a channel from the pool or create a new one
            let channel_key = {
                let mut pool = match lua.app_data_mut::<Pool>() {
                    Some(pool) => pool,
                    None => {
                        lua.set_app_data(Pool(Vec::with_capacity(64)));
                        lua.app_data_mut().unwrap()
                    }
                };
                match pool.0.pop() {
                    Some(q) => q,
                    None => {
                        drop(pool);
                        let globals = lua.globals();
                        let core: Table = globals.raw_get("core")?;
                        core.call_function::<_, Table>("queue", ())
                            .and_then(|v| lua.create_registry_value(v))?
                    }
                }
            };

            let future_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let channel: Table = lua.registry_value(&channel_key)?;
            let mut future2channel = match lua.app_data_mut::<FutureChannelMap>() {
                Some(map) => map,
                None => {
                    let map =
                        FutureChannelMap(HashMap::with_capacity_and_hasher(64, FxBuildHasher));
                    lua.set_app_data(map);
                    lua.app_data_mut().unwrap()
                }
            };
            future2channel.0.insert(future_id, channel_key);

            Ok((future_id, channel))
        })()
        .expect("failed to generate future id");

        // Spawn the future
        let _guard = runtime().enter();
        let args = match A::from_lua_multi(args, lua) {
            Ok(args) => args,
            Err(err) => return Either::Left(future::ready(Err(err))),
        };
        let fut = func(args);
        let result = tokio::task::spawn(async move {
            let result = fut.await;
            let _ = notifier.send(future_id).await;
            result
        });

        Either::Right(HaproxyFuture {
            lua,
            channel,
            id: future_id,
            fut: async move { result.await.into_lua_err()? },
        })
    })
}

struct YieldFixUp<'lua>(&'lua Lua, Function<'lua>);

impl<'lua> YieldFixUp<'lua> {
    fn new(lua: &'lua Lua) -> Result<Self> {
        let coroutine: Table = lua.globals().get("coroutine")?;
        let orig_yield: Function = coroutine.get("yield")?;
        let new_yield: Function = lua
            .load(
                r#"
                local active_channel = __HAPROXY_ACTIVE_CHANNEL
                if active_channel ~= nil then
                    active_channel:pop_wait()
                else
                    core.msleep(1)
                end
            "#,
            )
            .into_function()?;
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

pin_project_lite::pin_project! {
    struct HaproxyFuture<'lua, F> {
        lua: &'lua Lua,
        channel: Table<'lua>,
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
            Poll::Ready(res) => {
                // Release channel to the pool
                let mut pool = this.lua.app_data_mut::<Pool>().unwrap();
                let mut future2channel = this.lua.app_data_mut::<FutureChannelMap>().unwrap();
                if let Some(chan) = future2channel.0.remove(this.id) {
                    if pool.0.len() < POOL_MAX_SIZE {
                        pool.0.push(chan);
                    }
                }

                Poll::Ready(res)
            }
            Poll::Pending => {
                // Set the active queue so the mlua async helper will be able to wait on it
                let _ = (this.lua.globals()).raw_set("__HAPROXY_ACTIVE_CHANNEL", &*this.channel);
                Poll::Pending
            }
        }
    }
}
