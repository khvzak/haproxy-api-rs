use haproxy_api::{Core, ServiceMode};
use mlua::prelude::*;
use once_cell::sync::Lazy;
use tokio::{fs, runtime};

static TOKIO: Lazy<runtime::Runtime> = Lazy::new(|| {
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("cannot start tokio runtime")
});

#[mlua::lua_module(skip_memory_check)]
fn haproxy_async_module(lua: &Lua) -> LuaResult<bool> {
    let core = Core::new(lua)?;

    // It's important to use `create_async_function` from the haproxy_api
    let get_file = haproxy_api::create_async_function(lua, |lua, path: String| async move {
        let _guard = TOKIO.enter();
        match fs::read(&path).await {
            Ok(content) => Ok((Some(lua.create_string(&content)?), None)),
            Err(err) => Ok((None, Some(lua.create_string(&err.to_string())?))),
        }
    })?;

    let code = mlua::chunk! {
        local applet = ...
        // Strip first '/'
        local response, err = $get_file(string.sub(applet.path, 2))
        if err ~= nil then
            err = err.."\n"
            applet:set_status(404)
            applet:add_header("content-length", string.len(err))
            applet:add_header("content-type", "text/plain")
            applet:start_response()
            applet:send(err)
            return
        end

        applet:set_status(200)
        applet:add_header("content-length", string.len(response))
        applet:add_header("content-type", "application/octet-stream")
        applet:start_response()
        applet:send(response)
    };
    core.register_lua_service("serve_file", ServiceMode::Http, code)?;

    Ok(true)
}
