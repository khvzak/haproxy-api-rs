use std::collections::HashMap;

use mlua::{
    AnyUserData, AsChunk, ExternalError, FromLuaMulti, IntoLua, Lua, Result, Table, TableExt, Value,
};

#[cfg(feature = "async")]
use {
    futures_util::future::Either,
    mlua::{ExternalResult, Function, IntoLuaMulti, RegistryKey},
    rustc_hash::{FxBuildHasher, FxHashMap},
    std::future::{self, Future},
    std::ops::Deref,
    std::pin::Pin,
    std::sync::atomic::{AtomicU32, Ordering},
    std::sync::OnceLock,
    std::task::{Context, Poll},
    tokio::io::AsyncWriteExt,
    tokio::net::TcpListener,
    tokio::runtime,
    tokio::sync::{mpsc, oneshot},
};

use crate::filter::UserFilterWrapper;
use crate::{Proxy, UserFilter};

/// The "Core" class contains all the HAProxy core functions.
#[derive(Clone)]
pub struct Core<'lua> {
    lua: &'lua Lua,
    class: Table<'lua>,
}

#[derive(Debug, Copy, Clone)]
pub struct Time {
    pub sec: u64,
    pub usec: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    TcpReq,
    TcpRes,
    HttpReq,
    HttpRes,
}

#[derive(Debug, Copy, Clone)]
pub enum ServiceMode {
    Tcp,
    Http,
}

#[derive(Debug, Copy, Clone)]
pub enum LogLevel {
    Emerg,
    Alert,
    Crit,
    Err,
    Warning,
    Notice,
    Info,
    Debug,
}

impl<'lua> Core<'lua> {
    /// Creates new Core object using Lua global `core`
    #[inline]
    pub fn new(lua: &'lua Lua) -> Result<Self> {
        let class: Table = lua.globals().get("core")?;
        Ok(Core { lua, class })
    }

