// Manual diagnostic (kept on purpose): exercises the REAL backend ssl://
// path (our rustls TLS setup + the genesis-checking verify_chain) against
// the shipped mainnet BTC default Electrum servers.
use libswap::chain::ChainBackend;

fn main() {
    let params = libswap::registry::lookup("btc", libswap::params::Network::Mainnet)
        .expect("btc mainnet params");
    for url in [
        "ssl://electrum.blockstream.info:50002",
        "ssl://electrum.emzy.de:50002",
    ] {
        match libswap::chain::ElectrumBackend::new(params, url) {
            Ok(b) => match b.verify_chain() {
                Ok(()) => println!("OK  {url} — TLS + server.version + genesis all verified"),
                Err(e) => println!("VERIFY-ERR {url}: {e:#}"),
            },
            Err(e) => println!("CONNECT-ERR {url}: {e:#}"),
        }
    }
}
