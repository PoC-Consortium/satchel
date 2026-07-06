// Manual diagnostic (kept on purpose): exercises the REAL nodeless backend
// ssl:// path (our rustls TLS setup + the genesis-checking verify_chain)
// against a live Electrum server, for ANY registered coin.
//
//   cargo run -p libswap --example electrum_probe
//     → the shipped BTC mainnet default servers (built-in coin).
//   cargo run -p libswap --example electrum_probe -- ltc ssl://host:50002 [ssl://host2:50002 …]
//     → a file-added coin (btcx/ltc from ../../satchel/coins.toml) against the
//       given server(s). Proves whether a real LTC/etc. Electrum server
//       negotiates protocol 1.4+ AND reports the coin's own genesis — the two
//       things `verify_chain` gates a nodeless coin setup on.
//   cargo run -p libswap --example electrum_probe -- ltc --all
//   cargo run -p libswap --example electrum_probe -- --all
//     → probe every mainnet `electrum = [...]` default shipped in coins.toml
//       (one coin / all coins) — the vetting gate to re-run before every
//       change to the shipped server fleets (#102). Exits non-zero if any
//       configured server fails, so it can gate scripts.
use std::time::Instant;

use libswap::chain::ChainBackend;
use libswap::params::{ChainParams, Network};

/// `true` iff the server passed.
fn probe(params: &'static ChainParams, url: &str) -> bool {
    let started = Instant::now();
    match libswap::chain::ElectrumBackend::new(params, url) {
        Ok(b) => match b.verify_chain() {
            Ok(()) => {
                let ms = started.elapsed().as_millis();
                println!("OK  {url} — TLS + server.version + genesis verified ({ms} ms)");
                true
            }
            Err(e) => {
                println!("VERIFY-ERR {url}: {e:#}");
                false
            }
        },
        Err(e) => {
            println!("CONNECT-ERR {url}: {e:#}");
            false
        }
    }
}

/// The shipped mainnet `electrum = [...]` defaults per coin, straight from
/// the raw coins.toml (the `connection` table is Satchel's — the engine's
/// `coins_file` parser deliberately ignores it, so read it raw here).
fn shipped_fleets(toml_str: &str) -> Vec<(String, Vec<String>)> {
    let value: toml::Value = toml_str.parse().expect("coins.toml parses");
    let mut fleets = Vec::new();
    let Some(coins) = value.get("coin").and_then(|c| c.as_array()) else {
        return fleets;
    };
    for coin in coins {
        let Some(id) = coin.get("coin_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let urls: Vec<String> = coin
            .get("mainnet")
            .and_then(|n| n.get("connection"))
            .and_then(|c| c.get("electrum"))
            .and_then(|e| e.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|u| u.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if !urls.is_empty() {
            fleets.push((id.to_string(), urls));
        }
    }
    fleets
}

fn probe_coin(coin: &str, urls: &[String]) -> (usize, usize) {
    let params = libswap::registry::lookup(coin, Network::Mainnet)
        .unwrap_or_else(|| panic!("no mainnet params for coin {coin:?} (in coins.toml?)"));
    println!(
        "probing {coin} mainnet — hrp {}, genesis {}… ({} server(s))",
        params.bech32_hrp,
        &params.genesis_hash[..16],
        urls.len()
    );
    let ok = urls.iter().filter(|url| probe(params, url)).count();
    (ok, urls.len())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Load the shipped coins.toml FIRST (built-ins + file coins like ltc):
    // lookup() lazily locks the registry to built-ins-only on first call, so
    // this must run before any lookup or a file coin never resolves.
    let toml_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../satchel/coins.toml");
    let toml_str = std::fs::read_to_string(toml_path).expect("read coins.toml");
    libswap::registry::init_from_str(&toml_str).expect("init registry from coins.toml");

    // --all forms: vet the SHIPPED fleets (one coin or every coin).
    if let Some(idx) = args.iter().position(|a| a == "--all") {
        let only_coin = (idx > 0).then(|| args[0].clone());
        let mut ok = 0usize;
        let mut total = 0usize;
        for (coin, urls) in shipped_fleets(&toml_str) {
            if only_coin.as_deref().is_some_and(|c| c != coin) {
                continue;
            }
            let (o, t) = probe_coin(&coin, &urls);
            ok += o;
            total += t;
        }
        println!("fleet: {ok}/{total} servers verified");
        std::process::exit(if ok == total && total > 0 { 0 } else { 1 });
    }

    let (coin, urls): (String, Vec<String>) = if args.is_empty() {
        // Default: BTC mainnet defaults (built-in coin, no registry init needed).
        (
            "btc".into(),
            vec![
                "ssl://electrum.blockstream.info:50002".into(),
                "ssl://electrum.emzy.de:50002".into(),
            ],
        )
    } else if args.len() >= 2 {
        (args[0].clone(), args[1..].to_vec())
    } else {
        eprintln!("usage: electrum_probe [--all | <coin> --all | <coin> <ssl://host:port> …]");
        std::process::exit(2);
    };
    let (ok, total) = probe_coin(&coin, &urls);
    std::process::exit(if ok == total { 0 } else { 1 });
}
