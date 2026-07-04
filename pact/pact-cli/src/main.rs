//! pact-cli — the bitcoin-cli of Pact: a thin JSON-RPC client for pactd.
//!
//! No swap logic and no engine here — every command is a JSON-RPC call to
//! a running pactd. Auth mirrors bitcoin-cli: read `.cookie` from the data
//! dir, or pass `--rpcuser`/`--rpcpassword`.
//!
//! Structured subcommands (offer/accept/recv/fund/redeem/refund/abort/
//! status/board) wrap an RPC plus the file I/O of the manual handshake;
//! `pact-cli call <method> [params...]` is the generic passthrough.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "pact-cli",
    version,
    about = "JSON-RPC client for pactd (PoCX trading)"
)]
struct Cli {
    /// pactd JSON-RPC URL.
    #[arg(long, default_value = "http://127.0.0.1:9737")]
    rpc: String,
    /// Data dir to read `.cookie` (and pact.conf) for auth.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Override auth (instead of the cookie).
    #[arg(long)]
    rpcuser: Option<String>,
    #[arg(long)]
    rpcpassword: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generic passthrough: pact-cli call <method> [params...]
    Call {
        method: String,
        /// Each param parsed as JSON if possible, else treated as a string.
        params: Vec<String>,
    },
    /// Start a swap as initiator; writes the `init` message to --out.
    Offer {
        #[arg(long)]
        give: String,
        #[arg(long)]
        get: String,
        #[arg(long)]
        t1: u32,
        #[arg(long)]
        t2: u32,
        #[arg(long)]
        out: PathBuf,
    },
    /// Accept an `init` message (--in); writes `accept` to --out.
    Accept {
        #[arg(long)]
        r#in: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Ingest a counterparty message (accept/funded/redeemed/abort).
    Recv {
        #[arg(long)]
        r#in: PathBuf,
    },
    /// Fund our HTLC leg; writes the `funded` message to --out.
    Fund {
        #[arg(long)]
        swap: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Redeem the counterparty HTLC (initiator: reveals the preimage).
    Redeem {
        #[arg(long)]
        swap: String,
    },
    /// Broadcast the refund for our HTLC (valid once MTP >= T).
    Refund {
        #[arg(long)]
        swap: String,
    },
    /// Abort a swap before any funding.
    Abort {
        #[arg(long)]
        swap: String,
        #[arg(long, default_value = "user aborted")]
        reason: String,
    },
    /// Show swap state(s).
    Status {
        #[arg(long)]
        swap: Option<String>,
    },
    /// Recover in-flight swaps from our encrypted relay snapshots (#54) —
    /// for a fresh install or wiped data dir restored from the same seed.
    /// Idempotent: swaps already present locally are left untouched.
    /// ONLY run this once the machine that ran the swaps is retired — two
    /// live machines driving one swap can double-fund it.
    Restore,
    /// Check (read-only) how many in-flight swaps `restore` would recover
    /// from the relay snapshots, without adopting any (#54).
    RescueStatus,
    /// Seed lifecycle: show whether a seed exists / is encrypted / locked.
    Walletstatus,
    /// Wallet activity of a nodeless coin, newest first (epic #58).
    Transactions {
        /// Coin id (e.g. `btcx`) — must be a nodeless (Electrum-backed) coin.
        coin: String,
    },
    /// List shipped coins: which are configured + live connection status.
    Coins,
    /// List derived swap-pair availability for the current setup.
    Pairs,
    /// Genesis-validate a backend for a coin before configuring it.
    Validatecoin {
        #[arg(long)]
        coin: String,
        /// Comma-separated backend URL(s); first is the wallet-qualified RPC.
        #[arg(long)]
        backend: String,
    },
    /// Create a new seed; prints the mnemonic ONCE — write it down.
    /// With --passphrase the seed is encrypted at rest (PACTSEEDv1).
    Createseed {
        #[arg(long)]
        passphrase: Option<String>,
        /// Seed length: 12 words (default — hot transit wallet) or 24.
        #[arg(long, default_value_t = 12)]
        words: u16,
    },
    /// Import an existing BIP39 mnemonic (optionally encrypted at rest).
    Importseed {
        #[arg(long)]
        mnemonic: String,
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Unlock an encrypted seed for this session (held in memory only).
    Unlock {
        #[arg(long)]
        passphrase: String,
    },
    /// Corkboard interactions.
    Board {
        #[command(subcommand)]
        action: BoardCommand,
    },
}

#[derive(Subcommand, Debug)]
enum BoardCommand {
    Post {
        #[arg(long)]
        give: String,
        #[arg(long)]
        get: String,
        #[arg(long, default_value_t = 12 * 3600)]
        t1_secs: u32,
        #[arg(long, default_value_t = 6 * 3600)]
        t2_secs: u32,
    },
    Offers,
    Take {
        #[arg(long)]
        offer: String,
    },
    Revoke {
        #[arg(long)]
        offer: String,
    },
    /// One coordination + scheduler pass (the `tick` RPC).
    Sync,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let auth = resolve_auth(&cli)?;
    let client = RpcClient {
        url: cli.rpc.clone(),
        auth,
    };

    match cli.command {
        Command::Call { method, params } => {
            let params: Vec<Value> = params
                .iter()
                .map(|p| serde_json::from_str(p).unwrap_or_else(|_| Value::String(p.clone())))
                .collect();
            let result = client.call(&method, Value::Array(params))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Offer {
            give,
            get,
            t1,
            t2,
            out,
        } => {
            let r = client.call("offer", json!([give, get, t1, t2]))?;
            write_envelope(&out, &r["envelope"])?;
            println!(
                "swap {} offered; init written to {}",
                r["record"]["swap_id"],
                out.display()
            );
        }
        Command::Accept { r#in, out } => {
            let envelope = read_json(&r#in)?;
            let r = client.call("acceptoffer", json!([envelope]))?;
            write_envelope(&out, &r["envelope"])?;
            println!(
                "swap {} accepted; accept written to {}",
                r["record"]["swap_id"],
                out.display()
            );
        }
        Command::Recv { r#in } => {
            let envelope = read_json(&r#in)?;
            let r = client.call("recv", json!([envelope]))?;
            println!(
                "swap {}: state {}",
                r["record"]["swap_id"], r["record"]["state"]
            );
        }
        Command::Fund { swap, out } => {
            let r = client.call("fund", json!([swap]))?;
            write_envelope(&out, &r["envelope"])?;
            println!(
                "swap {}: funded; message written to {}",
                r["record"]["swap_id"],
                out.display()
            );
        }
        Command::Redeem { swap } => {
            let r = client.call("redeem", json!([swap]))?;
            println!("swap {}: {}", r["record"]["swap_id"], r["record"]["state"]);
        }
        Command::Refund { swap } => {
            let r = client.call("refund", json!([swap]))?;
            println!("swap {}: {}", r["record"]["swap_id"], r["record"]["state"]);
        }
        Command::Abort { swap, reason } => {
            let r = client.call("abort", json!([swap, reason]))?;
            println!("swap {}: {}", r["record"]["swap_id"], r["record"]["state"]);
        }
        Command::Status { swap } => {
            let result = match swap {
                Some(id) => client.call("getswap", json!([id]))?,
                None => client.call("listswaps", json!([]))?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Restore => {
            let r = client.call("restorefromrelay", json!([]))?;
            let (restored, seen) = (
                r["restored"].as_u64().unwrap_or(0),
                r["seen"].as_u64().unwrap_or(0),
            );
            println!("rescued {restored} swap(s) from {seen} relay snapshot(s)");
        }
        Command::RescueStatus => {
            let r = client.call("rescuestatus", json!([]))?;
            let (pending, seen) = (
                r["pending"].as_u64().unwrap_or(0),
                r["seen"].as_u64().unwrap_or(0),
            );
            println!("{pending} recoverable swap(s) in {seen} relay snapshot(s)");
            if let Some(w) = r["warning"].as_str() {
                println!("WARNING: {w}");
            }
        }
        Command::Walletstatus => {
            let result = client.call("walletstatus", json!([]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Transactions { coin } => {
            let result = client.call("listtransactions", json!([coin]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Coins => {
            let result = client.call("listcoins", json!([]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Pairs => {
            let result = client.call("listpairs", json!([]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Validatecoin { coin, backend } => {
            let result = client.call("validatecoin", json!([coin, backend]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Createseed { passphrase, words } => {
            let r = client.call("createseed", json!([passphrase, words]))?;
            println!(
                "seed created ({}). WRITE THIS DOWN — it is shown only once:\n\n  {}\n",
                if r["encrypted"].as_bool().unwrap_or(false) {
                    "encrypted"
                } else {
                    "unencrypted"
                },
                r["mnemonic"].as_str().unwrap_or("?")
            );
        }
        Command::Importseed {
            mnemonic,
            passphrase,
        } => {
            let r = client.call("importseed", json!([mnemonic, passphrase]))?;
            println!("seed imported; identity {}", r["identity"]);
        }
        Command::Unlock { passphrase } => {
            let r = client.call("unlock", json!([passphrase]))?;
            println!("unlocked; identity {}", r["identity"]);
        }
        Command::Board { action } => {
            let result = match action {
                BoardCommand::Post {
                    give,
                    get,
                    t1_secs,
                    t2_secs,
                } => client.call("boardpostoffer", json!([give, get, t1_secs, t2_secs]))?,
                BoardCommand::Offers => client.call("boardlistoffers", json!([]))?,
                BoardCommand::Take { offer } => client.call("boardtake", json!([offer]))?,
                BoardCommand::Revoke { offer } => client.call("boardrevoke", json!([offer]))?,
                BoardCommand::Sync => client.call("tick", json!([]))?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}

/// `user:pass` for HTTP Basic — explicit creds, else the cookie file.
fn resolve_auth(cli: &Cli) -> Result<String> {
    if let (Some(u), Some(p)) = (&cli.rpcuser, &cli.rpcpassword) {
        return Ok(format!("{u}:{p}"));
    }
    let dir = cli
        .data_dir
        .as_deref()
        .context("no auth: pass --rpcuser/--rpcpassword or --data-dir (to read .cookie)")?;
    let cookie = std::fs::read_to_string(dir.join(".cookie"))
        .with_context(|| format!("reading {}", dir.join(".cookie").display()))?;
    Ok(cookie.trim().to_string())
}

fn read_json(path: &Path) -> Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn write_envelope(path: &Path, envelope: &Value) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(envelope)? + "\n")
        .with_context(|| format!("writing {}", path.display()))
}

struct RpcClient {
    url: String,
    auth: String,
}

impl RpcClient {
    fn call(&self, method: &str, params: Value) -> Result<Value> {
        let rest = self
            .url
            .strip_prefix("http://")
            .context("--rpc must be http://")?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = hostport
            .rsplit_once(':')
            .context("--rpc needs an explicit port")?;
        let body =
            json!({ "jsonrpc": "2.0", "id": "pact-cli", "method": method, "params": params })
                .to_string();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nAuthorization: Basic {}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            base64(self.auth.as_bytes()),
            body.len()
        );
        let mut stream = TcpStream::connect((host, port.parse::<u16>()?))
            .with_context(|| format!("connecting to pactd at {host}:{port}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.write_all(request.as_bytes())?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw)?;
        let text = String::from_utf8_lossy(&raw);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .context("malformed HTTP response")?;
        let status = head.lines().next().unwrap_or("");
        if status.contains("401") {
            bail!("authentication failed (check the cookie / credentials)");
        }
        let body = if head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            dechunk(body)?
        } else {
            body.to_string()
        };
        let parsed: Value = serde_json::from_str(body.trim())
            .with_context(|| format!("non-JSON response: {status}"))?;
        if let Some(err) = parsed.get("error").filter(|e| !e.is_null()) {
            bail!("{}", err["message"].as_str().unwrap_or("RPC error"));
        }
        Ok(parsed["result"].clone())
    }
}

fn dechunk(body: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let (size_line, after) = rest.split_once("\r\n").context("bad chunked encoding")?;
        let size = usize::from_str_radix(size_line.trim(), 16).context("bad chunk size")?;
        if size == 0 {
            return Ok(out);
        }
        let chunk = after.get(..size).context("truncated chunk")?;
        out.push_str(chunk);
        rest = after
            .get(size..)
            .and_then(|r| r.strip_prefix("\r\n"))
            .context("bad chunk terminator")?;
    }
}

/// RFC 4648 base64 for the Basic auth header.
fn base64(input: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(A[(n >> 18 & 63) as usize] as char);
        out.push(A[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            A[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
