//! Minimal Bitcoin-Core-style JSON-RPC client over std-only HTTP/1.1.
//!
//! Localhost RPC needs no TLS, and hand-rolling ~100 lines avoids pinning
//! an RPC crate to a specific `bitcoin` crate version. Wallet-qualified
//! URLs (`http://user:pass@host:port/wallet/<name>`) address one wallet on
//! a multi-wallet node.
//!
//! Auth (#162, bitcoind semantics): credentials in the URL are used verbatim
//! (`user:pass`, including a literal `__cookie__:hex`). A
//! `__cookiefile__:<percent-encoded-abs-path>@` userinfo instead names the
//! node's `.cookie` FILE — read live at call time, cached, and re-read once
//! on an HTTP 401 (the node restarted and minted a new cookie), exactly like
//! `bitcoin-cli -rpccookiefile`. A URL with no userinfo at all can fall back
//! to caller-supplied default cookie locations
//! ([`RpcClient::from_url_or_cookie`]) — the `bitcoin-cli` no-flags default.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;

/// URL userinfo sentinel for cookie-FILE auth:
/// `http://__cookiefile__:<percent-encoded-abs-path>@host:port[/wallet/x]`.
/// Wires the PATH, not the secret — the client re-reads the file so a node
/// restart (fresh cookie) self-heals without recomposing the URL (#162).
pub const COOKIEFILE_SENTINEL: &str = "__cookiefile__";

#[derive(Debug)]
pub struct RpcClient {
    host: String,
    port: u16,
    path: String,
    auth: Auth,
}

/// How each request's Basic-auth payload is resolved.
#[derive(Debug)]
enum Auth {
    /// Fixed credentials straight from the URL (`user:pass`, including the
    /// legacy direct `__cookie__:hex` form), pre-encoded for the header.
    Fixed(String),
    /// Cookie-file auth: read the first existing candidate file at call
    /// time (the file content IS the `user:pass`), cache the encoded value,
    /// invalidate + re-read once on a 401 (#162). `candidates` is a single
    /// explicit path for the `__cookiefile__:` sentinel, or the platform
    /// default locations for the no-credentials fallback.
    CookieFile {
        candidates: Vec<PathBuf>,
        cached: RwLock<Option<String>>,
    },
}

#[derive(Debug, thiserror::Error)]
#[error("RPC error {code}: {message}")]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

/// HTTP 401 from the node. Typed so [`RpcClient::call`] can recognize it and
/// run the one-shot cookie re-read; everything else treats it as an error.
#[derive(Debug, thiserror::Error)]
#[error("RPC authentication failed ({status})")]
pub struct RpcUnauthorized {
    pub status: String,
}

impl RpcClient {
    /// Parse `http://user:pass@host:port[/wallet/name]`. The userinfo may be
    /// literal credentials or the [`COOKIEFILE_SENTINEL`] form; a URL with no
    /// userinfo is refused (use [`Self::from_url_or_cookie`] to allow the
    /// cookie auto-discovery fallback).
    pub fn from_url(url: &str) -> Result<Self> {
        Self::parse(url, Vec::new())
    }

    /// Like [`Self::from_url`], but a URL with NO credentials falls back to
    /// cookie-file auth over `default_cookie_candidates` (first existing file
    /// wins, probed per call) — bitcoind's own default when neither
    /// `-rpcuser` nor `-rpccookiefile` is given.
    pub fn from_url_or_cookie(url: &str, default_cookie_candidates: Vec<PathBuf>) -> Result<Self> {
        Self::parse(url, default_cookie_candidates)
    }

