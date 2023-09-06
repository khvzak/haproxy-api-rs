# haproxy-api
[![Latest Version]][crates.io] [![API Documentation]][docs.rs]

[Latest Version]: https://img.shields.io/crates/v/haproxy-api.svg
[crates.io]: https://crates.io/crates/haproxy-api
[API Documentation]: https://docs.rs/haproxy-api/badge.svg
[docs.rs]: https://docs.rs/haproxy-api

`haproxy-api` exposes [HAProxy] 2.x [Lua API] to use from Rust.
In conjunction with [mlua] it allows to run Rust code in HAProxy as a Task/Converter/Fetcher/Service/etc.
You can extend [HAProxy] on a safe and efficient way beyond Lua restrictions.

Thanks to [mlua], asynchronous mode is also supported and every time when requested Future is in `Pending` state, `haproxy-api` conviniently executes `core.yield()` under the hood to return to the HAProxy scheduler.

Please check the [`async_serve_file`](examples/async_serve_file) example to see how to serve files asynchronously using Tokio.
Bear in mind that asynchronous mode is not too efficient because there is no way to integrate with HAProxy scheduler (current behavior is more close to busy polling).

[HAProxy]: http://www.haproxy.org/
[Lua API]: http://www.arpalert.org/src/haproxy-lua-api/2.6/index.html
[mlua]: https://github.com/khvzak/mlua

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
