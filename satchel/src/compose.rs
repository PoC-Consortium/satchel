//! Compose the backend URL string pactd consumes (`http://user:pass@host:port
//! /wallet/x[,extra]`) from structured per-coin connection settings.
//!
//! The structured form (host / port / auth / datadir / wallet) is what the UI
//! edits; pactd still receives the same opaque `--coin id=urls` string it always
//! has, so there is no pactd contract change. Cookie auth is resolved here by
//! reading bitcoind's `.cookie` (`__cookie__:hex`) and using it verbatim as the
//! URL's `user:pass` — exactly the phoenix-pocx pattern.

use anyhow::{bail, Context, Result};

use crate::{coins_file, CoinConn};

/// Build the comma-separated backend URL list from a coin's structured fields.
/// Errors with a clear message when cookie/creds are missing — the setup form
/// surfaces these at validate time.
pub fn compose_chain_data(conn: &CoinConn, network: &str) -> Result<String> {
    // Nodeless (epic #58): the pact-seed funding wallet has NO Core primary —
    // the chain data is the Electrum URL list verbatim (pactd's engine
    // dispatches to the bdk wallet when the first URL isn't http://). Nothing
    // to recompose at launch (no cookie), so auth_method stays None and
    // `effective_chain_data` uses the stored string as-is.
    if conn.funding_wallet == "pact-seed" {
        let urls: Vec<String> = conn
            .extra_backends
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        anyhow::ensure!(
            !urls.is_empty(),
            "a nodeless coin needs at least one Electrum server URL"
        );
        for url in &urls {
            anyhow::ensure!(
                url.starts_with("tcp://") || url.starts_with("ssl://"),
                "Electrum URLs must start with tcp:// or ssl:// — got {url:?}"
            );
        }
        return Ok(urls.join(","));
    }
    let host = conn.rpc_host.as_deref().unwrap_or("127.0.0.1").trim();
    let port = conn.rpc_port.context("an RPC port is required")?;
    let auth_method = conn.auth_method.as_deref().unwrap_or("cookie");
    let auth = match auth_method {
        "cookie" => {
            let datadir = conn
                .datadir
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .context("cookie auth needs a data directory to find the .cookie file")?;
            let sub = conn
                .cookie_subpath
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| coins_file::default_cookie_subpath(network));
            let path = std::path::Path::new(datadir).join(sub);
            let raw = std::fs::read_to_string(&path).with_context(|| {
                format!(
                    "could not read the node cookie at {} — is the node running?",
                    path.display()
                )
            })?;
            let cookie = raw.trim().to_string();
            if cookie.is_empty() {
                bail!("cookie file {} is empty", path.display());
            }
            cookie // already "__cookie__:hex"
        }
        "userpass" => {
            let user = conn.rpc_user.as_deref().unwrap_or("").trim();
            let pass = conn.rpc_password.as_deref().unwrap_or("");
            if user.is_empty() {
                bail!("username/password auth needs an RPC username");
            }
            format!("{user}:{pass}")
        }
        other => bail!("unknown auth method {other:?} (expected \"cookie\" or \"userpass\")"),
    };
    let wallet = conn.wallet.as_deref().unwrap_or("").trim();
    let wallet_path = if wallet.is_empty() {
        String::new()
    } else {
        format!("/wallet/{wallet}")
    };
    let primary = format!("http://{auth}@{host}:{port}{wallet_path}");
    let mut urls = vec![primary];
    urls.extend(
        conn.extra_backends
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    Ok(urls.join(","))
}

/// The backend URL list to hand pactd at launch. Recomposed from the structured
/// fields when present (so a rotated cookie is re-read each launch); if that
/// fails (e.g. the node isn't up yet) or the entry is a legacy raw one, fall
/// back to the `chain_data` string stored at save time.
pub fn effective_chain_data(conn: &CoinConn, network: &str) -> String {
    if conn.auth_method.is_some() {
        if let Ok(composed) = compose_chain_data(conn, network) {
            return composed;
        }
    }
    conn.chain_data.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(auth: &str) -> CoinConn {
        CoinConn {
            coin_id: "btc".into(),
            chain_data: String::new(),
            funding_wallet: "core-rpc".into(),
            confirmations: None,
            rpc_host: Some("127.0.0.1".into()),
            rpc_port: Some(8332),
            auth_method: Some(auth.into()),
            rpc_user: None,
            rpc_password: None,
            datadir: None,
            cookie_subpath: None,
            wallet: Some("alice".into()),
            extra_backends: vec![],
        }
    }

    #[test]
    fn composes_nodeless_pact_seed_as_electrum_only_list() {
        // Nodeless (epic #58): funding_wallet "pact-seed" ⇒ chain_data is the
        // Electrum URL list verbatim, no Core primary, whatever auth fields say.
        let mut c = base("cookie");
        c.funding_wallet = "pact-seed".into();
        c.extra_backends = vec![
            " tcp://127.0.0.1:19750 ".into(),
            "ssl://electrum.example.org:50002".into(),
        ];
        let url = compose_chain_data(&c, "regtest").unwrap();
        assert_eq!(url, "tcp://127.0.0.1:19750,ssl://electrum.example.org:50002");

        // No URLs / a non-Electrum URL must refuse.
        c.extra_backends = vec![];
        assert!(compose_chain_data(&c, "regtest").is_err());
        c.extra_backends = vec!["http://127.0.0.1:8332".into()];
        assert!(compose_chain_data(&c, "regtest").is_err());
    }

    #[test]
    fn composes_userpass_with_wallet_and_extras() {
        let mut c = base("userpass");
        c.rpc_user = Some("u".into());
        c.rpc_password = Some("p".into());
        c.extra_backends = vec!["tcp://127.0.0.1:50001".into()];
        let url = compose_chain_data(&c, "regtest").unwrap();
        assert_eq!(
            url,
            "http://u:p@127.0.0.1:8332/wallet/alice,tcp://127.0.0.1:50001"
        );
    }

    #[test]
    fn composes_cookie_from_file() {
        let dir = std::env::temp_dir().join(format!("satchel-compose-{}", std::process::id()));
        let regtest = dir.join("regtest");
        std::fs::create_dir_all(&regtest).unwrap();
        std::fs::write(regtest.join(".cookie"), "__cookie__:deadbeef\n").unwrap();
        let mut c = base("cookie");
        c.datadir = Some(dir.display().to_string());
        c.wallet = None;
        let url = compose_chain_data(&c, "regtest").unwrap();
        assert_eq!(url, "http://__cookie__:deadbeef@127.0.0.1:8332");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cookie_missing_is_a_clear_error() {
        let mut c = base("cookie");
        c.datadir = Some("/no/such/dir".into());
        let err = compose_chain_data(&c, "regtest").unwrap_err().to_string();
        assert!(err.contains("cookie"), "{err}");
    }
}