    fn parse(url: &str, fallback: Vec<PathBuf>) -> Result<Self> {
        let rest = url
            .strip_prefix("http://")
            .context("RPC URL must start with http:// (localhost RPC; TLS unsupported)")?;
        // Split at the LAST '@' so an (encoded) userinfo can never leak into
        // the host part; the cookie-file path itself percent-encodes '@'.
        let (userinfo, rest) = match rest.rsplit_once('@') {
            Some((userinfo, rest)) => (Some(userinfo), rest),
            None => (None, rest),
        };
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = hostport
            .rsplit_once(':')
            .context("RPC URL must contain an explicit port")?;
        let auth = match userinfo {
            Some(userinfo) => {
                // The sentinel splits at its FIRST ':' — a literal Windows
                // drive-letter colon in the path is unambiguous after that.
                if let Some(encoded) = userinfo.strip_prefix("__cookiefile__:") {
                    let cookie_path = percent_decode(encoded)?;
                    if cookie_path.is_empty() {
                        bail!("__cookiefile__ URL carries an empty cookie path");
                    }
                    Auth::CookieFile {
                        candidates: vec![PathBuf::from(cookie_path)],
                        cached: RwLock::new(None),
                    }
                } else {
                    Auth::Fixed(base64(userinfo.as_bytes()))
                }
            }
            None if !fallback.is_empty() => Auth::CookieFile {
                candidates: fallback,
                cached: RwLock::new(None),
            },
            None => bail!(
                "RPC URL must contain user:pass@ credentials (or use cookie auth so the \
                 node's .cookie can be discovered)"
            ),
        };
        Ok(Self {
            host: host.to_string(),
            port: port.parse().context("invalid RPC port")?,
            path: path.to_string(),
            auth,
        })
    }

    /// Extra attempts to establish the TCP connection before giving up. A
    /// deadline-critical refund/redeem broadcast must not die because the node
    /// was momentarily refusing connections (restarting, briefly overloaded) —
    /// M5. Only the **connect** phase is retried: once the request has been
    /// written we never resend it, because methods like `sendtoaddress` (HTLC
    /// funding) are NOT idempotent — a lost response after the node already
    /// acted must not trigger a second send. A connect failure means the node
    /// never saw the request, so retrying it is always safe.
    const CONNECT_RETRIES: u32 = 3;
    const RETRY_BACKOFF: Duration = Duration::from_millis(250);

    pub fn call(&self, method: &str, params: &[Value]) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0", "id": "libswap", "method": method, "params": params,
        })
        .to_string();

        match self.call_once(method, &body, false) {
            // #162 self-heal: a 401 means the node rejected the request at the
            // auth layer, BEFORE dispatching the method — no side effects — so
            // one full retry is safe even for non-idempotent methods. Under
            // cookie-file auth the usual cause is a node restart (fresh
            // .cookie): re-read the file once and retry; a second 401 is a
            // genuinely wrong/rotated-away credential and surfaces.
            Err(e) if e.is::<RpcUnauthorized>() && matches!(self.auth, Auth::CookieFile { .. }) => {
                self.call_once(method, &body, true)
            }
            other => other,
        }
    }

    /// One connect + exchange with auth resolved fresh (`reread_cookie` forces
    /// the cookie file to be read again — the 401 path).
    fn call_once(&self, method: &str, body: &str, reread_cookie: bool) -> Result<Value> {
        let auth_b64 = self.auth_b64(reread_cookie)?;
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect((self.host.as_str(), self.port)) {
                Ok(s) => break s,
                Err(e) => {
                    attempt += 1;
                    if attempt > Self::CONNECT_RETRIES {
                        return Err(anyhow::Error::new(e).context(format!(
                            "connecting to RPC at {}:{} (after {} attempts)",
                            self.host,
                            self.port,
                            Self::CONNECT_RETRIES + 1
                        )));
                    }
                    std::thread::sleep(Self::RETRY_BACKOFF);
                }
            }
        };
        self.exchange(stream, method, body, &auth_b64)
    }

    /// Resolve this request's Basic-auth payload. Fixed credentials come from
    /// the URL; cookie-file auth reads the first existing candidate file
    /// (bitcoind writes `.cookie` at startup, content = `__cookie__:hex`),
    /// caches the encoded value, and re-reads when `reread` is set. A missing
    /// file is a clear "node not running / cookie absent" class error.
    fn auth_b64(&self, reread: bool) -> Result<String> {
        match &self.auth {
            Auth::Fixed(b64) => Ok(b64.clone()),
            Auth::CookieFile { candidates, cached } => {
                if !reread {
                    if let Ok(guard) = cached.read() {
                        if let Some(b64) = guard.as_ref() {
                            return Ok(b64.clone());
                        }
                    }
                }
                let cookie = read_cookie(candidates)?;
                let b64 = base64(cookie.as_bytes());
                if let Ok(mut guard) = cached.write() {
                    *guard = Some(b64.clone());
                }
                Ok(b64)
            }
        }
    }

    /// Send the request on an established `stream` and parse the reply. NOT
    /// retried (see [`Self::call`]): the request has already been written, so
    /// a non-idempotent method may have taken effect on the node.
    fn exchange(
        &self,
        mut stream: TcpStream,
        method: &str,
        body: &str,
        auth_b64: &str,
    ) -> Result<Value> {
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nAuthorization: Basic {}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.path,
            self.host,
            self.port,
            auth_b64,
            body.len(),
            body
        );

        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(120)))?;
        stream.write_all(request.as_bytes())?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;

        let response = String::from_utf8_lossy(&response);
        let (head, http_body) = response
            .split_once("\r\n\r\n")
            .context("malformed HTTP response from RPC")?;
        let status = head.lines().next().unwrap_or("");
        if status.contains("401") {
            return Err(RpcUnauthorized {
                status: status.to_string(),
            }
            .into());
        }

        let parsed: Value = serde_json::from_str(http_body.trim())
            .with_context(|| format!("non-JSON RPC response to {method}: {status}"))?;
        if let Some(err) = parsed.get("error").filter(|e| !e.is_null()) {
            return Err(RpcError {
                code: err["code"].as_i64().unwrap_or(0),
                message: err["message"].as_str().unwrap_or("unknown").to_string(),
            }
            .into());
        }
        Ok(parsed["result"].clone())
    }
}

