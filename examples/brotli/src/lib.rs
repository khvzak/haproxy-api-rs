use std::io::Write;

use brotlic::{BrotliEncoderOptions, CompressorWriter, Quality, WindowSize};
use haproxy_api::{Core, FilterMethod, FilterResult, Headers, HttpMessage, Txn, UserFilter};
use mlua::prelude::*;

#[derive(Default)]
struct BrotliFilter {
    enabled: bool,
    writer: Option<CompressorWriter<Vec<u8>>>,
    options: BrotliFilterOptions,
}

#[derive(Debug, Clone)]
struct BrotliFilterOptions {
    quality: u8,
    window: u8,
    offload: bool,
    content_types: Vec<String>,
}

impl LuaUserData for BrotliFilterOptions {}

impl Default for BrotliFilterOptions {
    fn default() -> Self {
        BrotliFilterOptions {
            quality: 5,
            window: 18,
            offload: false,
            content_types: Vec::new(),
        }
    }
}

impl BrotliFilter {
    fn process_request_headers(&mut self, txn: Txn, msg: HttpMessage) -> LuaResult<()> {
        // Check if we can prefer brotli over other encodings
        // We support only GET method
        self.enabled = txn.f.get::<_, String>("method", ())? == "GET"
            && Self::prefer_brotli_encoding(msg.get_headers()?)?;

        if self.enabled && self.options.offload {
            msg.del_header("accept-encoding")?;
        }

        Ok(())
    }

    fn process_response_headers(&mut self, lua: &Lua, txn: Txn, msg: HttpMessage) -> LuaResult<()> {
        // We encode only "200" responses
        if !self.enabled || txn.f.get::<_, u16>("status", ())? != 200 {
            return Ok(());
        }

        let headers = msg.get_headers()?;
        // Do not encode when `content-encoding` already present
        let mut skip_encoding = headers.get_first::<LuaValue>("content-encoding")?.is_some();
        // Do not encode when `cache-control` includes `no-transform`
        skip_encoding |= headers
            .get::<String>("cache-control")?
            .iter()
            .any(|v| v.contains("no-transform"));
        // Check content type
        if !skip_encoding {
            let content_type = headers
                .get_first::<String>("content-type")?
                .unwrap_or_default()
                .to_ascii_lowercase();
            skip_encoding = content_type.is_empty() || content_type.starts_with("multipart");
            if !skip_encoding {
                let mut found = self.options.content_types.is_empty();
                for prefix in &self.options.content_types {
                    if content_type.starts_with(prefix) {
                        found = true;
                        break;
                    }
                }
                skip_encoding = !found;
            }
        }
        if skip_encoding {
            return Ok(());
        }

        // Update ETag
        match headers.get::<String>("etag")? {
            etag if etag.len() > 1 => return Ok(()),
            etag if etag.len() == 1 && etag[0].starts_with('"') => {
                msg.set_header("etag", format!("W/{}", etag[0]))?;
            }
            _ => {}
        }

        let size_hint = headers
            .get_first::<u32>("content-length")
            .unwrap_or(None)
            .unwrap_or(0);

        // Initialize brotli encoder
        let buf = Vec::with_capacity(4096);
        let encoder = BrotliEncoderOptions::new()
            .quality(Quality::new(self.options.quality).unwrap_or(Quality::worst()))
            .window_size(WindowSize::new(self.options.window).unwrap_or(WindowSize::default()))
            .size_hint(size_hint)
            .build()
            .expect("Failed to build brotli encoder");
        self.writer = Some(CompressorWriter::with_encoder(encoder, buf));

        // Update response headers
        msg.del_header("content-length")?;
        msg.set_header("content-encoding", "br")?;
        msg.set_header("transfer-encoding", "chunked")?;
        msg.add_header("vary", "Accept-Encoding")?;

        Self::register_data_filter(lua, txn, msg.channel()?)
    }

