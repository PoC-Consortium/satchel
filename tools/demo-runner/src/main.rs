//! Self-contained Satchel demo orchestrator.
//!
//! Brings up a local regtest stack with ONLY the sibling bundled binaries — no
//! Python, cargo, or Node at runtime — then launches Satchel and tears the whole
//! thing down when you close the Satchel window:
//!
//!   * regtest PoCX + BTC nodes (`pocx-bitcoind`, `btc-bitcoind`)
//!   * a local Nostr relay (`nostr-rs-relay`) — no corkboard server
//!   * two counterparty bots (`pactd`) posting a two-sided book over the relay
//!   * Satchel as managed "Alice" (its own bundled `pactd` sidecar)
//!
//! Cross-platform: one codebase -> a native binary per OS. It owns every process
//! it spawns, so teardown is just "kill my children" (+ wipe the temp workdir).
//!
//! Expected layout (the runner finds binaries relative to its own exe):
//!   <demo>/bin/      demo-runner, pocx-bitcoind, btc-bitcoind, nostr-rs-relay
//!   <demo>/satchel/  satchel, pactd, pact-cli
//!
//! Local regtest only — not real funds.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

const EXE: &str = std::env::consts::EXE_SUFFIX;

const POCX_RPC_PORT: u16 = 19443;
const BTC_RPC_PORT: u16 = 19543;
const RELAY_PORT: u16 = 19788;
const BOB_PORT: u16 = 19737;
const CAROL_PORT: u16 = 19738;
const RPC_USER: &str = "pactdemo";
const RPC_PASS: &str = "pactdemo";

const POCX_GENESIS: &str = "2a98a52253aeff06093948b00568d380b7634621bc606403127973c9acbbfde0";
const BTC_GENESIS: &str = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206";

