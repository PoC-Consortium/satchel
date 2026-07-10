//! Compose the backend URL string pactd consumes (`http://user:pass@host:port
//! /wallet/x[,extra]`) from structured per-coin connection settings.
//!
//! The structured form (host / port / auth / datadir / wallet) is what the UI
//! edits; pactd still receives the same opaque `--coin id=urls` string it always
//! has. Cookie auth wires the cookie-file PATH, not its contents (#162): the
//! URL carries a `__cookiefile__:<percent-encoded-abs-path>@` sentinel and
//! pactd reads the file live per call (re-reading on a 401), exactly like
//! `bitcoin-cli -rpccookiefile` — Satchel used to read the file here and bake
//! `__cookie__:hex` into the URL, which went stale the moment the node
//! restarted. With no datadir configured the URL carries no auth at all and
//! pactd auto-discovers the cookie in the node's platform-default data dir
//! (bitcoind's own no-flags behavior). Userpass is still resolved verbatim.

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
            // #162: never read the cookie here — the datadir+subpath join is
            // kept in this one place, but the URL carries only the resulting
            // PATH (as the `__cookiefile__:` sentinel) so pactd resolves the
            // cookie live and self-heals a node restart's 401. No datadir →
            // no auth in the URL: pactd auto-discovers the platform-default
            // cookie for the coin (bitcoind behavior).
            match conn
                .datadir
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(datadir) => {
                    let sub = conn
                        .cookie_subpath
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| coins_file::default_cookie_subpath(network));
                    let path = std::path::Path::new(datadir).join(sub);
                    format!(
                        "__cookiefile__:{}",
                        percent_encode_path(&path.display().to_string())
                    )
                }
                None => String::new(),
            }
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
    let primary = if auth.is_empty() {
        // Cookie auth without a datadir: pactd's Core-RPC client discovers
        // the node's default `.cookie` itself (#162).
        format!("http://{host}:{port}{wallet_path}")
    } else {
        format!("http://{auth}@{host}:{port}{wallet_path}")
    };
    let mut urls = vec![primary];
    urls.extend(
        conn.extra_backends
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    Ok(urls.join(","))
}

/// Percent-encode a filesystem path for the `__cookiefile__:<path>` URL
/// sentinel (decoded by pactd's `RpcClient`). Conservative: ASCII
/// alphanumerics, `-._~/`, Windows `\` separators and the drive-letter `:`
/// stay literal (they cannot confuse the parse — pactd splits the userinfo at
/// the LAST `@` and the sentinel at its FIRST `:`); everything else (`@`,
/// `,`, spaces, `%`, non-ASCII bytes) is `%XX`-encoded so a path can never
/// break URL or comma-separated URL-list parsing.
fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(b as char),
            b'-' | b'.' | b'_' | b'~' | b'/' | b'\\' | b':' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The backend URL list to hand pactd at launch. Recomposed from the structured
/// fields when present (so config edits and template changes take effect, and a
/// cookie coin always gets the live `__cookiefile__` sentinel — the cookie file
/// itself is never read here, #162); if composing fails (e.g. a missing port on
/// a hand-edited entry) or the entry is a legacy raw one, fall back to the
/// `chain_data` string stored at save time.
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
            default_seen: None,
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
        assert_eq!(
            url,
            "tcp://127.0.0.1:19750,ssl://electrum.example.org:50002"
        );

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
    fn composes_cookie_as_cookiefile_sentinel_without_reading_the_file() {
        // #162: cookie auth emits the joined PATH as the `__cookiefile__:`
        // sentinel — the file is NEVER read here (this dir doesn't even
        // exist), so a node that is down at compose time no longer matters
        // and a rotated cookie can't go stale in the URL.
        let mut c = base("cookie");
        c.datadir = Some("/no/such dir".into());
        c.wallet = None;
        let url = compose_chain_data(&c, "regtest").unwrap();
        // The datadir→subpath separator is platform-native from Path::join
        // (the subpath keeps its own '/'); the space is percent-encoded so
        // the URL (and URL lists) always parse.
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            url,
            format!("http://__cookiefile__:/no/such%20dir{sep}regtest/.cookie@127.0.0.1:8332")
        );

        // An explicit cookie_subpath wins over the network default.
        c.cookie_subpath = Some("testnet3/.cookie".into());
        let url = compose_chain_data(&c, "regtest").unwrap();
        assert!(url.contains("testnet3"), "{url}");
    }

    #[test]
    fn cookie_without_datadir_composes_bare_url_for_autodiscovery() {
        // #162 amendment: cookie auth is the default FALLBACK, never
        // mandatory to configure — no datadir means pactd auto-discovers the
        // node's default cookie, so the URL carries no auth at all.
        let c = base("cookie");
        let url = compose_chain_data(&c, "regtest").unwrap();
        assert_eq!(url, "http://127.0.0.1:8332/wallet/alice");
    }

    #[test]
    fn percent_encoding_escapes_url_breaking_bytes() {
        // '@' would break the userinfo split, ',' the URL-list split; the
        // Windows drive colon and backslashes stay literal (harmless).
        assert_eq!(
            percent_encode_path("C:\\Users\\J Doe\\App@Data\\a,b\\.cookie"),
            "C:\\Users\\J%20Doe\\App%40Data\\a%2Cb\\.cookie"
        );
        assert_eq!(
            percent_encode_path("/home/x/.bitcoin/.cookie"),
            "/home/x/.bitcoin/.cookie"
        );
    }
}
