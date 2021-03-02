use async_std::fs;
use haproxy_api::{Core, ServiceMode};
use mlua::prelude::*;

#[mlua::lua_module]
fn haproxy_async_module(lua: &Lua) -> LuaResult<bool> {
    let core = Core::new(lua)?;

    // It's important to use `create_async_function` from the haproxy_api
    let get_file = haproxy_api::create_async_function(lua, |lua, path: String| async move {
        match fs::read(&path).await {
            Ok(content) => Ok((Some(lua.create_string(&content)?), None)),
            Err(err) => Ok((None, Some(lua.create_string(&err.to_string())?))),
        }
    })?;
    lua.globals().set("get_file", get_file)?;

    let code = r#"
        local applet = ...
        -- Strip first '/'
        local response, err = get_file(string.sub(applet.path, 2))
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
    "#;
    core.register_lua_service("serve_file", ServiceMode::Http, code)?;

    Ok(true)
}
