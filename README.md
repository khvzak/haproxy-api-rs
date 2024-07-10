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