// Two-sided book (mirrors the harness playground). Bob BUYS pocx (gives BTC);
// Carol SELLS pocx (gives POCX). Protocol alternates v1 (HTLC) / v2 (Taproot).
const BOB_OFFERS: &[(&str, &str)] = &[
    ("btc:0.0005", "btcx:24"),
    ("btc:0.001", "btcx:47"),
    ("btc:0.001", "btcx:50"),
    ("btc:0.0015", "btcx:72"),
    ("btc:0.002", "btcx:102"),
    ("btc:0.003", "btcx:153"),
];
const CAROL_OFFERS: &[(&str, &str)] = &[
    ("btcx:25", "btc:0.0005"),
    ("btcx:50", "btc:0.00104"),
    ("btcx:50", "btc:0.00098"),
    ("btcx:75", "btc:0.00156"),
    ("btcx:100", "btc:0.00196"),
];
const PROTOCOLS: &[&str] = &["pact-htlc-v1", "pact-htlc-v2"];

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn basic_auth(user: &str, pass: &str) -> String {
    use std::fmt::Write as _;
    // Minimal base64 (no extra dep) for "user:pass".
    let raw = format!("{user}:{pass}");
    let mut out = String::new();
    let b = raw.as_bytes();
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in b.chunks(3) {
        let (b0, b1, b2) = (chunk[0] as u32, *chunk.get(1).unwrap_or(&0) as u32, *chunk.get(2).unwrap_or(&0) as u32);
        let n = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(out, "{}{}", T[(n >> 18) as usize & 63] as char, T[(n >> 12) as usize & 63] as char);
        let _ = write!(out, "{}", if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        let _ = write!(out, "{}", if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    format!("Basic {out}")
}

/// JSON-RPC POST with HTTP Basic auth (bitcoind + pactd both speak this).
fn rpc(url: &str, auth: &str, method: &str, params: Value) -> Result<Value> {
    let resp = ureq::post(url)
        .set("Authorization", auth)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .send_json(json!({ "jsonrpc": "2.0", "id": "demo", "method": method, "params": params }));
    let body: Value = match resp {
        Ok(r) => r.into_json()?,
        // bitcoind returns 500 with a JSON error body; ureq surfaces that as Err.
        Err(ureq::Error::Status(_, r)) => r.into_json()?,
        Err(e) => return Err(e.into()),
    };
    if !body["error"].is_null() {
        bail!("rpc {method}: {}", body["error"]);
    }
    Ok(body["result"].clone())
}

struct Node {
    name: &'static str,
    url: String,
    auth: String,
    child: Child,
}

impl Node {
    fn start(binary: &Path, datadir: &Path, port: u16, name: &'static str, genesis: &str) -> Result<Node> {
        std::fs::create_dir_all(datadir)?;
        let child = Command::new(binary)
            .arg("-regtest")
            .arg(format!("-datadir={}", datadir.display()))
            .arg("-listen=0")
            .arg("-server=1")
            .arg(format!("-rpcport={port}"))
            .arg(format!("-rpcuser={RPC_USER}"))
            .arg(format!("-rpcpassword={RPC_PASS}"))
            .arg("-fallbackfee=0.0001")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn {name} node ({})", binary.display()))?;
        let node = Node {
            name,
            url: format!("http://127.0.0.1:{port}"),
            auth: basic_auth(RPC_USER, RPC_PASS),
            child,
        };
        node.wait_rpc()?;
        let got = node.call("getblockhash", json!([0]))?;
        if got.as_str() != Some(genesis) {
            bail!("{name}: wrong chain (genesis {got}); is this the right binary?");
        }
        Ok(node)
    }

    fn call(&self, method: &str, params: Value) -> Result<Value> {
        rpc(&self.url, &self.auth, method, params)
    }

    fn wallet_call(&self, wallet: &str, method: &str, params: Value) -> Result<Value> {
        rpc(&format!("{}/wallet/{wallet}", self.url), &self.auth, method, params)
    }

    fn wait_rpc(&self) -> Result<()> {
        let deadline = now_secs() + 60;
        while now_secs() < deadline {
            if self.call("getblockcount", json!([])).is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(300));
        }
        bail!("{}: RPC not up after 60s", self.name)
    }

    fn fund_wallet(&self, wallet: &str, blocks: u64) -> Result<()> {
        self.call("createwallet", json!([wallet]))?;
        if blocks > 0 {
            let addr = self.wallet_call(wallet, "getnewaddress", json!([]))?;
            self.call("generatetoaddress", json!([blocks, addr]))?;
        }
        Ok(())
    }

    fn set_mocktime(&self, t: u64) -> Result<()> {
        self.call("setmocktime", json!([t]))?;
        Ok(())
    }

    fn chain_time(&self) -> u64 {
        self.call("getblockchaininfo", json!([]))
            .ok()
            .and_then(|v| v["time"].as_u64())
            .unwrap_or(0)
    }

    fn mine(&self, wallet: &str, blocks: u64) -> Result<()> {
        let addr = self.wallet_call(wallet, "getnewaddress", json!([]))?;
        self.call("generatetoaddress", json!([blocks, addr]))?;
        Ok(())
    }

    fn wallet_url(&self, wallet: &str) -> String {
        format!("http://{RPC_USER}:{RPC_PASS}@127.0.0.1:{}/wallet/{wallet}", self.port())
    }

    fn port(&self) -> u16 {
        self.url.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0)
    }

    fn stop(mut self) {
        let _ = self.call("stop", json!([]));
        thread::sleep(Duration::from_millis(500));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A bot's pactd: spawn, wait for /health, read its cookie, post offers.
struct Bot {
    name: &'static str,
    url: String,
    auth: String,
    child: Child,
}

impl Bot {
    fn start(pactd: &Path, datadir: &Path, port: u16, name: &'static str, pocx_url: &str, btc_url: &str, relay_ws: &str) -> Result<Bot> {
        std::fs::create_dir_all(datadir)?;
        let child = Command::new(pactd)
            .arg("--data-dir").arg(datadir)
            .arg("--network").arg("regtest")
            .arg("--coin").arg(format!("btcx={pocx_url}"))
            .arg("--coin").arg(format!("btc={btc_url}"))
            .arg("--listen").arg(format!("127.0.0.1:{port}"))
            .arg("--tick-secs").arg("2")
            .arg("--auto-init")
            .arg("--auto-fund")
            .arg("--nostr-relay").arg(relay_ws)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn {name} pactd"))?;
        // Wait for /health, then read the cookie.
        let health = format!("http://127.0.0.1:{port}/health");
        let deadline = now_secs() + 60;
        while now_secs() < deadline {
            if ureq::get(&health).timeout(Duration::from_secs(5)).call().is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(300));
        }
        let cookie = std::fs::read_to_string(datadir.join(".cookie")).context("read bot cookie")?;
        Ok(Bot {
            name,
            url: format!("http://127.0.0.1:{port}/"),
            auth: basic_auth_raw(cookie.trim()),
            child,
        })
    }

    fn post_offer(&self, give: &str, get: &str, proto: &str) -> Result<()> {
        rpc(&self.url, &self.auth, "boardpostoffer", json!([give, get, 14400, 7200, proto]))?;
        Ok(())
    }

    fn live_offer_ids(&self) -> Vec<String> {
        rpc(&self.url, &self.auth, "boardlistoffers", json!([]))
            .ok()
            .and_then(|v| v["offers"].as_array().cloned())
            .map(|a| a.iter().filter_map(|o| o["swap_id"].as_str().map(String::from)).collect())
            .unwrap_or_default()
    }

    fn kill(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Cookie is already "user:pass"; pactd's HTTP Basic wants base64 of that.
fn basic_auth_raw(cookie: &str) -> String {
    // Reuse the encoder via a fake split: base64(cookie) with the helper.
    let (u, p) = cookie.split_once(':').unwrap_or((cookie, ""));
    basic_auth(u, p)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n[demo] ERROR: {e:#}");
        eprintln!("[demo] (see the temp work dir for node logs)");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let bin_dir = std::env::current_exe()?.parent().context("exe dir")?.to_path_buf();
    let demo_root = bin_dir.parent().context("demo root")?.to_path_buf();
    let satchel_dir = demo_root.join("satchel");

    let pocx_bin = bin_dir.join(format!("pocx-bitcoind{EXE}"));
    let btc_bin = bin_dir.join(format!("btc-bitcoind{EXE}"));
    let relay_bin = bin_dir.join(format!("nostr-rs-relay{EXE}"));
    let pactd_bin = satchel_dir.join(format!("pactd{EXE}"));
    let satchel_bin = satchel_dir.join(format!("satchel{EXE}"));
    for (label, p) in [("pocx node", &pocx_bin), ("btc node", &btc_bin), ("relay", &relay_bin), ("pactd", &pactd_bin), ("satchel", &satchel_bin)] {
        if !p.exists() {
            bail!("missing bundled binary: {label} at {}", p.display());
        }
    }

    let work = std::env::temp_dir().join(format!("pact-demo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;

    println!("[demo] starting regtest nodes ...");
    let pocx = Node::start(&pocx_bin, &work.join("pocx"), POCX_RPC_PORT, "pocx", POCX_GENESIS)?;
    let btc = Node::start(&btc_bin, &work.join("btc"), BTC_RPC_PORT, "btc", BTC_GENESIS)?;
    let start_t = now_secs();
    pocx.set_mocktime(start_t)?;
    btc.set_mocktime(start_t)?;

    println!("[demo] funding wallets ...");
    pocx.fund_wallet("alice_pocx", 110)?;
    pocx.fund_wallet("bob_pocx", 0)?;
    pocx.fund_wallet("carol_pocx", 110)?;
    btc.fund_wallet("bob_btc", 110)?;
    btc.fund_wallet("alice_btc", 110)?;
    btc.fund_wallet("carol_btc", 0)?;

    println!("[demo] starting local Nostr relay ...");
    let relay = start_relay(&relay_bin, &work.join("relay"))?;
    let relay_ws = format!("ws://127.0.0.1:{RELAY_PORT}");

    println!("[demo] starting counterparty bots ...");
    let bob = Bot::start(&pactd_bin, &work.join("pact-bob"), BOB_PORT, "bob", &pocx.wallet_url("bob_pocx"), &btc.wallet_url("bob_btc"), &relay_ws)?;
    let carol = Bot::start(&pactd_bin, &work.join("pact-carol"), CAROL_PORT, "carol", &pocx.wallet_url("carol_pocx"), &btc.wallet_url("carol_btc"), &relay_ws)?;

    post_book(&bob, BOB_OFFERS);
    post_book(&carol, CAROL_OFFERS);
    println!("[demo] {} + {} offers posted to the relay", bob.live_offer_ids().len(), carol.live_offer_ids().len());

    println!("[demo] preparing Satchel (Alice) ...");
    write_satchel_config(&pactd_bin, &pocx, &btc, &relay_ws)?;

    println!("[demo] launching Satchel -- CLOSE THE WINDOW to end the demo.");
    let mut satchel = Command::new(&satchel_bin)
        .current_dir(&satchel_dir)
        .spawn()
        .context("launch Satchel")?;

    // Background: advance both mock clocks + mine so confirmations + timelocks
    // progress and taken swaps complete; refill any taken offers.
    let stop = Arc::new(AtomicBool::new(false));
    let miner = spawn_miner(stop.clone(), pocx, btc, bob, carol, start_t);

    let _ = satchel.wait();
    println!("[demo] Satchel closed -- tearing down ...");
    stop.store(true, Ordering::SeqCst);
    let (pocx, btc, bob, carol) = miner.join().expect("miner thread");
    bob.kill();
    carol.kill();
    relay_kill(relay);
    pocx.stop();
    btc.stop();
    let _ = std::fs::remove_dir_all(&work);
    println!("[demo] done.");
    Ok(())
}

fn post_book(bot: &Bot, offers: &[(&str, &str)]) {
    let live = bot.live_offer_ids().len();
    for (i, (give, get)) in offers.iter().enumerate().skip(live) {
        let proto = PROTOCOLS[i % PROTOCOLS.len()];
        if let Err(e) = bot.post_offer(give, get, proto) {
            eprintln!("[demo] {} post failed ({give}->{get}): {e:#}", bot.name);
        }
    }
}

#[allow(clippy::type_complexity)]
fn spawn_miner(
    stop: Arc<AtomicBool>,
    pocx: Node,
    btc: Node,
    bob: Bot,
    carol: Bot,
    start_t: u64,
) -> thread::JoinHandle<(Node, Node, Bot, Bot)> {
    thread::spawn(move || {
        let wall0 = now_secs();
        let mut last_post = now_secs();
        while !stop.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(4));
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let tip = pocx.chain_time().max(btc.chain_time());
            let t = tip.max(start_t + (now_secs() - wall0)) + 1;
            let _ = pocx.set_mocktime(t);
            let _ = btc.set_mocktime(t);
            let _ = pocx.mine("alice_pocx", 1);
            let _ = btc.mine("bob_btc", 1);
            if now_secs() - last_post > 30 {
                post_book(&bob, BOB_OFFERS);
                post_book(&carol, CAROL_OFFERS);
                last_post = now_secs();
            }
        }
        (pocx, btc, bob, carol)
    })
}

fn start_relay(bin: &Path, dir: &Path) -> Result<Child> {
    std::fs::create_dir_all(dir)?;
    let cfg = dir.join("config.toml");
    let db = dir.display().to_string().replace('\\', "/");
    let mut f = std::fs::File::create(&cfg)?;
    write!(
        f,
        "[info]\nrelay_url = \"ws://127.0.0.1:{RELAY_PORT}/\"\nname = \"pact-demo\"\n\n[network]\naddress = \"127.0.0.1\"\nport = {RELAY_PORT}\n\n[database]\ndata_directory = \"{db}\"\n"
    )?;
    let child = Command::new(bin)
        .arg("--config").arg(&cfg)
        .arg("--db").arg(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn nostr-rs-relay")?;
    // Wait for the port to accept connections.
    let deadline = now_secs() + 30;
    while now_secs() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", RELAY_PORT)).is_ok() {
            return Ok(child);
        }
        thread::sleep(Duration::from_millis(200));
    }
    bail!("nostr relay did not come up");
}

fn relay_kill(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Satchel's per-install config dir for bundle id `org.pocx.satchel`.
fn satchel_config_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").context("APPDATA")?;
        Ok(PathBuf::from(appdata).join("org.pocx.satchel"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").context("HOME")?;
        Ok(PathBuf::from(home).join("Library/Application Support/org.pocx.satchel"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let base = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
        Ok(PathBuf::from(base).join("org.pocx.satchel"))
    }
}

fn write_satchel_config(pactd: &Path, pocx: &Node, btc: &Node, relay_ws: &str) -> Result<()> {
    let dir = satchel_config_dir()?;
    std::fs::create_dir_all(&dir)?;
    // Factory-new the managed pactd state so the demo is reproducible.
    let _ = std::fs::remove_dir_all(dir.join("pactd"));
    let cfg = json!({
        "pactd_path": pactd.display().to_string().replace('\\', "/"),
        "coins": [
            { "coin_id": "btcx", "chain_data": pocx.wallet_url("alice_pocx"), "funding_wallet": "core-rpc" },
            { "coin_id": "btc",  "chain_data": btc.wallet_url("alice_btc"),  "funding_wallet": "core-rpc" }
        ],
        "board_urls": [],
        "nostr_relays": [relay_ws],
        "listen": "127.0.0.1:9737",
        "auto_fund": true,
        "tick_secs": 2,
        "ui": { "theme": "system", "language": "en", "nav_open": true }
    });
    std::fs::write(dir.join("satchel.json"), serde_json::to_vec_pretty(&cfg)?)?;
    Ok(())
}