    /// Returns a map of declared proxies (frontends and backends), indexed by proxy name.
    #[inline]
    pub fn proxies(&self) -> Result<HashMap<String, Proxy<'lua>>> {
        self.class.get("proxies")
    }

    /// Returns a map of declared proxies with backend capability, indexed by the backend name.
    #[inline]
    pub fn backends(&self) -> Result<HashMap<String, Proxy<'lua>>> {
        self.class.get("backends")
    }

    /// Returns a map of declared proxies with frontend capability, indexed by the frontend name.
    #[inline]
    pub fn frontends(&self) -> Result<HashMap<String, Proxy<'lua>>> {
        self.class.get("frontends")
    }

    /// Returns the executing thread number starting at 0.
    /// If thread is 0, Lua scope is shared by all threads, otherwise the scope is dedicated to a single thread.
    /// This is HAProxy >=2.4 feature.
    #[inline]
    pub fn thread(&self) -> Result<u16> {
        self.class.get("thread")
    }

    /// Sends a log on the default syslog server if it is configured and on the stderr if it is allowed.
    #[inline]
    pub fn log(&self, level: LogLevel, msg: impl AsRef<str>) -> Result<()> {
        let msg = msg.as_ref();
        self.class.call_function("log", (level, msg))
    }

    /// Adds the ACL `key` in the ACLs list referenced by `filename`.
    #[inline]
    pub fn add_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("add_acl", (filename, key))
    }

    /// Deletes the ACL entry by `key` in the ACLs list referenced by `filename`.
    #[inline]
    pub fn del_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_acl", (filename, key))
    }

    /// Deletes the map entry indexed with the specified `key` in the list of maps
    /// referenced by his `filename`.
    #[inline]
    pub fn del_map(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_map", (filename, key))
    }

    /// Sets the `value` associated to the `key` in the map referenced by `filename`.
    #[inline]
    pub fn set_map(&self, filename: &str, key: &str, value: &str) -> Result<()> {
        self.class.call_function("set_map", (filename, key, value))
    }

    /// Returns HAProxy core information (uptime, pid, memory pool usage, tasks number, ...).
    #[inline]
    pub fn get_info(&self) -> Result<Vec<String>> {
        self.class.call_function("get_info", ())
    }

    /// Returns the current time.
    /// The time returned is fixed by the HAProxy core and assures than the hour will be monotonic
    /// and that the system call `gettimeofday` will not be called too.
    #[inline]
    pub fn now(&self) -> Result<Time> {
        let time: Table = self.class.call_function("now", ())?;
        Ok(Time {
            sec: time.get("sec")?,
            usec: time.get("usec")?,
        })
    }

    /// Takes a string representing http date, and returns an integer containing the corresponding date
    ///  with a epoch format.
    /// A valid http date me respect the format IMF, RFC850 or ASCTIME.
    #[inline]
    pub fn http_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("http_date", date)?;
        date.ok_or_else(|| "invalid date".into_lua_err())
    }

    /// Take a string representing IMF date, and returns an integer containing the corresponding date
    /// with a epoch format.
    #[inline]
    pub fn imf_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("imf_date", date)?;
        date.ok_or_else(|| "invalid date".into_lua_err())
    }

    /// Takess a string representing RFC850 date, and returns an integer containing the corresponding date
    /// with a epoch format.
    #[inline]
    pub fn rfc850_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("rfc850_date", date)?;
        date.ok_or_else(|| "invalid date".into_lua_err())
    }

    /// Takes a string representing ASCTIME date, and returns an integer containing the corresponding date
    /// with a epoch format.
    #[inline]
    pub fn asctime_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("asctime_date", date)?;
        date.ok_or_else(|| "invalid date".into_lua_err())
    }

    /// Registers a function executed as an action.
    /// The expected actions are `tcp-req`, `tcp-res`, `http-req` or `http-res`.
    /// All the registered actions can be used in HAProxy with the prefix `lua.`.
    pub fn register_action<A, F>(
        &self,
        name: &str,
        actions: &[Action],
        nb_args: usize,
        func: F,
    ) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        F: Fn(&'lua Lua, A) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        let actions = actions
            .iter()
            .map(|act| match act {
                Action::TcpReq => "tcp-req",
                Action::TcpRes => "tcp-res",
                Action::HttpReq => "http-req",
                Action::HttpRes => "http-res",
            })
            .collect::<Vec<_>>();
        self.class
            .call_function("register_action", (name, actions, func, nb_args))
    }

    /// Same as [`register_action`] but using Lua function.
    ///
    /// [`register_action`]: #method.register_action
    pub fn register_lua_action<'a, S>(
        &self,
        name: &str,
        actions: &[&str],
        nb_args: usize,
        code: S,
    ) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    /// Registers a function executed as a converter.
    /// All the registered converters can be used in HAProxy with the prefix `lua.`.
    pub fn register_converters<A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: IntoLua<'lua>,
        F: Fn(&'lua Lua, A) -> Result<R> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_converters", (name, func))
    }

    /// Same as [`register_converters`] but using Lua function.
    ///
    /// [`register_converters`]: #method.register_converters
    pub fn register_lua_converters<'a, S>(&self, name: &str, code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_converters", (name, func))
    }

    /// Registers a function executed as sample fetch.
    /// All the registered sample fetch can be used in HAProxy with the prefix `lua.`.
    pub fn register_fetches<A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: IntoLua<'lua>,
        F: Fn(&'lua Lua, A) -> Result<R> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    /// Same as [`register_fetches`] but using Lua function.
    ///
    /// [`register_fetches`]: #method.register_fetches
    pub fn register_lua_fetches<'a, S>(&self, name: &str, code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_fetches", (name, func))
    }

    /// Registers a custom filter that implements [`UserFilter`] trait.
    pub fn register_filter<T: UserFilter + 'static>(&self, name: &str) -> Result<()> {
        let lua = self.lua;
        let func = lua.create_function(|_, (class, args): (Table, Table)| {
            class.raw_set("args", args)?;
            Ok(class)
        });
        let filter_class = UserFilterWrapper::<T>::make_class(lua)?;
        self.class
            .call_function("register_filter", (name, filter_class, func))
    }

    /// Registers a Lua function executed as a service.
    /// All the registered service can be used in HAProxy with the prefix `lua.`.
    pub fn register_lua_service<'a, S>(&self, name: &str, mode: ServiceMode, code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.class
            .call_function("register_service", (name, mode, func))
    }

    /// Registers a function executed after the configuration parsing.
    /// This is useful to check any parameters.
    pub fn register_init<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'lua Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_init", func)
    }

    /// Registers and start an independent task.
    /// The task is started when the HAProxy main scheduler starts.
    pub fn register_task<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'lua Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    /// Registers and start an independent asynchronous task.
    #[cfg(feature = "async")]
    pub fn register_async_task<F, FR>(&self, func: F) -> Result<()>
    where
        F: Fn() -> FR + 'static,
        FR: Future<Output = Result<()>> + Send + 'static,
    {
        let func = create_async_function(self.lua, move |()| func())?;
        self.class.call_function("register_task", func)
    }

    /// Same as [`register_task`] but using Lua function.
    ///
    /// [`register_task`]: #method.register_task
    pub fn register_lua_task<'a, S>(&self, code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_task", func)
    }

    /// Registers a Lua function executed as a cli command.
    pub fn register_lua_cli<'a, S>(&self, path: &[&str], usage: &str, code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_cli", (path, usage, func))
    }

    /// Changes the nice of the current task or current session.
    #[inline]
    pub fn set_nice(&self, nice: i32) -> Result<()> {
        self.class.call_function("set_nice", nice)
    }

    /// Parses ipv4 or ipv6 addresses and its facultative associated network.
    #[inline]
    pub fn parse_addr(&self, addr: &str) -> Result<AnyUserData<'lua>> {
        self.class.call_function("parse_addr", addr)
    }

    /// Matches two networks.
    /// For example "127.0.0.1/32" matches "127.0.0.0/8". The order of network is not important.
    #[inline]
    pub fn match_addr(&self, addr1: AnyUserData, addr2: AnyUserData) -> Result<bool> {
        self.class.call_function("match_addr", (addr1, addr2))
    }

    // SKIP: concat/done/yield/tokenize/etc
}

