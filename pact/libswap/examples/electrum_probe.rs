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
use libswap::chain::ChainBackend;
use libswap::params::{ChainParams, Network};

fn probe(params: &'static ChainParams, url: &str) {
    match libswap::chain::ElectrumBackend::new(params, url) {
        Ok(b) => match b.verify_chain() {
            Ok(()) => println!("OK  {url} — TLS + server.version + genesis all verified"),
            Err(e) => println!("VERIFY-ERR {url}: {e:#}"),
        },
        Err(e) => println!("CONNECT-ERR {url}: {e:#}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
        eprintln!("usage: electrum_probe [<coin> <ssl://host:port> …]");
        std::process::exit(2);
    };

    // Load the shipped coins.toml FIRST (built-ins + file coins like ltc):
    // lookup() lazily locks the registry to built-ins-only on first call, so
    // this must run before any lookup or a file coin never resolves.
    let toml = concat!(env!("CARGO_MANIFEST_DIR"), "/../../satchel/coins.toml");
    let s = std::fs::read_to_string(toml).expect("read coins.toml");
    libswap::registry::init_from_str(&s).expect("init registry from coins.toml");
    let params = libswap::registry::lookup(&coin, Network::Mainnet)
        .unwrap_or_else(|| panic!("no mainnet params for coin {coin:?} (in coins.toml?)"));
    println!(
        "probing {coin} mainnet — hrp {}, genesis {}…",
        params.bech32_hrp,
        &params.genesis_hash[..16]
    );
    for url in &urls {
        probe(params, url);
    }
}