/// Read the node cookie from the first existing candidate file. The content
/// is the whole `user:pass` (bitcoind writes `__cookie__:hex`).
fn read_cookie(candidates: &[PathBuf]) -> Result<String> {
    for path in candidates {
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let cookie = raw.trim();
        if cookie.is_empty() {
            bail!("node cookie file {} is empty", path.display());
        }
        return Ok(cookie.to_string());
    }
    let looked: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();
    bail!(
        "no node RPC cookie found (looked for {}) — is the node running?",
        looked.join(", ")
    )
}

/// Decode `%XX` escapes in a `__cookiefile__` path (the composer encodes
/// `@`, `,`, spaces, … so a path can never break URL / URL-list parsing).
fn percent_decode(s: &str) -> Result<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let pair = s
                .get(i + 1..i + 3)
                .context("truncated %-escape in cookie-file path")?;
            out.push(u8::from_str_radix(pair, 16).context("bad %-escape in cookie-file path")?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).context("cookie-file path is not valid UTF-8")
}

/// Plain JSON-over-HTTP request (no auth) for REST services like the
/// Corkboard. `url` is `http://host:port/path`; `body = None` sends GET.
pub fn http_json(url: &str, body: Option<&Value>) -> Result<Value> {
    let rest = url
        .strip_prefix("http://")
        .context("URL must start with http:// (local/community services; TLS via reverse proxy)")?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = hostport
        .rsplit_once(':')
        .context("URL must contain an explicit port")?;
    let port: u16 = port.parse().context("invalid port")?;

    let (method, payload) = match body {
        Some(value) => ("POST", value.to_string()),
        None => ("GET", String::new()),
    };
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );

    let mut stream =
        TcpStream::connect((host, port)).with_context(|| format!("connecting to {host}:{port}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    let response = String::from_utf8_lossy(&response);
    let (head, http_body) = response
        .split_once("\r\n\r\n")
        .context("malformed HTTP response")?;
    let status = head.lines().next().unwrap_or("");
    // Axum services use chunked transfer-encoding; reassemble if present.
    let http_body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(http_body)?
    } else {
        http_body.to_string()
    };
    let parsed: Value = serde_json::from_str(http_body.trim())
        .with_context(|| format!("non-JSON response: {status}"))?;
    if !status.contains("200") {
        bail!(
            "{method} {path}: {status}: {}",
            parsed["error"].as_str().unwrap_or("unknown error")
        );
    }
    Ok(parsed)
}

/// Minimal HTTP/1.1 chunked-transfer decoder.
fn dechunk(body: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let (size_line, after) = rest.split_once("\r\n").context("bad chunked encoding")?;
        let size = usize::from_str_radix(size_line.trim(), 16).context("bad chunk size")?;
        if size == 0 {
            return Ok(out);
        }
        let chunk = after
            .get(..size)
            .context("truncated or non-UTF8-aligned chunk")?;
        out.push_str(chunk);
        rest = after
            .get(size..)
            .and_then(|r| r.strip_prefix("\r\n"))
            .context("bad chunk terminator")?;
    }
}