    fn prefer_brotli_encoding(headers: Headers) -> LuaResult<bool> {
        let accept_encoding = headers.get::<String>("accept-encoding")?;
        let vals = accept_encoding
            .iter()
            .flat_map(|v| v.split(',').map(str::trim))
            .filter_map(|v| {
                let (enc, qval) = match v.split_once(";q=") {
                    Some((e, q)) => (e, q),
                    None => return Some((v, 1.0f32)),
                };
                let qval = match qval.parse::<f32>() {
                    Ok(f) if f <= 1.0 => f, // q-values over 1 are unacceptable,
                    _ => return None,
                };
                Some((enc, qval))
            });

        let (mut preferred_encoding, mut max_qval) = ("", 0.);
        for (enc, qval) in vals {
            if qval > max_qval {
                (preferred_encoding, max_qval) = (enc, qval);
            } else if qval == max_qval && enc == "br" {
                preferred_encoding = "br";
            }
        }
        Ok(preferred_encoding == "br")
    }

    fn parse_args(args: LuaTable) -> LuaResult<BrotliFilterOptions> {
        // Fetch ready parsed options
        if let Ok(options) = args.raw_get::<_, BrotliFilterOptions>(0) {
            return Ok(options);
        }

        let mut options = BrotliFilterOptions::default();
        for arg in args.clone().raw_sequence_values::<String>() {
            match &*arg? {
                "offload" => options.offload = true,
                arg if arg.starts_with("type:") => {
                    options.content_types = arg[5..]
                        .split(',')
                        .map(|s| s.trim().to_ascii_lowercase())
                        .collect();
                }
                arg if arg.starts_with("quality:") => {
                    let mut quality = arg[8..].trim().parse::<u8>().unwrap_or_default();
                    if quality > 11 {
                        quality = 11;
                    }
                    options.quality = quality;
                }
                arg if arg.starts_with("window:") => {
                    let mut window = arg[7..].trim().parse::<u8>().unwrap_or_default();
                    if window < 10 {
                        window = 10;
                    }
                    if window > 24 {
                        window = 24;
                    }
                    options.window = window;
                }
                _ => {}
            }
        }
        args.raw_set(0, options.clone())?;
        Ok(options)
    }
}

impl UserFilter for BrotliFilter {
    const METHODS: u8 = FilterMethod::HTTP_HEADERS | FilterMethod::HTTP_PAYLOAD;

    fn new(_: &Lua, args: LuaTable) -> LuaResult<Self> {
        Ok(BrotliFilter {
            options: Self::parse_args(args)?,
            ..Default::default()
        })
    }

    fn http_headers(&mut self, lua: &Lua, txn: Txn, msg: HttpMessage) -> LuaResult<FilterResult> {
        if !msg.is_resp()? {
            self.process_request_headers(txn, msg)?;
        } else {
            self.process_response_headers(lua, txn, msg)?;
        }
        Ok(FilterResult::Continue)
    }

    fn http_payload(&mut self, _: &Lua, _: Txn, msg: HttpMessage) -> LuaResult<Option<usize>> {
        if let Some(chunk) = msg.body(None, None)? {
            let chunk = chunk.as_bytes();
            let writer = self.writer.as_mut().expect("Brotli writer must exists");
            if !chunk.is_empty() {
                writer
                    .write_all(chunk)
                    .expect("Failed to write to brotli encoder");
                writer.flush().expect("Failed to flush brotli encoder");
            }
            if !msg.eom()? {
                if !writer.get_ref().is_empty() {
                    msg.set(writer.get_ref(), None, None)?;
                    writer.get_mut().clear();
                } else if !chunk.is_empty() {
                    msg.remove(None, None)?;
                }
            } else {
                let data = self.writer.take().unwrap().into_inner().unwrap();
                msg.set(data, None, None)?;
            }
        }
        Ok(None)
    }
}

#[mlua::lua_module]
fn haproxy_brotli_filter(lua: &Lua) -> LuaResult<bool> {
    let core = Core::new(lua)?;
    core.register_filter::<BrotliFilter>("brotli").unwrap();
    Ok(true)
}
