//! Probe candidate Nostr relays for Pact eligibility.
//!
//! For each relay we check two things:
//!   1. NIP-11 policy (HTTP GET, `Accept: application/nostr+json`) — a relay
//!      that requires auth/payment/restricted-writes or a min PoW is unusable as
//!      a default, so we flag it.
//!   2. A live round-trip — publish a throwaway kind-31510 (addressable offer,
//!      with a `d` tag + NIP-40 expiration) and a kind-1059 gift-wrap (`#p`-tagged
//!      to ourselves), then fetch them back. A relay that ACCEPTS and RETAINS
//!      both is eligible for the default list; one that drops our niche kinds is
//!      not, however friendly its NIP-11 looks.
//!
//! Usage: cargo run --manifest-path tools/relay-prober/Cargo.toml -- [wss://… …]
//! With no args, probes a built-in candidate list.

use std::time::Duration;

use anyhow::Result;
use nostr_sdk::prelude::*;
use serde::Deserialize;

const OFFER_KIND: u16 = 31510;
const GIFTWRAP_KIND: u16 = 1059;
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);

// Long-running, historically free/open public relays — the starting candidates.
const DEFAULT_CANDIDATES: &[&str] = &[
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://nostr.mom",
    "wss://nostr-pub.wellorder.net",
    "wss://offchain.pub",
    "wss://relay.snort.social",
];

#[derive(Debug, Default, Deserialize)]
struct Nip11 {
    limitation: Option<Limitation>,
    supported_nips: Option<Vec<u32>>,
}

#[derive(Debug, Default, Deserialize)]
struct Limitation {
    auth_required: Option<bool>,
    payment_required: Option<bool>,
    restricted_writes: Option<bool>,
    min_pow_difficulty: Option<u32>,
}

struct Verdict {
    relay: String,
    nip11_ok: bool,
    nip11_note: String,
    offer_ok: bool,
    giftwrap_ok: bool,
}

impl Verdict {
    fn eligible(&self) -> bool {
        self.nip11_ok && self.offer_ok && self.giftwrap_ok
    }
}

/// NIP-11: fetch the relay info doc over HTTPS and judge its write policy.
async fn probe_nip11(http: &reqwest::Client, relay: &str) -> (bool, String) {
    let url = relay.replacen("wss://", "https://", 1).replacen("ws://", "http://", 1);
    let resp = http
        .get(&url)
        .header("Accept", "application/nostr+json")
        .timeout(Duration::from_secs(8))
        .send()
        .await;
    let doc: Nip11 = match resp {
        Ok(r) => match r.json().await {
            Ok(d) => d,
            Err(_) => return (true, "no NIP-11 doc (assumed open)".into()),
        },
        Err(e) => return (false, format!("unreachable: {e}")),
    };
    let l = doc.limitation.unwrap_or_default();
    let mut blockers = Vec::new();
    if l.auth_required == Some(true) {
        blockers.push("auth_required");
    }
    if l.payment_required == Some(true) {
        blockers.push("payment_required");
    }
    if l.restricted_writes == Some(true) {
        blockers.push("restricted_writes");
    }
    if l.min_pow_difficulty.unwrap_or(0) > 0 {
        blockers.push("min_pow");
    }
    let nips = doc.supported_nips.map(|n| n.len()).unwrap_or(0);
    if blockers.is_empty() {
        (true, format!("open ({nips} NIPs)"))
    } else {
        (false, blockers.join("+"))
    }
}

/// Round-trip: publish offer + gift-wrap to this one relay, then read them back.
async fn probe_roundtrip(relay: &str, keys: &Keys) -> Result<(bool, bool)> {
    let client = Client::default();
    client.add_relay(relay).await?;
    client.connect().await;

    let stamp = Timestamp::now().as_secs();
    let d = format!("relay-probe-{stamp}");
    let offer = EventBuilder::new(Kind::Custom(OFFER_KIND), "{\"probe\":true}")
        .tag(Tag::identifier(d.clone()))
        .tag(Tag::expiration(Timestamp::from(stamp + 3600)))
        .sign_with_keys(keys)?;
    let ephemeral = Keys::generate();
    let giftwrap = EventBuilder::new(Kind::Custom(GIFTWRAP_KIND), "probe")
        .tag(Tag::public_key(keys.public_key()))
        .sign_with_keys(&ephemeral)?;

    // Accepted? send_event reports per-relay success; an empty success set means
    // the relay rejected it.
    let offer_sent = client.send_event(&offer).await.map(|o| !o.success.is_empty()).unwrap_or(false);
    let gw_sent = client.send_event(&giftwrap).await.map(|o| !o.success.is_empty()).unwrap_or(false);

    // Retained? read them back by id (offer by author, gift-wrap by #p to us).
    let offer_back = if offer_sent {
        let f = Filter::new().kind(Kind::Custom(OFFER_KIND)).author(keys.public_key());
        client.fetch_events(f, FETCH_TIMEOUT).await.map(|ev| ev.iter().any(|e| e.id == offer.id)).unwrap_or(false)
    } else {
        false
    };
    let gw_back = if gw_sent {
        let f = Filter::new().kind(Kind::Custom(GIFTWRAP_KIND)).pubkey(keys.public_key());
        client.fetch_events(f, FETCH_TIMEOUT).await.map(|ev| ev.iter().any(|e| e.id == giftwrap.id)).unwrap_or(false)
    } else {
        false
    };

    let _ = client.disconnect().await;
    Ok((offer_back, gw_back))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let relays: Vec<String> = if args.is_empty() {
        DEFAULT_CANDIDATES.iter().map(|s| s.to_string()).collect()
    } else {
        args
    };

    let http = reqwest::Client::builder().user_agent("pact-relay-prober").build()?;
    let keys = Keys::generate();
    println!("Probing {} relay(s) — offer kind {OFFER_KIND}, gift-wrap kind {GIFTWRAP_KIND}\n", relays.len());

    let mut verdicts = Vec::new();
    for relay in &relays {
        print!("  {relay} … ");
        let (nip11_ok, nip11_note) = probe_nip11(&http, relay).await;
        let (offer_ok, giftwrap_ok) = match probe_roundtrip(relay, &keys).await {
            Ok(v) => v,
            Err(e) => {
                println!("connect failed: {e}");
                verdicts.push(Verdict { relay: relay.clone(), nip11_ok, nip11_note, offer_ok: false, giftwrap_ok: false });
                continue;
            }
        };
        let v = Verdict { relay: relay.clone(), nip11_ok, nip11_note, offer_ok, giftwrap_ok };
        println!(
            "nip11={} offer={} giftwrap={} => {}",
            if v.nip11_ok { "ok" } else { &v.nip11_note },
            if offer_ok { "kept" } else { "dropped" },
            if giftwrap_ok { "kept" } else { "dropped" },
            if v.eligible() { "ELIGIBLE" } else { "skip" },
        );
        verdicts.push(v);
    }

    let eligible: Vec<&Verdict> = verdicts.iter().filter(|v| v.eligible()).collect();
    println!("\n=== Eligible default relays ({}/{}) ===", eligible.len(), verdicts.len());
    for v in &eligible {
        println!("{}", v.relay);
    }
    Ok(())
}
