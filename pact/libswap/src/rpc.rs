//! Minimal Bitcoin-Core-style JSON-RPC client over std-only HTTP/1.1.
//!
//! Localhost RPC needs no TLS, and hand-rolling ~100 lines avoids pinning
//! an RPC crate to a specific `bitcoin` crate version. Wallet-qualified
//! URLs (`http://user:pass@host:port/wallet/<name>`) address one wallet on
//! a multi-wallet node.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RpcClient {
    host: String,
    port: u16,
    path: String,
    auth_b64: String,
}

#[derive(Debug, thiserror::Error)]
#[error("RPC error {code}: {message}")]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcClient {
    /// Parse `http://user:pass@host:port[/wallet/name]`.
    pub fn from_url(url: &str) -> Result<Self> {
        let rest = url
            .strip_prefix("http://")
            .context("RPC URL must start with http:// (localhost RPC; TLS unsupported)")?;
        let (auth, rest) = rest
            .rsplit_once('@')
            .context("RPC URL must contain user:pass@ credentials")?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = hostport
            .rsplit_once(':')
            .context("RPC URL must contain an explicit port")?;
        Ok(Self {
            host: host.to_string(),
            port: port.parse().context("invalid RPC port")?,
            path: path.to_string(),
            auth_b64: base64(auth.as_bytes()),
        })
    }

    pub fn call(&self, method: &str, params: &[Value]) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0", "id": "libswap", "method": method, "params": params,
        })
        .to_string();
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nAuthorization: Basic {}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.path,
            self.host,
            self.port,
            self.auth_b64,
            body.len(),
            body
        );

        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .with_context(|| format!("connecting to RPC at {}:{}", self.host, self.port))?;
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
            bail!("RPC authentication failed ({status})");
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
        let c = RpcClient::from_url("http://u:p@localhost:8332").unwrap();
        assert_eq!(c.path, "/");
        assert!(RpcClient::from_url("https://u:p@h:1").is_err());
        assert!(RpcClient::from_url("http://h:1/x").is_err());
    }
}