impl<'lua> IntoLua<'lua> for LogLevel {
    #[inline]
    fn into_lua(self, lua: &'lua Lua) -> Result<Value<'lua>> {
        (match self {
            LogLevel::Emerg => 0,
            LogLevel::Alert => 1,
            LogLevel::Crit => 2,
            LogLevel::Err => 3,
            LogLevel::Warning => 4,
            LogLevel::Notice => 5,
            LogLevel::Info => 6,
            LogLevel::Debug => 7,
        })
        .into_lua(lua)
    }
}

#[cfg(feature = "async")]
type FutureId = u32;

#[cfg(feature = "async")]
#[derive(Clone, Debug)]
struct FutureNotifier(mpsc::Sender<FutureId>);

#[cfg(feature = "async")]
impl Deref for FutureNotifier {
    type Target = mpsc::Sender<FutureId>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Max size of the pool of channels
#[cfg(feature = "async")]
const POOL_MAX_SIZE: usize = 512;

// Pool of channels
#[cfg(feature = "async")]
struct Pool(Vec<RegistryKey>);

#[cfg(feature = "async")]
struct FutureChannelMap(FxHashMap<FutureId, RegistryKey>);

// Future id generator
#[cfg(feature = "async")]
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[cfg(feature = "async")]
fn runtime() -> &'static runtime::Runtime {
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
#[cfg(feature = "async")]
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
#[cfg(feature = "async")]
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

#[cfg(feature = "async")]
struct YieldFixUp<'lua>(&'lua Lua, Function<'lua>);

#[cfg(feature = "async")]
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

#[cfg(feature = "async")]
impl<'lua> Drop for YieldFixUp<'lua> {
    fn drop(&mut self) {
        if let Err(e) = (|| {
            let coroutine: Table = self.0.globals().get("coroutine")?;
            coroutine.set("yield", self.1.clone())
        })() {
            println!("Error in YieldFixUp destructor: {}", e);
        }
    }
}

#[cfg(feature = "async")]
pin_project_lite::pin_project! {
    struct HaproxyFuture<'lua, F> {
        lua: &'lua Lua,
        channel: Table<'lua>,
        id: FutureId,
        #[pin]
        fut: F,
    }
}

#[cfg(feature = "async")]
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
