# haproxy-api
[![Latest Version]][crates.io] [![API Documentation]][docs.rs]

[Latest Version]: https://img.shields.io/crates/v/haproxy-api.svg
[crates.io]: https://crates.io/crates/haproxy-api
[API Documentation]: https://docs.rs/haproxy-api/badge.svg
[docs.rs]: https://docs.rs/haproxy-api

`haproxy-api` exposes [HAProxy] 2.8+ [Lua API] to use from Rust.
In conjunction with [mlua] it allows to run Rust code in HAProxy as a Task/Converter/Fetcher/Service/etc.
You can extend [HAProxy] on a safe and efficient way beyond Lua restrictions.

## Async support

Asynchronous mode is supported using [Tokio] runtime. The HAProxy runtime is fully integrated with [Tokio] runtime using HAProxy queueing system and auxiliary tcp listener for async tasks readiness notifications.

A multi-threaded tokio runtime is automatically started when the first async function is executed.

Please check the [async_serve_file](examples/async_serve_file) example to see how to serve files asynchronously.

Native HAProxy modules must enable the `module` feature on `haproxy-api` and
select the Lua version used by HAProxy, such as `lua54`.

For ordinary Lua-enabled Cargo commands, `scripts/cargo-lua` selects Lua 5.5
when pkg-config finds it and otherwise falls back to Lua 5.4. On macOS it also
checks the standard Homebrew package locations.

```sh
./scripts/cargo-lua test
```

Use Cargo directly for native modules or another explicit feature set.

## Migration to 0.10

This release includes source-breaking changes:

- Lua-enabled builds use `mlua` 0.12.
- Results that may be `nil` now return `Result<Option<T>>`.
- `StickTable::dump` accepts an optional Lua table filter.
- `EventSub::unsub` returns `Result<()>`.

[HAProxy]: http://www.haproxy.org/
[Lua API]: http://www.arpalert.org/src/haproxy-lua-api/2.6/index.html
[mlua]: https://github.com/khvzak/mlua
[Tokio]: https://tokio.rs/

## Usage

Please check our [examples](examples):
* [async serve file](examples/async_serve_file) - How to serve files asynchronously
* [brotli](examples/brotli) - How to add brotli compression to HAProxy using filters API
* [simple](examples/simple) - How to register fetches and converters

## Restrictions

Executing HAProxy functions that require yielding is not supported (eg: `core.sleep`), and these functionality is not exposed.
Although you can run them from Lua or using `register_lua_*` set of functions.

## License

This project is licensed under the [MIT license](LICENSE)
