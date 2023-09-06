use haproxy_api::{Action, Core, ServiceMode, Txn};
use mlua::prelude::*;

#[mlua::lua_module(skip_memory_check)]
fn haproxy_simple_module(lua: &Lua) -> LuaResult<bool> {
    let core = Core::new(lua)?;

    // Reverse input string
    core.register_converters("rust_conv", |_lua, input: String| {
        Ok(input.chars().rev().collect::<String>())
    })?;

    // Fetch first value of header `name`
    core.register_fetches("rust_fetch", |_lua, (txn, name): (Txn, String)| {
        let val = txn
            .http()?
            .req_get_headers()?
            .get_first::<LuaValue>(&name)?;
        Ok(val)
    })?;

    // Dumps all request headers to console
    core.register_action("rust_act", &[Action::HttpReq], 0, |_lua, txn: Txn| {
        for kv in txn.http()?.req_get_headers()?.pairs() {
            let (k, v): (String, Vec<String>) = kv?;
            println!("{}: {:?}", k, v);
        }
        Ok(())
    })?;

    let code = mlua::chunk! {
        local applet = ...
        local response = "Hello, World!"
        applet:set_status(200)
        applet:add_header("content-length", string.len(response))
        applet:add_header("content-type", "text/plain")
        applet:start_response()
        applet:send(response)
    };
    core.register_lua_service("rust_service", ServiceMode::Http, code)?;

    Ok(true)
}