/// Standard base64 (RFC 4648, with padding) — only used for Basic auth.
fn base64(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18 & 63) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_values() {
        assert_eq!(base64(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
    }

    #[test]
    fn url_parsing() {
        let c = RpcClient::from_url("http://u:p@127.0.0.1:19443/wallet/alice_pocx").unwrap();
        assert_eq!(c.host, "127.0.0.1");
        assert_eq!(c.port, 19443);
        assert_eq!(c.path, "/wallet/alice_pocx");
        assert_eq!(c.auth_b64(false).unwrap(), base64(b"u:p"));
        let c = RpcClient::from_url("http://u:p@localhost:8332").unwrap();
        assert_eq!(c.path, "/");
        assert!(RpcClient::from_url("https://u:p@h:1").is_err());
        assert!(RpcClient::from_url("http://h:1/x").is_err());
        // The direct `__cookie__:hex` form (harness/CLI, legacy stored config)
        // stays accepted verbatim as fixed credentials.
        let c = RpcClient::from_url("http://__cookie__:deadbeef@127.0.0.1:8332").unwrap();
        assert_eq!(c.auth_b64(false).unwrap(), base64(b"__cookie__:deadbeef"));
    }

    #[test]
    fn cookiefile_sentinel_resolves_and_rereads_on_demand() {
        let dir = std::env::temp_dir().join(format!("libswap-rpc-cookie-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cookie_path = dir.join("rpc dir").join(".cookie");
        std::fs::create_dir_all(cookie_path.parent().unwrap()).unwrap();
        std::fs::write(&cookie_path, "__cookie__:aa11\n").unwrap();

        // Percent-encode exactly like satchel's composer: '@', ',' and spaces.
        let encoded = cookie_path
            .display()
            .to_string()
            .replace('%', "%25")
            .replace(' ', "%20");
        let url = format!("http://__cookiefile__:{encoded}@127.0.0.1:8332/wallet/x");
        let c = RpcClient::from_url(&url).unwrap();
        assert_eq!(c.path, "/wallet/x");

        // Resolves from the file, and the value is cached.
        assert_eq!(c.auth_b64(false).unwrap(), base64(b"__cookie__:aa11"));
        std::fs::write(&cookie_path, "__cookie__:bb22\n").unwrap();
        assert_eq!(
            c.auth_b64(false).unwrap(),
            base64(b"__cookie__:aa11"),
            "cached value served until a re-read is forced"
        );
        // The 401 path forces a re-read and picks up the rotated cookie (#162).
        assert_eq!(c.auth_b64(true).unwrap(), base64(b"__cookie__:bb22"));
        // ... and the re-read value replaces the cache.
        assert_eq!(c.auth_b64(false).unwrap(), base64(b"__cookie__:bb22"));

        // A missing file at call time is a clear node-not-running class error.
        std::fs::remove_file(&cookie_path).unwrap();
        let err = c.auth_b64(true).unwrap_err().to_string();
        assert!(err.contains("is the node running"), "{err}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_credentials_falls_back_to_cookie_candidates() {
        let dir = std::env::temp_dir().join(format!("libswap-rpc-discover-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("nowhere").join(".cookie");
        let present = dir.join(".cookie");
        std::fs::write(&present, "__cookie__:cc33").unwrap();

        // Bare URL + candidate list (bitcoind auto-discovery): the first
        // EXISTING candidate wins.
        let c = RpcClient::from_url_or_cookie(
            "http://127.0.0.1:8332/wallet/x",
            vec![missing.clone(), present.clone()],
        )
        .unwrap();
        assert_eq!(c.auth_b64(false).unwrap(), base64(b"__cookie__:cc33"));

        // No candidates existing at call time → clear error listing them.
        let c2 = RpcClient::from_url_or_cookie("http://127.0.0.1:8332", vec![missing]).unwrap();
        let err = c2.auth_b64(false).unwrap_err().to_string();
        assert!(err.contains("is the node running"), "{err}");

        // Bare URL with no fallback stays refused (unchanged contract).
        assert!(RpcClient::from_url("http://127.0.0.1:8332/wallet/x").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn percent_decode_roundtrips_awkward_paths() {
        // '@' and ',' would break URL / URL-list parsing if left literal —
        // the composer encodes them; the drive colon and backslashes may stay.
        assert_eq!(
            percent_decode("C:\\Users\\J%20Doe\\App%40Data\\a%2Cb\\.cookie").unwrap(),
            "C:\\Users\\J Doe\\App@Data\\a,b\\.cookie"
        );
        assert!(percent_decode("bad%2").is_err());
        assert!(percent_decode("bad%zz").is_err());
    }
}
