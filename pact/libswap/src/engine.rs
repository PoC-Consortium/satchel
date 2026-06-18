//! The swap engine: drives one party's side of a swap through the spec §8
//! handshake and the §9 procedures. The CLI (and later pactd) is a thin
//! shell around this module.
//!
//! Phase 1 scope: regtest only (gate lifts per network as hardening
//! lands — PoCX testnet params are not even final yet), Core-RPC
//! backends, manual message transport.
//!
//! §6.3 compliance: the refund transaction is built and signed at funding
//! time, persisted in the swap record, and broadcast by [`Engine::tick`]
//! (pactd's scheduler) once the chain's MTP reaches T — no human present.
//! Rebuilding from seed + record remains the recovery fallback.

use anyhow::{bail, ensure, Context, Result};
use bitcoin::secp256k1::PublicKey;
use bitcoin::{OutPoint, ScriptBuf};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use crate::adaptor_swap::AdaptorState;
use crate::chain::{ChainBackend, MultiBackend};
use crate::htlc::extract_preimage;
use crate::keys::{hash_preimage, swap_id};
use crate::messages::{
    self, AbortBody, AcceptBody, ChainRef, Envelope, FundedBody, InitBody, RedeemedBody,
};
use crate::params::{ChainParams, Network};
use crate::registry;
use crate::store::{AdaptorSwapRecord, Store, SwapRecord};
use crate::swap::{
    build_redeem_tx, build_refund_tx, spend_fee_sat, Role, State, SwapParams, DUST_LIMIT_SAT,
    FUND_TX_VSIZE, MIN_SPEND_FEE_SAT, REDEEM_TX_VSIZE, REFUND_TX_VSIZE,
};

pub struct Engine {
    pub store: Store,
    /// Per-coin chain-data backends, keyed by `coin_id` (Phase C). Each value
    /// is the comma-separated backend URL list a `MultiBackend` is built from;
    /// the first entry is the wallet-qualified Core-RPC URL that also funds
    /// swaps (funding wallet = core-rpc). Owned by Satchel (`satchel.json`) and
    /// passed in at launch; pactd holds no coin config of its own.
    pub coins: BTreeMap<String, String>,
    /// Per-coin confirmation depth (reorg-safety / finality), keyed by
    /// `coin_id`. The number of confirmations before a funding/redeem on that
    /// coin is treated as final — gates auto-redeem and completion in both v1
    /// and v2. A coin absent here falls back to [`default_confirmations`].
    /// Owned by Satchel (`satchel.json`, the Coins setup page) and passed in at
    /// launch, exactly like `coins`. Local safety policy, not consensus.
    pub coin_confirmations: BTreeMap<String, u32>,
    /// Corkboard base URL; enables the relay-based handshake (sync_board).
    pub board_url: Option<String>,
    /// Nostr relay URLs (comma-separated `wss://…`). When set, a
    /// `NostrBoard` joins the board fan-out alongside any HTTP corkboard;
    /// the async relay-pool service uses the URLs, the engine only touches
    /// the local `nostr_*` buffers (docs/NOSTR_TRANSPORT.md).
    pub nostr_relays: Option<String>,
    /// Fund our HTLC leg automatically during board-driven swaps. OFF by
    /// default: funding commits real money, and an auto-funding maker can
    /// be griefed into locking funds until T1 by takers who never fund.
    /// Per-trade caps are the roadmap mitigation.
    pub auto_fund: bool,
}

fn chain_params(chain: &ChainRef) -> Result<&'static ChainParams> {
    registry::lookup(&chain.coin_id, chain.network)
        .with_context(|| format!("unsupported chain {}/{:?}", chain.coin_id, chain.network))
}

/// This party's MuSig2 signing inputs for one v2 redeem session.
struct LegSession {
    ctx: musig2::KeyAggContext,
    agg_point: musig2::secp::Point,
    my_point: musig2::secp::Point,
    my_scalar: musig2::secp::Scalar,
    _leg: crate::taproot::TaprootLeg,
}

/// Deterministic redeem sweep destination for a leg's claimer — the claimer's
/// swap key as P2TR, so both parties build the identical redeem tx. (Spec v2
/// note: production communicates a fresh core-wallet sweep address instead.)
fn adaptor_redeem_dest(chain: &ChainRef, claimer_swap: &PublicKey) -> Result<ScriptBuf> {
    let params = chain_params(chain)?;
    let xonly = claimer_swap.x_only_public_key().0;
    params.parse_address(&params.p2tr_address(&xonly)?)
}

/// Whether a v2 (adaptor) board offer is *possible* for this pair on this
/// network — both legs Taproot-capable and the adaptor allowed (built + not
/// mainnet-gated). This is "can it run v2", independent of what the default is.
fn adaptor_offer_allowed(give: &str, get: &str, network: Network) -> bool {
    let caps = |id: &str| registry::get(id).map(|c| c.capabilities);
    match (caps(give), caps(get)) {
        (Some(a), Some(b)) => {
            registry::protocols_for(a, b).contains(&registry::Protocol::Adaptor)
                && registry::adaptor_allowed(network)
        }
        _ => false,
    }
}

/// Protocol a board offer advertises by default. The whole suite defaults to
/// classic **HTLC (v1)** — auditable, battle-tested — whenever the pair supports
/// it. Only a Taproot-only pair (no HTLC) falls back to the v2 adaptor on
/// non-mainnet. v2 is otherwise opt-in: the maker pins it explicitly.
fn board_offer_protocol(give: &str, get: &str, network: Network) -> &'static str {
    let caps = |id: &str| registry::get(id).map(|c| c.capabilities);
    match (caps(give), caps(get)) {
        (Some(a), Some(b))
            if !registry::protocols_for(a, b).contains(&registry::Protocol::Htlc)
                && adaptor_offer_allowed(give, get, network) =>
        {
            crate::adaptor_swap::PROTOCOL_V2
        }
        _ => crate::PROTOCOL_VERSION,
    }
}

/// Resolve the protocol a new offer advertises. `None` uses the default
/// ([`board_offer_protocol`] — HTLC v1; v2 is opt-in); `Some` forces a choice
/// (a maker can opt into v2 for a Taproot pair). Forcing v2 on a pair/network
/// that can't run it is rejected.
fn resolve_offer_protocol(
    give: &str,
    get: &str,
    network: Network,
    forced: Option<&str>,
) -> Result<String> {
    match forced {
        None => Ok(board_offer_protocol(give, get, network).into()),
        Some(p) => {
            ensure!(
                p == crate::PROTOCOL_VERSION || p == crate::adaptor_swap::PROTOCOL_V2,
                "unknown offer protocol {p:?}"
            );
            if p == crate::adaptor_swap::PROTOCOL_V2 {
                ensure!(
                    adaptor_offer_allowed(give, get, network),
                    "{give}<->{get} cannot run v2 adaptor swaps on {network:?} (needs Taproot, non-mainnet)"
                );
            }
            Ok(p.to_string())
        }
    }
}

/// A fresh CSPRNG nonce seed (spec v2 §3.2 — nonces are never seed-derived).
fn fresh_nonce_seed() -> [u8; 32] {
    use bitcoin::secp256k1::rand::RngCore;
    let mut s = [0u8; 32];
    bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut s);
    s
}

/// BIP32 coin-type for a chain leg (spec §4.1 `coin(c)`).
fn coin_of(chain: &ChainRef) -> Result<u32> {
    registry::bip32_coin_type(&chain.coin_id)
}

/// Gate for the **v1 HTLC** entry points (`offer`/`accept`): the pair must
/// resolve to classic HTLC (CLTV + segwit v0 on both legs). v2 adaptor swaps
/// don't come through here — they use `adaptor_init`/`adaptor_accept` and the
/// board autopilot, gated by [`ensure_adaptor_supported`]. This arm is only
/// reached for a (currently unshipped) Taproot-only pair, where the caller
/// should use the adaptor path instead.
fn ensure_pair_supported(chain_a: &ChainRef, chain_b: &ChainRef) -> Result<()> {
    let caps_a = registry::get(&chain_a.coin_id)
        .with_context(|| format!("unknown coin {:?}", chain_a.coin_id))?
        .capabilities;
    let caps_b = registry::get(&chain_b.coin_id)
        .with_context(|| format!("unknown coin {:?}", chain_b.coin_id))?
        .capabilities;
    match registry::select_protocol(caps_a, caps_b) {
        Some(registry::Protocol::Htlc) => Ok(()),
        Some(registry::Protocol::Adaptor) => bail!(
            "{}<->{} resolves to a v2 adaptor swap — use the adaptor path \
             (adaptor_init/adaptor_accept or a pact-htlc-v2 board offer), not the HTLC offer",
            chain_a.coin_id,
            chain_b.coin_id
        ),
        None => bail!(
            "no supported swap protocol for {}<->{} (HTLC needs CLTV + segwit v0 on both)",
            chain_a.coin_id,
            chain_b.coin_id
        ),
    }
}

/// Ensure a pair can run a v2 adaptor swap: both legs Taproot-capable, and
/// the mainnet gate satisfied (spec v2; V2_ADAPTOR_SWAPS.md mainnet gate).
fn ensure_adaptor_supported(chain_a: &ChainRef, chain_b: &ChainRef) -> Result<()> {
    let caps_a = registry::get(&chain_a.coin_id)
        .with_context(|| format!("unknown coin {:?}", chain_a.coin_id))?
        .capabilities;
    let caps_b = registry::get(&chain_b.coin_id)
        .with_context(|| format!("unknown coin {:?}", chain_b.coin_id))?
        .capabilities;
    ensure!(
        registry::protocols_for(caps_a, caps_b).contains(&registry::Protocol::Adaptor),
        "{}<->{} does not support adaptor swaps (both legs need Taproot)",
        chain_a.coin_id,
        chain_b.coin_id
    );
    for c in [chain_a, chain_b] {
        ensure!(
            registry::adaptor_allowed(c.network),
            "{}<->{}: v2 adaptor swaps are gated on {} (mainnet) pending the security audit",
            chain_a.coin_id,
            chain_b.coin_id,
            c.coin_id
        );
    }
    Ok(())
}

pub(crate) fn local_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs()
}

/// C8 — handshake / pre-funding stall timeout (seconds). After this long with
/// no progress we (a) drop a taker-side pending take the maker never answered
/// with an `init`, and (b) auto-abort a swap stuck in a pre-funding state
/// (`created`/`accepted`). Both are SAFE because nothing is locked on-chain
/// before funding — no funds can be lost, we are only tidying dead state. Kept
/// well inside the offer TTL (24h default) so a normal slow handshake is never
/// cut short. 15 minutes also matches `init_matches_offer`'s clock-skew
/// tolerance, so a take that times out here is one the maker could no longer
/// honour anyway.
pub(crate) const PRE_FUNDING_TIMEOUT_SECS: u64 = 15 * 60;

/// Spec §7.3 network-profile duration minimums (regtest is exempt, §7.5).
/// Checked against the local clock at offer/accept time.
fn validate_profile(network: Network, t1: u32, t2: u32, n_a: u32, n_b: u32) -> Result<()> {
    if network == Network::Regtest {
        return Ok(());
    }
    let now = local_now();
    ensure!(
        u64::from(t2) >= now + 3 * 3600,
        "spec §7.3: T2 must be at least 3h away (got {}s)",
        i64::try_from(u64::from(t2))
            .unwrap_or(0)
            .saturating_sub(now as i64)
    );
    ensure!(t1 - t2 >= 4 * 3600, "spec §7.3: T1 − T2 must be ≥ 4h");
    ensure!(
        u64::from(t1) <= now + 48 * 3600,
        "spec §7.3: T1 must be ≤ 48h away"
    );
    ensure!(n_a >= 6, "spec §7.3: N_A must be ≥ 6 (got {n_a})");
    ensure!(n_b >= 1, "spec §7.3: N_B must be ≥ 1 (got {n_b})");
    Ok(())
}

/// Default confirmation requirement per chain: the 120 s chain gets the
/// higher count (spec §7.3); regtest uses 1. This is the fallback when the
/// operator has not set a per-coin depth (see [`Engine::confirmations_for`]).
pub fn default_confirmations(chain: &ChainParams) -> u32 {
    match (chain.network, chain.target_spacing_secs < 300) {
        (Network::Regtest, _) => 1,
        (_, true) => 10,
        (_, false) => 3,
    }
}

fn parse_pubkey(hex_key: &str, what: &str) -> Result<PublicKey> {
    PublicKey::from_str(hex_key).with_context(|| format!("invalid pubkey for {what}"))
}

fn parse_hash(hex_hash: &str) -> Result<[u8; 32]> {
    hex::decode(hex_hash)
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .context("hash_h must be 32 bytes of hex")
}

impl Engine {
    pub fn open(
        data_dir: &Path,
        passphrase: Option<&str>,
        coins: BTreeMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            store: Store::open(data_dir, passphrase)?,
            coins,
            coin_confirmations: BTreeMap::new(),
            board_url: None,
            nostr_relays: None,
            auto_fund: false,
        })
    }

    fn backend(&self, chain: &ChainRef) -> Result<MultiBackend> {
        let urls = self.coins.get(&chain.coin_id).with_context(|| {
            format!(
                "coin {:?} has no chain-data backend configured — set it up in Satchel \
                 (or pass --coin {0}=<url>)",
                chain.coin_id
            )
        })?;
        let backend = MultiBackend::new(chain_params(chain)?, urls)?;
        backend.verify_chain()?;
        Ok(backend)
    }

    /// Coin ids with a configured chain-data backend (display order: the
    /// shipped registry order, then any extras). Drives `listcoins`/`listpairs`.
    pub fn configured_coins(&self) -> Vec<String> {
        let mut ordered: Vec<String> = registry::REGISTRY
            .iter()
            .map(|c| c.id.to_string())
            .filter(|id| self.coins.contains_key(id))
            .collect();
        for id in self.coins.keys() {
            if !ordered.contains(id) {
                ordered.push(id.clone());
            }
        }
        ordered
    }

    /// The confirmation depth (reorg-safety / finality) to require for `chain`:
    /// the operator's per-coin setting if present, else the network/spacing
    /// [`default_confirmations`] heuristic. The single source of truth for
    /// N_a/N_b defaults across v1 and v2.
    pub fn confirmations_for(&self, chain: &ChainRef) -> Result<u32> {
        if let Some(n) = self.coin_confirmations.get(&chain.coin_id) {
            return Ok((*n).max(1));
        }
        Ok(default_confirmations(chain_params(chain)?))
    }

    /// The effective confirmation depth per *configured* coin, for `listcoins`
    /// (so the setup UI can show the value in force and its default). Returns
    /// `(effective, default)` for the given coin on `network`.
    pub fn coin_confirmations_view(&self, network: Network, coin_id: &str) -> Result<(u32, u32)> {
        let chain = ChainRef {
            coin_id: coin_id.to_string(),
            network,
        };
        let default = default_confirmations(chain_params(&chain)?);
        let effective = self
            .coin_confirmations
            .get(coin_id)
            .copied()
            .map(|n| n.max(1))
            .unwrap_or(default);
        Ok((effective, default))
    }

    /// Live connection probe for a *configured* coin: verifies the backend
    /// serves the right chain (genesis check, via [`Self::backend`]) and
    /// returns its tip height. Errors describe what is wrong with the node.
    pub fn probe_coin(&self, network: Network, coin_id: &str) -> Result<u64> {
        self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?
        .tip_height()
    }

    /// Validate a *proposed* backend set for a coin against the live node
    /// (genesis-hash check, spec §3.3) before Satchel saves it — does not
    /// touch the engine's own config. Returns the node's tip on success.
    pub fn validate_coin(&self, network: Network, coin_id: &str, chain_data: &str) -> Result<u64> {
        let params = registry::lookup(coin_id, network).with_context(|| {
            format!("unknown coin {coin_id:?} for {network:?} (not in the shipped registry)")
        })?;
        let backend = MultiBackend::new(params, chain_data)?;
        backend.verify_chain()?;
        backend.tip_height()
    }

    /// Network admission policy: regtest is free; testnet permits a
    /// plaintext seed but warns (encryption is the user's choice, as in
    /// Bitcoin Core); mainnet stays closed until the protocol and
    /// implementation have had external review.
    fn ensure_network_allowed(&self, network: Network) -> Result<()> {
        match network {
            Network::Regtest => Ok(()),
            Network::Testnet => {
                // Relaxed from a hard refusal to a warning (SATCHEL_PLAN, the
                // seed decision): an unencrypted hot transit seed is a
                // permitted trade-off — file/host access then exposes the
                // transit keys + identity, but auto-refund survives reboots
                // with no passphrase. The mainnet block below is the separate
                // audit gate and stays.
                if !self.store.seed_is_encrypted()? {
                    eprintln!(
                        "warning: running testnet with an UNENCRYPTED seed — anyone with \
                         file/host access gets the transit keys + identity. Encryption is \
                         recommended; this is permitted, like Bitcoin Core."
                    );
                }
                Ok(())
            }
            Network::Mainnet => {
                anyhow::bail!("mainnet is disabled pending external review (see pact/README.md)")
            }
        }
    }

    /// Reconstruct full SwapParams; requires the accept handshake done.
    fn swap_params(&self, rec: &SwapRecord) -> Result<SwapParams> {
        let params = SwapParams {
            chain_a: chain_params(&rec.chain_a)?,
            chain_b: chain_params(&rec.chain_b)?,
            amount_a: rec.amount_a,
            amount_b: rec.amount_b,
            hash_h: parse_hash(&rec.hash_h)?,
            t1: rec.t1,
            t2: rec.t2,
            n_a: rec.n_a,
            n_b: rec.n_b,
            alice_refund_pubkey_a: parse_pubkey(&rec.alice_refund_pubkey_a, "alice refund A")?,
            alice_redeem_pubkey_b: parse_pubkey(&rec.alice_redeem_pubkey_b, "alice redeem B")?,
            bob_redeem_pubkey_a: parse_pubkey(
                rec.bob_redeem_pubkey_a
                    .as_deref()
                    .context("handshake incomplete: no accept yet")?,
                "bob redeem A",
            )?,
            bob_refund_pubkey_b: parse_pubkey(
                rec.bob_refund_pubkey_b
                    .as_deref()
                    .context("handshake incomplete: no accept yet")?,
                "bob refund B",
            )?,
        };
        params.validate_structure()?;
        Ok(params)
    }

    fn signed_envelope(&self, msg_type: &str, swap_id: &str, body: Value) -> Result<Envelope> {
        let mut envelope = Envelope {
            v: 1,
            msg_type: msg_type.into(),
            swap_id: swap_id.into(),
            from: String::new(),
            body,
            sig: String::new(),
        };
        messages::sign(&mut envelope, &self.store.seed()?.identity_keypair()?)?;
        Ok(envelope)
    }

    /// §9 step 0, initiator: allocate index, derive H, build `init`.
    /// `n_a`/`n_b` default per spec §7.3 when not given.
    pub fn offer(
        &self,
        network: Network,
        give: (String, u64),
        get: (String, u64),
        t1: u32,
        t2: u32,
        n_a: Option<u32>,
        n_b: Option<u32>,
    ) -> Result<(SwapRecord, Envelope)> {
        ensure!(give.0 != get.0, "give and get must be different coins");
        self.ensure_network_allowed(network)?;
        let chain_a = ChainRef {
            coin_id: give.0.clone(),
            network,
        };
        let chain_b = ChainRef {
            coin_id: get.0.clone(),
            network,
        };
        ensure_pair_supported(&chain_a, &chain_b)?;
        let n_a = match n_a {
            Some(n) => n,
            None => self.confirmations_for(&chain_a)?,
        };
        let n_b = match n_b {
            Some(n) => n,
            None => self.confirmations_for(&chain_b)?,
        };
        validate_profile(network, t1, t2, n_a, n_b)?;

        let seed = self.store.seed()?;
        let index = self.store.next_swap_index()?;
        let preimage = seed.preimage(index)?;
        let hash_h = hash_preimage(&preimage);
        let id = swap_id(&hash_h);

        let alice_refund_pubkey_a = seed.swap_pubkey(coin_of(&chain_a)?, index)?.to_string();
        let alice_redeem_pubkey_b = seed.swap_pubkey(coin_of(&chain_b)?, index)?.to_string();

        let body = InitBody {
            protocol: crate::PROTOCOL_VERSION.into(),
            chain_a: chain_a.clone(),
            chain_b: chain_b.clone(),
            amount_a: give.1,
            amount_b: get.1,
            hash_h: hex::encode(hash_h),
            t1,
            t2,
            n_a,
            n_b,
            alice_refund_pubkey_a: alice_refund_pubkey_a.clone(),
            alice_redeem_pubkey_b: alice_redeem_pubkey_b.clone(),
            // No board context here; the board-driven `take` handler stamps the
            // originating offer_id into the init body before relaying (C11).
            offer_id: None,
        };

        let record = SwapRecord {
            swap_id: id.clone(),
            role: Role::Initiator,
            state: State::Created,
            created_at: local_now(),
            swap_index: index,
            chain_a,
            chain_b,
            amount_a: give.1,
            amount_b: get.1,
            hash_h: hex::encode(hash_h),
            t1,
            t2,
            n_a,
            n_b,
            alice_refund_pubkey_a,
            alice_redeem_pubkey_b,
            bob_redeem_pubkey_a: None,
            bob_refund_pubkey_b: None,
            counterparty_identity: None,
            htlc_a_txid: None,
            htlc_a_vout: None,
            htlc_b_txid: None,
            htlc_b_vout: None,
            htlc_b_height: None,
            preimage: None,
            refund_tx_hex: None,
            final_txid: None,
            final_tx_hex: None,
        };
        // Structural check on our own offer before anything is persisted.
        ensure!(t2 < t1, "spec §7.1: T2 must be < T1");
        self.store.put(&record)?;
        let envelope = self.signed_envelope("init", &id, serde_json::to_value(&body)?)?;
        Ok((record, envelope))
    }

    /// §8.3 validation + §9 step 0, participant: build `accept`.
    pub fn accept(&self, init: &Envelope) -> Result<(SwapRecord, Envelope)> {
        messages::verify(init)?;
        ensure!(
            init.msg_type == "init",
            "expected an init message, got {}",
            init.msg_type
        );
        let body: InitBody =
            serde_json::from_value(init.body.clone()).context("malformed init body")?;
        ensure!(
            body.protocol == crate::PROTOCOL_VERSION,
            "unknown protocol {} (we speak {})",
            body.protocol,
            crate::PROTOCOL_VERSION
        );
        chain_params(&body.chain_a)?;
        chain_params(&body.chain_b)?;
        ensure!(
            body.chain_a.network == body.chain_b.network,
            "both legs must be on the same network tier"
        );
        self.ensure_network_allowed(body.chain_a.network)?;
        ensure!(
            body.chain_a.coin_id != body.chain_b.coin_id,
            "chains must differ"
        );
        ensure_pair_supported(&body.chain_a, &body.chain_b)?;
        ensure!(body.t2 < body.t1, "spec §7.1: T2 must be < T1");
        ensure!(
            body.amount_a > 0 && body.amount_b > 0,
            "amounts must be positive"
        );
        validate_profile(body.chain_a.network, body.t1, body.t2, body.n_a, body.n_b)?;
        let hash_h = parse_hash(&body.hash_h)?;
        ensure!(
            init.swap_id == swap_id(&hash_h),
            "swap_id does not match hash_h (spec §4.4)"
        );
        parse_pubkey(&body.alice_refund_pubkey_a, "alice refund A")?;
        parse_pubkey(&body.alice_redeem_pubkey_b, "alice redeem B")?;

        let seed = self.store.seed()?;
        let index = self.store.next_swap_index()?;
        let bob_redeem_pubkey_a = seed
            .swap_pubkey(coin_of(&body.chain_a)?, index)?
            .to_string();
        let bob_refund_pubkey_b = seed
            .swap_pubkey(coin_of(&body.chain_b)?, index)?
            .to_string();

        let record = SwapRecord {
            swap_id: init.swap_id.clone(),
            role: Role::Participant,
            state: State::Accepted,
            created_at: local_now(),
            swap_index: index,
            chain_a: body.chain_a,
            chain_b: body.chain_b,
            amount_a: body.amount_a,
            amount_b: body.amount_b,
            hash_h: body.hash_h,
            t1: body.t1,
            t2: body.t2,
            n_a: body.n_a,
            n_b: body.n_b,
            alice_refund_pubkey_a: body.alice_refund_pubkey_a,
            alice_redeem_pubkey_b: body.alice_redeem_pubkey_b,
            bob_redeem_pubkey_a: Some(bob_redeem_pubkey_a.clone()),
            bob_refund_pubkey_b: Some(bob_refund_pubkey_b.clone()),
            counterparty_identity: Some(init.from.clone()),
            htlc_a_txid: None,
            htlc_a_vout: None,
            htlc_b_txid: None,
            htlc_b_vout: None,
            htlc_b_height: None,
            preimage: None,
            refund_tx_hex: None,
            final_txid: None,
            final_tx_hex: None,
        };
        self.store.put(&record)?;
        let body = AcceptBody {
            bob_redeem_pubkey_a,
            bob_refund_pubkey_b,
        };
        let envelope =
            self.signed_envelope("accept", &init.swap_id, serde_json::to_value(&body)?)?;
        Ok((record, envelope))
    }

    /// v2 (pact-htlc-v2) initiator: build the adaptor-swap `init` (spec v2 §7).
    /// Reserves the swap index (so the v2 keys + adaptor secret are claimed)
    /// and returns the signed `InitV2` envelope. Mainnet is refused by the
    /// gate; regtest/testnet run. Full stateful lifecycle (funding, redeem,
    /// scheduler) is the remaining daemon integration — the crypto/tx flow is
    /// proven in `adaptor_engine`.
    pub fn adaptor_init(
        &self,
        network: Network,
        give: (String, u64),
        get: (String, u64),
        t1: u32,
        t2: u32,
    ) -> Result<(AdaptorSwapRecord, Envelope)> {
        ensure!(give.0 != get.0, "give and get must be different coins");
        let (amount_a, amount_b) = (give.1, get.1);
        ensure!(amount_a > 0 && amount_b > 0, "amounts must be positive");
        self.ensure_network_allowed(network)?;
        let chain_a = ChainRef {
            coin_id: give.0,
            network,
        };
        let chain_b = ChainRef {
            coin_id: get.0,
            network,
        };
        ensure_adaptor_supported(&chain_a, &chain_b)?;
        ensure!(t2 < t1, "spec v2 §6: T2 must be < T1");

        let seed = self.store.seed()?;
        let index = self.store.next_swap_index()?;
        let adaptor_point = seed.adaptor_point(index)?;
        let body = crate::messages::InitV2Body {
            protocol: crate::adaptor_swap::PROTOCOL_V2.into(),
            chain_a: chain_a.clone(),
            chain_b: chain_b.clone(),
            amount_a,
            amount_b,
            t1,
            t2,
            alice_swap_a: seed.swap_pubkey(coin_of(&chain_a)?, index)?.to_string(),
            alice_swap_b: seed.swap_pubkey(coin_of(&chain_b)?, index)?.to_string(),
            alice_refund_a: seed
                .refund_xonly_pubkey(coin_of(&chain_a)?, index)?
                .to_string(),
            adaptor_point: adaptor_point.to_string(),
            offer_id: None,
        };
        let id = crate::keys::swap_id_v2(&adaptor_point);
        let (n_a, n_b) = (
            self.confirmations_for(&chain_a)?,
            self.confirmations_for(&chain_b)?,
        );
        let rec = AdaptorSwapRecord {
            swap_id: id.clone(),
            role: Role::Initiator,
            state: AdaptorState::Created,
            created_at: local_now(),
            swap_index: index,
            chain_a,
            chain_b,
            amount_a,
            amount_b,
            t1,
            t2,
            n_a,
            n_b,
            adaptor_point: adaptor_point.to_string(),
            alice_swap_a: body.alice_swap_a.clone(),
            alice_swap_b: body.alice_swap_b.clone(),
            alice_refund_a: body.alice_refund_a.clone(),
            bob_swap_a: None,
            bob_swap_b: None,
            bob_refund_b: None,
            counterparty_identity: None,
            funding_a_txid: None,
            funding_a_vout: None,
            funding_b_txid: None,
            funding_b_vout: None,
            their_pubnonce_a: None,
            their_pubnonce_b: None,
            their_partial_a: None,
            their_partial_b: None,
            adaptor_sig_a: None,
            adaptor_sig_b: None,
            final_txid_a: None,
            final_txid_b: None,
            final_tx_a_hex: None,
            final_tx_b_hex: None,
        };
        self.store.put_adaptor(&rec)?;
        let envelope = self.signed_envelope("init", &id, serde_json::to_value(&body)?)?;
        Ok((rec, envelope))
    }

    /// v2 participant: verify an `InitV2`, persist the swap, and build the
    /// `AcceptV2` reply. After this both sides hold every key needed to
    /// reconstruct identical Taproot legs (`AdaptorSwapParams`).
    pub fn adaptor_accept(&self, init: &Envelope) -> Result<(AdaptorSwapRecord, Envelope)> {
        messages::verify(init)?;
        ensure!(
            init.msg_type == "init",
            "expected an init message, got {}",
            init.msg_type
        );
        let body: crate::messages::InitV2Body =
            serde_json::from_value(init.body.clone()).context("malformed init-v2 body")?;
        ensure!(
            body.protocol == crate::adaptor_swap::PROTOCOL_V2,
            "unknown protocol {} (we speak {})",
            body.protocol,
            crate::adaptor_swap::PROTOCOL_V2
        );
        ensure!(
            body.chain_a.network == body.chain_b.network,
            "both legs must be on the same network"
        );
        self.ensure_network_allowed(body.chain_a.network)?;
        ensure!(
            body.chain_a.coin_id != body.chain_b.coin_id,
            "chains must differ"
        );
        ensure_adaptor_supported(&body.chain_a, &body.chain_b)?;
        ensure!(body.t2 < body.t1, "spec v2 §6: T2 must be < T1");
        ensure!(
            body.amount_a > 0 && body.amount_b > 0,
            "amounts must be positive"
        );
        ensure!(
            init.swap_id
                == crate::keys::swap_id_v2(&parse_pubkey(&body.adaptor_point, "adaptor point")?),
            "swap_id does not match the adaptor point (spec v2 §3.3)"
        );
        parse_pubkey(&body.alice_swap_a, "alice swap A")?;
        parse_pubkey(&body.alice_swap_b, "alice swap B")?;
        body.alice_refund_a
            .parse::<bitcoin::XOnlyPublicKey>()
            .context("alice refund A")?;

        let seed = self.store.seed()?;
        let index = self.store.next_swap_index()?;
        let body_out = crate::messages::AcceptV2Body {
            bob_swap_a: seed
                .swap_pubkey(coin_of(&body.chain_a)?, index)?
                .to_string(),
            bob_swap_b: seed
                .swap_pubkey(coin_of(&body.chain_b)?, index)?
                .to_string(),
            bob_refund_b: seed
                .refund_xonly_pubkey(coin_of(&body.chain_b)?, index)?
                .to_string(),
        };
        let (n_a, n_b) = (
            self.confirmations_for(&body.chain_a)?,
            self.confirmations_for(&body.chain_b)?,
        );
        let rec = AdaptorSwapRecord {
            swap_id: init.swap_id.clone(),
            role: Role::Participant,
            state: AdaptorState::Accepted,
            created_at: local_now(),
            swap_index: index,
            chain_a: body.chain_a,
            chain_b: body.chain_b,
            amount_a: body.amount_a,
            amount_b: body.amount_b,
            t1: body.t1,
            t2: body.t2,
            n_a,
            n_b,
            adaptor_point: body.adaptor_point,
            alice_swap_a: body.alice_swap_a,
            alice_swap_b: body.alice_swap_b,
            alice_refund_a: body.alice_refund_a,
            bob_swap_a: Some(body_out.bob_swap_a.clone()),
            bob_swap_b: Some(body_out.bob_swap_b.clone()),
            bob_refund_b: Some(body_out.bob_refund_b.clone()),
            counterparty_identity: Some(init.from.clone()),
            funding_a_txid: None,
            funding_a_vout: None,
            funding_b_txid: None,
            funding_b_vout: None,
            their_pubnonce_a: None,
            their_pubnonce_b: None,
            their_partial_a: None,
            their_partial_b: None,
            adaptor_sig_a: None,
            adaptor_sig_b: None,
            final_txid_a: None,
            final_txid_b: None,
            final_tx_a_hex: None,
            final_tx_b_hex: None,
        };
        self.store.put_adaptor(&rec)?;
        let envelope =
            self.signed_envelope("accept", &init.swap_id, serde_json::to_value(&body_out)?)?;
        Ok((rec, envelope))
    }

    // ---- v2 stateful lifecycle (spec v2 §7) ----

    /// Reconstruct the swap params from a record (requires the accept done).
    fn adaptor_params(
        &self,
        rec: &AdaptorSwapRecord,
    ) -> Result<crate::adaptor_swap::AdaptorSwapParams> {
        let need = |o: &Option<String>, what: &str| -> Result<String> {
            o.clone()
                .with_context(|| format!("handshake incomplete: no {what} yet"))
        };
        Ok(crate::adaptor_swap::AdaptorSwapParams {
            amount_a: rec.amount_a,
            amount_b: rec.amount_b,
            t1: rec.t1,
            t2: rec.t2,
            alice_swap_a: parse_pubkey(&rec.alice_swap_a, "alice swap A")?,
            alice_swap_b: parse_pubkey(&rec.alice_swap_b, "alice swap B")?,
            bob_swap_a: parse_pubkey(&need(&rec.bob_swap_a, "bob swap A")?, "bob swap A")?,
            bob_swap_b: parse_pubkey(&need(&rec.bob_swap_b, "bob swap B")?, "bob swap B")?,
            alice_refund_a: rec.alice_refund_a.parse().context("alice refund A")?,
            bob_refund_b: need(&rec.bob_refund_b, "bob refund B")?
                .parse()
                .context("bob refund B")?,
            adaptor_point: parse_pubkey(&rec.adaptor_point, "adaptor point")?,
        })
    }

    /// Build a leg's cooperative redeem tx + its key-path sighash. Both parties
    /// compute the identical tx: the sweep destination is deterministic (the
    /// claimer's swap key as P2TR) and the fee is a fixed feerate. (Production
    /// would communicate a fresh core-wallet sweep address.)
    fn adaptor_redeem_tx(
        &self,
        rec: &AdaptorSwapRecord,
        secp: &bitcoin::secp256k1::Secp256k1<bitcoin::secp256k1::All>,
        leg_tag: &str,
    ) -> Result<(bitcoin::Transaction, [u8; 32])> {
        let p = self.adaptor_params(rec)?;
        let fee = spend_fee_sat(2, crate::taproot::KEYPATH_REDEEM_VSIZE);
        let (leg, chain, amount, claimer, txid, vout) = if leg_tag == "redeem_b" {
            (
                p.leg_b(secp)?,
                &rec.chain_b,
                rec.amount_b,
                p.alice_swap_b,
                rec.funding_b_txid.as_deref(),
                rec.funding_b_vout,
            )
        } else {
            (
                p.leg_a(secp)?,
                &rec.chain_a,
                rec.amount_a,
                p.bob_swap_a,
                rec.funding_a_txid.as_deref(),
                rec.funding_a_vout,
            )
        };
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid.context("no funding txid for leg yet")?)?,
            vout: vout.context("no funding vout for leg yet")?,
        };
        let dest = adaptor_redeem_dest(chain, &claimer)?;
        crate::taproot::build_keypath_redeem(secp, &leg, outpoint, amount, dest, fee)
    }

    /// Per-leg signing descriptor for THIS party (key order is funder-first).
    fn leg_session(
        &self,
        rec: &AdaptorSwapRecord,
        secp: &bitcoin::secp256k1::Secp256k1<bitcoin::secp256k1::All>,
        leg_tag: &str,
    ) -> Result<LegSession> {
        let p = self.adaptor_params(rec)?;
        let seed = self.store.seed()?;
        let (leg, ctx, coin, my_point) = if leg_tag == "redeem_b" {
            // funder Bob (idx0), counterparty Alice (idx1).
            let leg = p.leg_b(secp)?;
            let ctx = crate::adaptor_swap::tweaked_ctx_for_leg(
                secp,
                &leg,
                &p.bob_swap_b,
                &p.alice_swap_b,
            )?;
            let mine = if rec.role == Role::Initiator {
                p.alice_swap_b
            } else {
                p.bob_swap_b
            };
            (leg, ctx, coin_of(&rec.chain_b)?, mine)
        } else {
            // funder Alice (idx0), counterparty Bob (idx1).
            let leg = p.leg_a(secp)?;
            let ctx = crate::adaptor_swap::tweaked_ctx_for_leg(
                secp,
                &leg,
                &p.alice_swap_a,
                &p.bob_swap_a,
            )?;
            let mine = if rec.role == Role::Initiator {
                p.alice_swap_a
            } else {
                p.bob_swap_a
            };
            (leg, ctx, coin_of(&rec.chain_a)?, mine)
        };
        let my_scalar =
            crate::musig::seckey_to_scalar(&seed.swap_secret_key(coin, rec.swap_index)?)?;
        let agg_point: musig2::secp::Point = ctx.aggregated_pubkey();
        Ok(LegSession {
            ctx,
            agg_point,
            my_point: crate::musig::pubkey_to_point(&my_point)?,
            my_scalar,
            _leg: leg,
        })
    }

    /// Record OUR funding outpoint for the leg we fund and emit `funding_ready`
    /// (spec v2 §7). `adaptor_fund` calls the wallet first; this is the
    /// chain-free recorder so it is unit-testable.
    pub fn adaptor_funding_ready(&self, swap: &str, txid: &str, vout: u32) -> Result<Envelope> {
        let mut rec = self.store.get_adaptor(swap)?;
        match rec.role {
            Role::Initiator => {
                rec.funding_a_txid = Some(txid.into());
                rec.funding_a_vout = Some(vout);
            }
            Role::Participant => {
                rec.funding_b_txid = Some(txid.into());
                rec.funding_b_vout = Some(vout);
            }
        }
        self.store.put_adaptor(&rec)?;
        let leg = if rec.role == Role::Initiator {
            "a"
        } else {
            "b"
        };
        let body = crate::messages::FundingReadyV2Body {
            chain: leg.into(),
            txid: txid.into(),
            vout,
        };
        self.signed_envelope("funding_ready", swap, serde_json::to_value(&body)?)
    }

    /// Generate OUR use-once nonces for both redeem sessions and emit `nonces`.
    pub fn adaptor_nonces(&self, swap: &str) -> Result<Envelope> {
        let rec = self.store.get_adaptor(swap)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let mut pubnonces = std::collections::BTreeMap::new();
        for leg_tag in ["redeem_a", "redeem_b"] {
            let (_tx, sighash) = self.adaptor_redeem_tx(&rec, &secp, leg_tag)?;
            let s = self.leg_session(&rec, &secp, leg_tag)?;
            let (_sn, pn) = crate::adaptor_engine::session_nonce(
                &self.store,
                swap,
                leg_tag,
                fresh_nonce_seed(),
                s.my_point,
                s.agg_point,
                &sighash,
            )?;
            pubnonces.insert(leg_tag, crate::adaptor_engine::pubnonce_hex(&pn));
        }
        let body = crate::messages::NoncesV2Body {
            redeem_a_pubnonce: pubnonces["redeem_a"].clone(),
            redeem_b_pubnonce: pubnonces["redeem_b"].clone(),
        };
        self.signed_envelope("nonces", swap, serde_json::to_value(&body)?)
    }

    /// Produce OUR partial adaptor signatures for both sessions and emit
    /// `partial_sigs`. Requires the counterparty nonces (recorded by `recv`).
    pub fn adaptor_sign(&self, swap: &str) -> Result<Envelope> {
        let rec = self.store.get_adaptor(swap)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let t_point = crate::musig::pubkey_to_point(&parse_pubkey(&rec.adaptor_point, "T")?)?;
        let mut partials = std::collections::BTreeMap::new();
        for leg_tag in ["redeem_a", "redeem_b"] {
            let their_hex = if leg_tag == "redeem_a" {
                &rec.their_pubnonce_a
            } else {
                &rec.their_pubnonce_b
            };
            let their_pn = crate::adaptor_engine::pubnonce_from_hex(
                their_hex
                    .as_deref()
                    .context("counterparty nonce not received yet")?,
            )?;
            let (_tx, sighash) = self.adaptor_redeem_tx(&rec, &secp, leg_tag)?;
            let s = self.leg_session(&rec, &secp, leg_tag)?;
            let (sn, our_pn) = crate::adaptor_engine::session_nonce(
                &self.store,
                swap,
                leg_tag,
                fresh_nonce_seed(),
                s.my_point,
                s.agg_point,
                &sighash,
            )?;
            let aggnonce = musig2::AggNonce::sum([our_pn, their_pn]);
            let partial = crate::adaptor_engine::session_partial(
                &self.store,
                swap,
                leg_tag,
                &s.ctx,
                s.my_scalar,
                sn,
                &aggnonce,
                t_point,
                &sighash,
            )?;
            partials.insert(leg_tag, crate::adaptor_engine::partial_hex(&partial));
        }
        let body = crate::messages::PartialSigsV2Body {
            redeem_a_partial: partials["redeem_a"].clone(),
            redeem_b_partial: partials["redeem_b"].clone(),
        };
        self.signed_envelope("partial_sigs", swap, serde_json::to_value(&body)?)
    }

    /// Assemble + verify both leg `AdaptorSignature`s from our partials (nonce
    /// store) and the counterparty partials (record); advance to `Signed`.
    pub fn adaptor_assemble(&self, swap: &str) -> Result<AdaptorSwapRecord> {
        let mut rec = self.store.get_adaptor(swap)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let t_point = crate::musig::pubkey_to_point(&parse_pubkey(&rec.adaptor_point, "T")?)?;
        for leg_tag in ["redeem_a", "redeem_b"] {
            let (_tx, sighash) = self.adaptor_redeem_tx(&rec, &secp, leg_tag)?;
            let s = self.leg_session(&rec, &secp, leg_tag)?;
            // Our partial: re-derive from the persisted nonce session.
            let (sn, our_pn) = crate::adaptor_engine::session_nonce(
                &self.store,
                swap,
                leg_tag,
                fresh_nonce_seed(),
                s.my_point,
                s.agg_point,
                &sighash,
            )?;
            let their_pn_hex = if leg_tag == "redeem_a" {
                &rec.their_pubnonce_a
            } else {
                &rec.their_pubnonce_b
            };
            let their_pn = crate::adaptor_engine::pubnonce_from_hex(
                their_pn_hex
                    .as_deref()
                    .context("counterparty nonce missing")?,
            )?;
            let aggnonce = musig2::AggNonce::sum([our_pn, their_pn]);
            let our_partial = crate::adaptor_engine::session_partial(
                &self.store,
                swap,
                leg_tag,
                &s.ctx,
                s.my_scalar,
                sn,
                &aggnonce,
                t_point,
                &sighash,
            )?;
            let their_partial_hex = if leg_tag == "redeem_a" {
                &rec.their_partial_a
            } else {
                &rec.their_partial_b
            };
            let their_partial = crate::adaptor_engine::partial_from_hex(
                their_partial_hex
                    .as_deref()
                    .context("counterparty partial missing")?,
            )?;
            // Funder is idx0: redeem_a -> Alice funds; redeem_b -> Bob funds.
            let we_are_funder = (leg_tag == "redeem_a" && rec.role == Role::Initiator)
                || (leg_tag == "redeem_b" && rec.role == Role::Participant);
            let ordered = if we_are_funder {
                [our_partial, their_partial]
            } else {
                [their_partial, our_partial]
            };
            let sig = crate::adaptor_engine::aggregate_adaptor(
                &s.ctx, &aggnonce, t_point, ordered, &sighash,
            )?;
            musig2::adaptor::verify_single(s.agg_point, &sig, sighash, t_point)
                .map_err(|e| anyhow::anyhow!("aggregate adaptor sig for {leg_tag} invalid: {e}"))?;
            let hexsig = crate::adaptor_engine::adaptor_sig_hex(&sig);
            if leg_tag == "redeem_a" {
                rec.adaptor_sig_a = Some(hexsig);
            } else {
                rec.adaptor_sig_b = Some(hexsig);
            }
        }
        rec.state = AdaptorState::Signed;
        self.store.put_adaptor(&rec)?;
        Ok(rec)
    }

    /// Verify + apply a counterparty v2 handshake message
    /// (accept / funding_ready / nonces / partial_sigs).
    pub fn recv_adaptor(&self, envelope: &Envelope) -> Result<AdaptorSwapRecord> {
        messages::verify(envelope)?;
        let mut rec = self.store.get_adaptor(&envelope.swap_id)?;
        match &rec.counterparty_identity {
            None => rec.counterparty_identity = Some(envelope.from.clone()),
            Some(pinned) => ensure!(
                *pinned == envelope.from,
                "message signed by {} but counterparty pinned as {pinned}",
                envelope.from
            ),
        }
        match envelope.msg_type.as_str() {
            "accept" => {
                ensure!(
                    rec.role == Role::Initiator,
                    "only the initiator receives accept"
                );
                let b: crate::messages::AcceptV2Body =
                    serde_json::from_value(envelope.body.clone())
                        .context("malformed accept-v2 body")?;
                parse_pubkey(&b.bob_swap_a, "bob swap A")?;
                parse_pubkey(&b.bob_swap_b, "bob swap B")?;
                b.bob_refund_b
                    .parse::<bitcoin::XOnlyPublicKey>()
                    .context("bob refund B")?;
                rec.bob_swap_a = Some(b.bob_swap_a);
                rec.bob_swap_b = Some(b.bob_swap_b);
                rec.bob_refund_b = Some(b.bob_refund_b);
                rec.state = AdaptorState::Accepted;
            }
            "funding_ready" => {
                let b: crate::messages::FundingReadyV2Body =
                    serde_json::from_value(envelope.body.clone())
                        .context("malformed funding_ready body")?;
                match b.chain.as_str() {
                    "a" => {
                        rec.funding_a_txid = Some(b.txid);
                        rec.funding_a_vout = Some(b.vout);
                    }
                    "b" => {
                        rec.funding_b_txid = Some(b.txid);
                        rec.funding_b_vout = Some(b.vout);
                    }
                    other => bail!("funding_ready for unknown chain {other:?}"),
                }
            }
            "nonces" => {
                let b: crate::messages::NoncesV2Body =
                    serde_json::from_value(envelope.body.clone())
                        .context("malformed nonces body")?;
                rec.their_pubnonce_a = Some(b.redeem_a_pubnonce);
                rec.their_pubnonce_b = Some(b.redeem_b_pubnonce);
            }
            "partial_sigs" => {
                let b: crate::messages::PartialSigsV2Body =
                    serde_json::from_value(envelope.body.clone())
                        .context("malformed partial_sigs body")?;
                rec.their_partial_a = Some(b.redeem_a_partial);
                rec.their_partial_b = Some(b.redeem_b_partial);
            }
            other => bail!("unknown v2 message type {other:?}"),
        }
        self.store.put_adaptor(&rec)?;
        Ok(rec)
    }

    /// Fund OUR leg's Taproot output via the core wallet, then emit
    /// `funding_ready` (spec v2 §7). Chain-touching: proven against live
    /// nodes (the in-process flow is covered by `adaptor_funding_ready`).
    pub fn adaptor_fund(&self, swap: &str) -> Result<Envelope> {
        let rec = self.store.get_adaptor(swap)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(&rec)?;
        let (chain, leg, amount) = match rec.role {
            Role::Initiator => (rec.chain_a.clone(), p.leg_a(&secp)?, rec.amount_a),
            Role::Participant => (rec.chain_b.clone(), p.leg_b(&secp)?, rec.amount_b),
        };
        let backend = self.backend(&chain)?;
        let address = leg.address(&secp, backend.params())?;
        let txid = backend.wallet_send(&address, amount)?;
        let vout = backend.find_vout(&txid, &hex::encode(leg.script_pubkey(&secp)?.as_bytes()))?;
        self.adaptor_funding_ready(swap, &txid, vout)
    }

    /// Redeem: the initiator adapts leg B with her secret `t` and broadcasts
    /// (revealing `t`); the participant extracts `t` from Alice's on-chain
    /// leg-B signature and redeems leg A. Chain-touching.
    pub fn adaptor_redeem(&self, swap: &str) -> Result<AdaptorSwapRecord> {
        let mut rec = self.store.get_adaptor(swap)?;
        ensure!(
            rec.state == AdaptorState::Signed || rec.state == AdaptorState::RedeemedB,
            "redeem in state {:?} (assemble first)",
            rec.state
        );
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let seed = self.store.seed()?;
        match rec.role {
            Role::Initiator => {
                let t = crate::musig::seckey_to_scalar(&seed.adaptor_secret(rec.swap_index)?)?;
                let sig = crate::adaptor_engine::adaptor_sig_from_hex(
                    rec.adaptor_sig_b
                        .as_deref()
                        .context("no adaptor sig for leg B")?,
                )?;
                let final_b = sig
                    .adapt::<musig2::LiftedSignature>(t)
                    .context("adapt leg B")?;
                let (mut tx, _sh) = self.adaptor_redeem_tx(&rec, &secp, "redeem_b")?;
                crate::taproot::attach_keypath_signature(
                    &mut tx,
                    crate::adaptor_swap::lifted_to_bitcoin(&final_b)?,
                );
                let txid = self.backend(&rec.chain_b)?.broadcast(&tx)?;
                rec.final_txid_b = Some(txid.to_string());
                rec.final_tx_b_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
                rec.state = AdaptorState::RedeemedB;
            }
            Role::Participant => {
                let p = self.adaptor_params(&rec)?;
                let leg_b = p.leg_b(&secp)?;
                let outpoint_b = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.funding_b_txid.as_deref().context("no leg-B funding")?,
                    )?,
                    vout: rec.funding_b_vout.context("no leg-B vout")?,
                };
                let backend_b = self.backend(&rec.chain_b)?;
                let witness = backend_b
                    .find_spend_witness(&outpoint_b, &leg_b.script_pubkey(&secp)?, 0)?
                    .context("leg B not yet redeemed by the initiator — `t` not on chain")?;
                let sig_b = crate::adaptor_engine::adaptor_sig_from_hex(
                    rec.adaptor_sig_b
                        .as_deref()
                        .context("no adaptor sig for leg B")?,
                )?;
                let t = crate::adaptor_engine::reveal_from_onchain(
                    &sig_b,
                    witness.first().context("empty witness")?,
                )?;
                let sig_a = crate::adaptor_engine::adaptor_sig_from_hex(
                    rec.adaptor_sig_a
                        .as_deref()
                        .context("no adaptor sig for leg A")?,
                )?;
                let final_a = sig_a
                    .adapt::<musig2::LiftedSignature>(t)
                    .context("adapt leg A")?;
                let (mut tx, _sh) = self.adaptor_redeem_tx(&rec, &secp, "redeem_a")?;
                crate::taproot::attach_keypath_signature(
                    &mut tx,
                    crate::adaptor_swap::lifted_to_bitcoin(&final_a)?,
                );
                let txid = self.backend(&rec.chain_a)?.broadcast(&tx)?;
                rec.final_txid_a = Some(txid.to_string());
                rec.final_tx_a_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
                rec.state = AdaptorState::Completed;
            }
        }
        self.store.put_adaptor(&rec)?;
        Ok(rec)
    }

    /// Refund OUR funded leg via its single-key CLTV tapleaf once MTP ≥ T
    /// (spec v2 §5). No MuSig2 — the unattended path. Chain-touching.
    pub fn adaptor_refund(&self, swap: &str) -> Result<AdaptorSwapRecord> {
        let mut rec = self.store.get_adaptor(swap)?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let seed = self.store.seed()?;
        let p = self.adaptor_params(&rec)?;
        let (chain, leg, amount, coin, txid_o, vout_o) = match rec.role {
            Role::Initiator => (
                rec.chain_a.clone(),
                p.leg_a(&secp)?,
                rec.amount_a,
                coin_of(&rec.chain_a)?,
                rec.funding_a_txid.clone(),
                rec.funding_a_vout,
            ),
            Role::Participant => (
                rec.chain_b.clone(),
                p.leg_b(&secp)?,
                rec.amount_b,
                coin_of(&rec.chain_b)?,
                rec.funding_b_txid.clone(),
                rec.funding_b_vout,
            ),
        };
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid_o.as_deref().context("our leg is not funded")?)?,
            vout: vout_o.context("no funding vout")?,
        };
        let backend = self.backend(&chain)?;
        let mtp = backend.tip_median_time()?;
        ensure!(
            mtp >= u64::from(leg.locktime),
            "too early to refund: MTP {mtp} < T {}",
            leg.locktime
        );
        let refund_kp = seed.refund_secret_key(coin, rec.swap_index)?.keypair(&secp);
        let dest = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        let fee = spend_fee_sat(
            backend.fee_rate_sat_per_vb()?,
            crate::taproot::SCRIPTPATH_REFUND_VSIZE,
        );
        let tx =
            crate::taproot::build_refund_tx(&secp, &leg, outpoint, amount, dest, fee, &refund_kp)?;
        let txid = backend.broadcast(&tx)?;
        let hex = bitcoin::consensus::encode::serialize_hex(&tx);
        match rec.role {
            // The refund spends our own funded leg: Alice's is leg A, Bob's leg B.
            Role::Initiator => {
                rec.final_txid_a = Some(txid.to_string());
                rec.final_tx_a_hex = Some(hex);
            }
            Role::Participant => {
                rec.final_txid_b = Some(txid.to_string());
                rec.final_tx_b_hex = Some(hex);
            }
        }
        rec.state = AdaptorState::Refunded;
        self.store.put_adaptor(&rec)?;
        Ok(rec)
    }

    /// Scheduler step for one v2 swap (called from [`Self::tick`]) — mirrors
    /// the v1 `tick_one` policy: redeem while safe, else refund after the
    /// timelock, and keep an unconfirmed spend moving (spec v2 §8, inheriting
    /// v1 §7.4). Unattended: the participant auto-claims leg A once `t` is on
    /// chain so a closed GUI never loses funds.
    ///
    /// Two reorg-safety / liveness mechanics, new in this step:
    /// - **Reveal depth gate.** The initiator does not publish `t` (redeem leg
    ///   B) until Bob's leg-B funding is `n_b` confirmations deep, so a shallow
    ///   funding cannot reorg out from under the reveal (spec v2 §8 / v1 §9.5).
    /// - **Keep the spend moving.** While a redeem/refund sits unconfirmed the
    ///   scheduler re-broadcasts it; for the single-key CLTV refund it RBF-bumps
    ///   the fee (deterministic re-sign, the unattended-safe path). The
    ///   cooperative MuSig2 redeem is re-broadcast only — its fee is committed
    ///   in the pre-signed adaptor signature and cannot change without a fresh
    ///   interactive signing round (see [`Self::adaptor_keep_moving`]).
    fn adaptor_tick_one(&self, rec: &AdaptorSwapRecord) -> Result<Option<TickEvent>> {
        use AdaptorState::*;
        let ev = |action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: action.into(),
                detail,
            }))
        };
        // Signed: drive redeem/refund. RedeemedB/Completed/Refunded: keep the
        // broadcast spend moving until it confirms. Anything else is inert.
        if !matches!(rec.state, Signed | RedeemedB | Completed | Refunded) {
            return Ok(None);
        }
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(rec)?;
        let both_funded = rec.funding_a_txid.is_some() && rec.funding_b_txid.is_some();
        let outpoint = |txid: &Option<String>, vout: Option<u32>| -> Result<OutPoint> {
            Ok(OutPoint {
                txid: bitcoin::Txid::from_str(txid.as_deref().context("leg not funded")?)?,
                vout: vout.context("no vout")?,
            })
        };

        // Post-broadcast states: nurse the unconfirmed spend to confirmation.
        match (rec.role, rec.state) {
            (Role::Initiator, RedeemedB) => {
                // Redeem-B deep enough → leg B is ours for good: advance the
                // documented RedeemedB → Completed terminal.
                return self.adaptor_keep_moving(
                    rec,
                    &rec.chain_b,
                    &rec.final_txid_b,
                    &rec.final_tx_b_hex,
                    rec.n_b,
                    false,
                    true,
                );
            }
            (Role::Participant, Completed) => {
                return self.adaptor_keep_moving(
                    rec,
                    &rec.chain_a,
                    &rec.final_txid_a,
                    &rec.final_tx_a_hex,
                    rec.n_a,
                    false,
                    false,
                );
            }
            (Role::Initiator, Refunded) => {
                return self.adaptor_keep_moving(
                    rec,
                    &rec.chain_a,
                    &rec.final_txid_a,
                    &rec.final_tx_a_hex,
                    1,
                    true,
                    false,
                );
            }
            (Role::Participant, Refunded) => {
                return self.adaptor_keep_moving(
                    rec,
                    &rec.chain_b,
                    &rec.final_txid_b,
                    &rec.final_tx_b_hex,
                    1,
                    true,
                    false,
                );
            }
            _ => {}
        }

        match rec.role {
            Role::Initiator => {
                // Redeem leg B (reveal t) once Bob's leg-B funding is n_b deep
                // and we are still before T2. The depth gate is the reveal's
                // reorg safety: never publish t against a funding that can still
                // reorg away.
                if rec.state == Signed && both_funded {
                    let backend_b = self.backend(&rec.chain_b)?;
                    if backend_b.tip_median_time()? < u64::from(rec.t2) {
                        let op_b = outpoint(&rec.funding_b_txid, rec.funding_b_vout)?;
                        let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                        match backend_b.get_txout(&op_b, &spk_b)? {
                            Some(txout) if txout.confirmations >= u64::from(rec.n_b.max(1)) => {
                                let r = self.adaptor_redeem(&rec.swap_id)?;
                                return ev(
                                    "adaptor-redeem-b",
                                    format!("revealed t; state {:?}", r.state),
                                );
                            }
                            Some(_) => return Ok(None), // funding present but too shallow — wait
                            None => return Ok(None), // not yet funded/visible — wait (T1 protects leg A)
                        }
                    }
                }
                // Else reclaim leg A after T1 if it is still unspent. Only while
                // Signed: once we've revealed t (RedeemedB/Completed) leg A is
                // the counterparty's to claim — v1 parity (it does not reclaim
                // after redeeming either).
                if rec.state == Signed && rec.funding_a_txid.is_some() {
                    let mtp_a = self.backend(&rec.chain_a)?.tip_median_time()?;
                    if mtp_a >= u64::from(rec.t1) {
                        let op = outpoint(&rec.funding_a_txid, rec.funding_a_vout)?;
                        let spk = p.leg_a(&secp)?.script_pubkey(&secp)?;
                        if self.backend(&rec.chain_a)?.get_txout(&op, &spk)?.is_some() {
                            let r = self.adaptor_refund(&rec.swap_id)?;
                            return ev("adaptor-refund-a", format!("state {:?}", r.state));
                        }
                    }
                }
                Ok(None)
            }
            Role::Participant => {
                // Claim leg A as soon as Alice's leg-B redeem reveals t. No
                // depth gate: once t is on chain it is valid even if that spend
                // later reorgs, so racing to redeem A is always correct.
                if rec.state == Signed && both_funded {
                    let op_b = outpoint(&rec.funding_b_txid, rec.funding_b_vout)?;
                    let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                    if self
                        .backend(&rec.chain_b)?
                        .find_spend_witness(&op_b, &spk_b, 0)?
                        .is_some()
                    {
                        let r = self.adaptor_redeem(&rec.swap_id)?;
                        return ev(
                            "adaptor-redeem-a",
                            format!("extracted t; state {:?}", r.state),
                        );
                    }
                }
                // Else reclaim leg B after T2 if still unspent (only while Signed,
                // i.e. before we've claimed leg A).
                if rec.state == Signed && rec.funding_b_txid.is_some() {
                    let mtp_b = self.backend(&rec.chain_b)?.tip_median_time()?;
                    if mtp_b >= u64::from(rec.t2) {
                        let op_b = outpoint(&rec.funding_b_txid, rec.funding_b_vout)?;
                        let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                        if self
                            .backend(&rec.chain_b)?
                            .get_txout(&op_b, &spk_b)?
                            .is_some()
                        {
                            let r = self.adaptor_refund(&rec.swap_id)?;
                            return ev("adaptor-refund-b", format!("state {:?}", r.state));
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    /// Keep an already-broadcast v2 spend moving until it is `target_confs`
    /// deep (spec v2 §8, inheriting v1 §7.4 "MUST fee-bump aggressively"):
    ///
    /// - Confirmed to depth → done, nothing to do.
    /// - A **refund** (`is_refund`) is RBF-bumped: rebuilt at ~50% higher fee
    ///   and re-signed with the deterministic single-key refund key — safe by
    ///   construction (no MuSig2, deterministic nonce). Falls back to a plain
    ///   rebroadcast once a higher fee would dust the output.
    /// - A cooperative **redeem** is re-broadcast unchanged: its fee is sealed
    ///   into the pre-signed adaptor signature's sighash, so it cannot be
    ///   re-fee'd without a fresh interactive MuSig2 round (out of scope without
    ///   a protocol change; see V2_ADAPTOR_SWAPS.md). Rebroadcast still recovers
    ///   from a dropped mempool entry.
    fn adaptor_keep_moving(
        &self,
        rec: &AdaptorSwapRecord,
        chain: &ChainRef,
        final_txid: &Option<String>,
        final_tx_hex: &Option<String>,
        target_confs: u32,
        is_refund: bool,
        complete_on_depth: bool,
    ) -> Result<Option<TickEvent>> {
        let (Some(txid), Some(tx_hex)) = (final_txid.as_deref(), final_tx_hex.as_deref()) else {
            return Ok(None); // record predates tx-hex persistence — nothing to nurse
        };
        let backend = self.backend(chain)?;
        let spk =
            bitcoin::consensus::encode::deserialize::<bitcoin::Transaction>(&hex::decode(tx_hex)?)
                .ok()
                .map(|tx| tx.output[0].script_pubkey.clone());
        if backend.tx_confirmations(txid, spk.as_ref())? >= u64::from(target_confs.max(1)) {
            // Confirmed deep enough — the spend is final.
            if complete_on_depth && rec.state != AdaptorState::Completed {
                let mut updated = rec.clone();
                updated.state = AdaptorState::Completed;
                self.store.put_adaptor(&updated)?;
                return Ok(Some(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "adaptor-completed".into(),
                    detail: txid.to_string(),
                }));
            }
            return Ok(None);
        }
        if is_refund {
            return self.adaptor_bump_refund(rec, &backend, tx_hex);
        }
        // Cooperative redeem: fee is fixed by the adaptor signature — rebroadcast only.
        let tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
                .context("corrupt final_tx_hex")?;
        let txid = backend.broadcast(&tx)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "adaptor-rebroadcast".into(),
            detail: format!("{txid} (cooperative redeem — fixed fee, cannot RBF)"),
        }))
    }

    /// RBF-replace an unconfirmed single-key CLTV refund at an escalated fee
    /// (spec v2 §8 / v1 §7.4). Reuses the original sweep destination and
    /// re-signs with the deterministic refund key. Mirrors v1's [`Self::maybe_bump`]:
    /// ~50% escalation, falling back to a rebroadcast once a higher fee would
    /// push the output under the dust limit.
    fn adaptor_bump_refund(
        &self,
        rec: &AdaptorSwapRecord,
        backend: &MultiBackend,
        old_tx_hex: &str,
    ) -> Result<Option<TickEvent>> {
        let old_tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(old_tx_hex)?)
                .context("corrupt refund tx hex")?;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let seed = self.store.seed()?;
        let p = self.adaptor_params(rec)?;
        // Our refunded leg: Alice's is leg A, Bob's leg B.
        let (chain, leg, amount) = match rec.role {
            Role::Initiator => (&rec.chain_a, p.leg_a(&secp)?, rec.amount_a),
            Role::Participant => (&rec.chain_b, p.leg_b(&secp)?, rec.amount_b),
        };
        let destination = old_tx.output[0].script_pubkey.clone();
        let old_fee = amount.saturating_sub(old_tx.output[0].value.to_sat());
        let new_fee = old_fee + (old_fee / 2).max(MIN_SPEND_FEE_SAT);
        if amount <= new_fee + DUST_LIMIT_SAT {
            let txid = backend.broadcast(&old_tx)?;
            return Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "adaptor-rebroadcast".into(),
                detail: txid.to_string(),
            }));
        }
        let outpoint = old_tx.input[0].previous_output;
        let refund_kp = seed
            .refund_secret_key(coin_of(chain)?, rec.swap_index)?
            .keypair(&secp);
        let new_tx = crate::taproot::build_refund_tx(
            &secp,
            &leg,
            outpoint,
            amount,
            destination,
            new_fee,
            &refund_kp,
        )?;
        let txid = backend.broadcast(&new_tx)?;
        let hex = bitcoin::consensus::encode::serialize_hex(&new_tx);
        let mut updated = rec.clone();
        match updated.role {
            Role::Initiator => {
                updated.final_txid_a = Some(txid.to_string());
                updated.final_tx_a_hex = Some(hex);
            }
            Role::Participant => {
                updated.final_txid_b = Some(txid.to_string());
                updated.final_tx_b_hex = Some(hex);
            }
        }
        self.store.put_adaptor(&updated)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "adaptor-fee-bump".into(),
            detail: format!("{txid} (refund fee {old_fee} -> {new_fee} sat)"),
        }))
    }

    /// Verify + apply a counterparty message (accept/funded/redeemed/abort).
    pub fn recv(&self, envelope: &Envelope) -> Result<SwapRecord> {
        messages::verify(envelope)?;
        let mut rec = self.store.get(&envelope.swap_id)?;
        match &rec.counterparty_identity {
            None => rec.counterparty_identity = Some(envelope.from.clone()),
            Some(pinned) => ensure!(
                *pinned == envelope.from,
                "message signed by {} but counterparty pinned as {pinned} (spec §8.2)",
                envelope.from
            ),
        }

        match envelope.msg_type.as_str() {
            "accept" => {
                ensure!(
                    rec.role == Role::Initiator,
                    "only the initiator receives accept"
                );
                ensure!(
                    rec.state == State::Created,
                    "accept in state {:?}",
                    rec.state
                );
                let body: AcceptBody = serde_json::from_value(envelope.body.clone())
                    .context("malformed accept body")?;
                parse_pubkey(&body.bob_redeem_pubkey_a, "bob redeem A")?;
                parse_pubkey(&body.bob_refund_pubkey_b, "bob refund B")?;
                rec.bob_redeem_pubkey_a = Some(body.bob_redeem_pubkey_a);
                rec.bob_refund_pubkey_b = Some(body.bob_refund_pubkey_b);
                rec.state = State::Accepted;
                // Both HTLCs must now be constructible.
                self.swap_params(&rec)?;
            }
            "funded" => {
                let body: FundedBody = serde_json::from_value(envelope.body.clone())
                    .context("malformed funded body")?;
                let params = self.swap_params(&rec)?;
                let outpoint = OutPoint {
                    txid: bitcoin::Txid::from_str(&body.txid).context("funded: bad txid")?,
                    vout: body.vout,
                };
                // §6.1: the message is a pointer, not a proof — verify the
                // output against the locally reconstructed script.
                let (chain, htlc, amount, min_conf) = match body.chain.as_str() {
                    "a" => (&rec.chain_a, params.htlc_a()?, rec.amount_a, rec.n_a),
                    "b" => (&rec.chain_b, params.htlc_b()?, rec.amount_b, rec.n_b),
                    other => bail!("funded: unknown chain {other:?}"),
                };
                let backend = self.backend(chain)?;
                let htlc_spk = htlc.script_pubkey();
                let txout = backend
                    .get_txout(&outpoint, &htlc_spk)?
                    .context("funded: outpoint not found or already spent")?;
                let expected_spk = hex::encode(htlc_spk.as_bytes());
                ensure!(
                    txout.script_pubkey_hex == expected_spk,
                    "funded: output script does not match the reconstructed HTLC (spec §5)"
                );
                ensure!(
                    txout.value_sat == amount,
                    "funded: output value {} != agreed amount {amount} (spec §6.1)",
                    txout.value_sat
                );
                ensure!(
                    txout.confirmations >= u64::from(min_conf),
                    "funded: {} confirmations < required {min_conf} (spec §7.3)",
                    txout.confirmations
                );
                match body.chain.as_str() {
                    "a" => {
                        rec.htlc_a_txid = Some(body.txid);
                        rec.htlc_a_vout = Some(body.vout);
                        rec.state = State::FundedA;
                    }
                    _ => {
                        rec.htlc_b_txid = Some(body.txid);
                        rec.htlc_b_vout = Some(body.vout);
                        rec.htlc_b_height =
                            Some(backend.tip_height()?.saturating_sub(txout.confirmations));
                        rec.state = State::FundedB;
                    }
                }
            }
            "redeemed" => {
                let body: RedeemedBody = serde_json::from_value(envelope.body.clone())
                    .context("malformed redeemed body")?;
                let preimage = parse_hash(&body.preimage)?;
                ensure!(
                    hash_preimage(&preimage) == parse_hash(&rec.hash_h)?,
                    "redeemed: preimage does not hash to H"
                );
                rec.preimage = Some(body.preimage);
            }
            "abort" => {
                let body: AbortBody =
                    serde_json::from_value(envelope.body.clone()).unwrap_or(AbortBody {
                        reason: "unspecified".into(),
                    });
                // Advisory only after funding — timelocks are the safety.
                if rec.htlc_a_txid.is_none() && rec.htlc_b_txid.is_none() {
                    rec.state = State::Aborted;
                }
                eprintln!("counterparty abort: {}", body.reason);
            }
            other => bail!("unknown message type {other:?}"),
        }
        self.store.put(&rec)?;
        Ok(rec)
    }

    /// §9.1 (initiator, chain A) / §9.2 (participant, chain B).
    pub fn fund(&self, swap: &str) -> Result<(SwapRecord, Envelope)> {
        let mut rec = self.store.get(swap)?;
        let params = self.swap_params(&rec)?;

        let (leg, chain, htlc, amount) = match rec.role {
            Role::Initiator => {
                ensure!(
                    rec.state == State::Accepted,
                    "fund in state {:?}",
                    rec.state
                );
                ("a", rec.chain_a.clone(), params.htlc_a()?, rec.amount_a)
            }
            Role::Participant => {
                ensure!(
                    rec.state == State::FundedA,
                    "participant funds only after verifying the chain-A HTLC (spec §9.2), state is {:?}",
                    rec.state
                );
                ("b", rec.chain_b.clone(), params.htlc_b()?, rec.amount_b)
            }
        };
        let backend = self.backend(&chain)?;
        if rec.role == Role::Participant {
            // §7.4: funding late only shrinks Alice's window and wastes
            // fees; regtest profile margin is zero.
            let mtp = self.backend(&rec.chain_b)?.tip_median_time()?;
            ensure!(
                mtp < u64::from(rec.t2),
                "too late to fund: chain-B MTP {mtp} already past T2 {}",
                rec.t2
            );
            // Reorg guard: re-verify the chain-A HTLC at the moment we
            // commit money, not just when the `funded` message arrived.
            let htlc_a = params.htlc_a()?;
            let outpoint_a = OutPoint {
                txid: bitcoin::Txid::from_str(
                    rec.htlc_a_txid.as_deref().context("no chain-A HTLC")?,
                )?,
                vout: rec.htlc_a_vout.context("no chain-A HTLC vout")?,
            };
            let txout = self
                .backend(&rec.chain_a)?
                .get_txout(&outpoint_a, &htlc_a.script_pubkey())?
                .context("refusing to fund: the chain-A HTLC is no longer visible (reorg?)")?;
            ensure!(
                txout.confirmations >= u64::from(rec.n_a),
                "refusing to fund: chain-A HTLC dropped to {} confirmations (reorg?)",
                txout.confirmations
            );
        }

        let address = htlc.address(backend.params())?;
        let txid = backend.wallet_send(&address, amount)?;
        let vout = backend.find_vout(&txid, &hex::encode(htlc.script_pubkey().as_bytes()))?;

        // §6.3: sign the refund NOW and persist it with the record, so a
        // scheduler can reclaim funds after T with no keys re-derived and
        // no human present.
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(&txid)?,
            vout,
        };
        let seed = self.store.seed()?;
        let key = seed.swap_secret_key(coin_of(&chain)?, rec.swap_index)?;
        let destination = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        let fee = spend_fee_sat(backend.fee_rate_sat_per_vb()?, REFUND_TX_VSIZE);
        let refund_tx = build_refund_tx(&htlc, outpoint, amount, destination, fee, &key)?;
        rec.refund_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&refund_tx));

        match leg {
            "a" => {
                rec.htlc_a_txid = Some(txid.clone());
                rec.htlc_a_vout = Some(vout);
                rec.state = State::FundedA;
            }
            _ => {
                rec.htlc_b_txid = Some(txid.clone());
                rec.htlc_b_vout = Some(vout);
                rec.htlc_b_height = Some(backend.tip_height()?);
                rec.state = State::FundedB;
            }
        }
        self.store.put(&rec)?;

        let body = FundedBody {
            chain: leg.into(),
            txid,
            vout,
        };
        let envelope = self.signed_envelope("funded", swap, serde_json::to_value(&body)?)?;
        Ok((rec, envelope))
    }

    /// §9.3 (initiator: redeem chain B, revealing s) /
    /// §9.4 (participant: extract s from chain B, redeem chain A).
    pub fn redeem(&self, swap: &str) -> Result<SwapRecord> {
        let mut rec = self.store.get(swap)?;
        let params = self.swap_params(&rec)?;
        let seed = self.store.seed()?;

        match rec.role {
            Role::Initiator => {
                ensure!(
                    rec.state == State::FundedB,
                    "redeem in state {:?}",
                    rec.state
                );
                let outpoint = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_b_txid.as_deref().context("no chain-B HTLC")?,
                    )?,
                    vout: rec.htlc_b_vout.context("no chain-B HTLC vout")?,
                };
                let backend = self.backend(&rec.chain_b)?;

                // §7.4 reveal deadline (regtest margin 0): never put s in a
                // mempool when Bob's refund could already confirm.
                let mtp = backend.tip_median_time()?;
                ensure!(
                    mtp < u64::from(rec.t2),
                    "REFUSING to redeem: chain-B MTP {mtp} has reached T2 {} — \
                     revealing s now risks losing both legs; wait for the T1 refund (spec §7.4)",
                    rec.t2
                );
                let htlc = params.htlc_b()?;
                let txout = backend
                    .get_txout(&outpoint, &htlc.script_pubkey())?
                    .context("chain-B HTLC gone")?;
                ensure!(
                    txout.confirmations >= u64::from(rec.n_b),
                    "chain-B HTLC has {} confirmations < {}",
                    txout.confirmations,
                    rec.n_b
                );

                let preimage = seed.preimage(rec.swap_index)?;
                let key = seed.swap_secret_key(coin_of(&rec.chain_b)?, rec.swap_index)?;
                let destination = backend
                    .params()
                    .parse_address(&backend.wallet_new_address()?)?;
                let fee = spend_fee_sat(backend.fee_rate_sat_per_vb()?, REDEEM_TX_VSIZE);
                let tx = build_redeem_tx(
                    &htlc,
                    outpoint,
                    rec.amount_b,
                    destination,
                    fee,
                    &preimage,
                    &key,
                )?;
                let txid = backend.broadcast(&tx)?;
                rec.preimage = Some(hex::encode(preimage));
                rec.final_txid = Some(txid.to_string());
                rec.final_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
                rec.state = State::RedeemedB;
            }
            Role::Participant => {
                ensure!(
                    matches!(rec.state, State::FundedB | State::Completed),
                    "redeem in state {:?}",
                    rec.state
                );
                let outpoint_b = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_b_txid.as_deref().context("no chain-B HTLC")?,
                    )?,
                    vout: rec.htlc_b_vout.context("no chain-B HTLC vout")?,
                };
                // Learn s: courtesy message if received, else the chain.
                let preimage = match &rec.preimage {
                    Some(hex_s) => parse_hash(hex_s)?,
                    None => {
                        let backend_b = self.backend(&rec.chain_b)?;
                        let witness = backend_b
                            .find_spend_witness(
                                &outpoint_b,
                                &params.htlc_b()?.script_pubkey(),
                                rec.htlc_b_height.unwrap_or(0),
                            )?
                            .context("chain-B HTLC not spent yet — nothing to redeem")?;
                        extract_preimage(&witness, &params.hash_h)
                            .context("chain-B spend does not reveal a valid preimage (refund?)")?
                    }
                };

                let outpoint_a = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_a_txid.as_deref().context("no chain-A HTLC")?,
                    )?,
                    vout: rec.htlc_a_vout.context("no chain-A HTLC vout")?,
                };
                let backend_a = self.backend(&rec.chain_a)?;
                let mtp = backend_a.tip_median_time()?;
                ensure!(
                    mtp < u64::from(rec.t1),
                    "chain-A MTP {mtp} has reached T1 {} — redeem would race Alice's refund (spec §7.4)",
                    rec.t1
                );

                let htlc = params.htlc_a()?;
                let key = seed.swap_secret_key(coin_of(&rec.chain_a)?, rec.swap_index)?;
                let destination = backend_a
                    .params()
                    .parse_address(&backend_a.wallet_new_address()?)?;
                let fee = spend_fee_sat(backend_a.fee_rate_sat_per_vb()?, REDEEM_TX_VSIZE);
                let tx = build_redeem_tx(
                    &htlc,
                    outpoint_a,
                    rec.amount_a,
                    destination,
                    fee,
                    &preimage,
                    &key,
                )?;
                let txid = backend_a.broadcast(&tx)?;
                rec.preimage = Some(hex::encode(preimage));
                rec.final_txid = Some(txid.to_string());
                rec.final_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
                rec.state = State::Completed;
            }
        }
        self.store.put(&rec)?;
        Ok(rec)
    }

    /// §9.5: reclaim our own HTLC once the chain's MTP reaches its T.
    pub fn refund(&self, swap: &str) -> Result<SwapRecord> {
        let mut rec = self.store.get(swap)?;
        let params = self.swap_params(&rec)?;
        let seed = self.store.seed()?;

        let (chain, htlc, outpoint, amount, locktime) = match rec.role {
            Role::Initiator => (
                rec.chain_a.clone(),
                params.htlc_a()?,
                OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_a_txid.as_deref().context("nothing funded")?,
                    )?,
                    vout: rec.htlc_a_vout.context("nothing funded")?,
                },
                rec.amount_a,
                rec.t1,
            ),
            Role::Participant => (
                rec.chain_b.clone(),
                params.htlc_b()?,
                OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_b_txid.as_deref().context("nothing funded")?,
                    )?,
                    vout: rec.htlc_b_vout.context("nothing funded")?,
                },
                rec.amount_b,
                rec.t2,
            ),
        };

        let backend = self.backend(&chain)?;
        let mtp = backend.tip_median_time()?;
        ensure!(
            mtp >= u64::from(locktime),
            "refund not yet valid: chain MTP {mtp} < T {locktime} (BIP113 lag is normal — retry later)"
        );
        ensure!(
            backend
                .get_txout(&outpoint, &htlc.script_pubkey())?
                .is_some(),
            "HTLC already spent — check whether the counterparty redeemed (status/recv)"
        );

        // Prefer the refund signed at funding time (§6.3); rebuilding from
        // seed + record is the recovery fallback for pre-§6.3 records.
        let tx = match &rec.refund_tx_hex {
            Some(tx_hex) => bitcoin::consensus::encode::deserialize::<bitcoin::Transaction>(
                &hex::decode(tx_hex).context("corrupt refund_tx_hex")?,
            )
            .context("corrupt refund_tx_hex")?,
            None => {
                let key = seed.swap_secret_key(coin_of(&chain)?, rec.swap_index)?;
                let destination = backend
                    .params()
                    .parse_address(&backend.wallet_new_address()?)?;
                let fee = spend_fee_sat(backend.fee_rate_sat_per_vb()?, REFUND_TX_VSIZE);
                build_refund_tx(&htlc, outpoint, amount, destination, fee, &key)?
            }
        };
        let txid = backend.broadcast(&tx)?;
        rec.final_txid = Some(txid.to_string());
        rec.final_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
        rec.state = State::Refunded;
        self.store.put(&rec)?;
        Ok(rec)
    }

    /// One scheduler pass over every swap — pactd runs this periodically
    /// (and `pactd --once` runs exactly one pass). Performs only chain
    /// actions, never messaging: auto-redeem when a redeem is safe and
    /// due, auto-refund once MTP passes T, bookkeeping when our final tx
    /// confirms. Errors on one swap never block the others.
    pub fn tick(&self) -> Vec<TickEvent> {
        let records = match self.store.list() {
            Ok(records) => records,
            Err(err) => {
                return vec![TickEvent {
                    swap_id: "-".into(),
                    action: "error".into(),
                    detail: format!("{err:#}"),
                }]
            }
        };
        let mut events = Vec::new();
        // C8: drop pending takes the maker never answered (no `init` within
        // the timeout). Done before the swap loop; these have no swap record
        // yet, so tick_one never sees them.
        if let Err(err) = self.prune_stale_pending_takes(&mut events) {
            events.push(TickEvent {
                swap_id: "-".into(),
                action: "error".into(),
                detail: format!("pending-take prune: {err:#}"),
            });
        }
        for record in records {
            match self.tick_one(&record) {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(err) => events.push(TickEvent {
                    swap_id: record.swap_id.clone(),
                    action: "error".into(),
                    detail: format!("{err:#}"),
                }),
            }
        }
        // v2 (pact-htlc-v2) adaptor swaps: same auto-redeem/auto-refund policy.
        for rec in self.store.list_adaptor().unwrap_or_default() {
            match self.adaptor_tick_one(&rec) {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(err) => events.push(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "error".into(),
                    detail: format!("{err:#}"),
                }),
            }
        }
        events
    }

    /// C8: abandon taker-side pending takes older than the handshake timeout.
    /// An abandoned take (maker committed elsewhere / vanished) would otherwise
    /// linger in our db forever — the only other clock on a take is the 24h
    /// offer TTL. Nothing is locked, so dropping it is safe; we just stop
    /// waiting on a dead handshake and emit a `take-timeout` event per drop.
    fn prune_stale_pending_takes(&self, events: &mut Vec<TickEvent>) -> Result<()> {
        let now = local_now();
        for (offer_id, _offer_json, created_at) in self.store.pending_takes_with_age()? {
            // `created_at == 0` is a pre-C8 row (migrated default): treat it as
            // already-stale so it is cleaned up on the first tick after upgrade.
            if now.saturating_sub(created_at) >= PRE_FUNDING_TIMEOUT_SECS {
                self.store.remove_pending_take(&offer_id)?;
                events.push(TickEvent {
                    swap_id: offer_id.clone(),
                    action: "take-timeout".into(),
                    detail: format!(
                        "no init within {}s; abandoning pending take",
                        PRE_FUNDING_TIMEOUT_SECS
                    ),
                });
            }
        }
        Ok(())
    }

    fn tick_one(&self, rec: &SwapRecord) -> Result<Option<TickEvent>> {
        let event = |action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: action.into(),
                detail,
            }))
        };

        match (rec.role, rec.state) {
            // Alice with both legs funded: redeem chain B while safe, else
            // fall back to the T1 refund of chain A.
            (Role::Initiator, State::FundedB) => {
                let backend_b = self.backend(&rec.chain_b)?;
                let outpoint_b = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_b_txid.as_deref().context("no HTLC B")?,
                    )?,
                    vout: rec.htlc_b_vout.context("no HTLC B vout")?,
                };
                let htlc_b_spk = self.swap_params(rec)?.htlc_b()?.script_pubkey();
                if backend_b.tip_median_time()? < u64::from(rec.t2) {
                    match backend_b.get_txout(&outpoint_b, &htlc_b_spk)? {
                        Some(txout) if txout.confirmations >= u64::from(rec.n_b) => {
                            let updated = self.redeem(&rec.swap_id)?;
                            return event("auto-redeem", updated.final_txid.unwrap_or_default());
                        }
                        Some(_) => return Ok(None), // waiting on confirmations
                        None => {
                            // A verified HTLC vanished without us spending
                            // it: reorged out (or in a mempool gap). No
                            // automatic action — never reveal s for an
                            // output we can't see; T1 protects our leg.
                            return event(
                                "reorg-alert",
                                format!("chain-B HTLC {outpoint_b} no longer visible"),
                            );
                        }
                    }
                }
                self.try_refund_due(rec, "a")
            }
            // Alice funded chain A but chain B never (verifiably) appeared.
            (Role::Initiator, State::FundedA) => self.try_refund_due(rec, "a"),
            // Alice's redeem broadcast: mark completed once it confirms;
            // fee-bump while it does not (§7.4: the reveal must not linger
            // in a mempool as T2 approaches).
            (Role::Initiator, State::RedeemedB) => {
                let backend_b = self.backend(&rec.chain_b)?;
                let txid = rec.final_txid.as_deref().context("no redeem txid")?;
                // Completion needs the chain's full confirmation policy,
                // not 1 conf — a shallow redeem can still reorg away, and
                // the T1 refund stays armed until this point (spec §9.5).
                if backend_b.tx_confirmations(txid, spend_spk(rec).as_ref())? >= u64::from(rec.n_b)
                {
                    let mut updated = rec.clone();
                    updated.state = State::Completed;
                    self.store.put(&updated)?;
                    return event("completed", txid.to_string());
                }
                self.maybe_bump(rec, &backend_b)
            }
            // Bob's chain-A redeem unconfirmed: bump until it lands (his
            // deadline is T1).
            (Role::Participant, State::Completed) => {
                let backend_a = self.backend(&rec.chain_a)?;
                let txid = rec.final_txid.as_deref().context("no redeem txid")?;
                if backend_a.tx_confirmations(txid, spend_spk(rec).as_ref())? >= 1 {
                    return Ok(None);
                }
                self.maybe_bump(rec, &backend_a)
            }
            // A refund that has not confirmed yet: keep it moving.
            (role, State::Refunded) => {
                let chain = match role {
                    Role::Initiator => &rec.chain_a,
                    Role::Participant => &rec.chain_b,
                };
                let backend = self.backend(chain)?;
                let txid = rec.final_txid.as_deref().context("no refund txid")?;
                if backend.tx_confirmations(txid, spend_spk(rec).as_ref())? >= 1 {
                    return Ok(None);
                }
                self.maybe_bump(rec, &backend)
            }
            // Bob with both legs funded: watch chain B for Alice's reveal;
            // redeem chain A when it appears, refund chain B after T2.
            (Role::Participant, State::FundedB) => {
                let backend_b = self.backend(&rec.chain_b)?;
                let outpoint_b = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.htlc_b_txid.as_deref().context("no HTLC B")?,
                    )?,
                    vout: rec.htlc_b_vout.context("no HTLC B vout")?,
                };
                let params = self.swap_params(rec)?;
                let spend = backend_b.find_spend_witness(
                    &outpoint_b,
                    &params.htlc_b()?.script_pubkey(),
                    rec.htlc_b_height.unwrap_or(0),
                )?;
                if let Some(witness) = spend {
                    if extract_preimage(&witness, &params.hash_h).is_some() {
                        let backend_a = self.backend(&rec.chain_a)?;
                        if backend_a.tip_median_time()? < u64::from(rec.t1) {
                            let updated = self.redeem(&rec.swap_id)?;
                            return event("auto-redeem", updated.final_txid.unwrap_or_default());
                        }
                        return Ok(None); // too late to redeem safely
                    }
                    // Spent without a preimage: that was our own refund or
                    // an anomaly; nothing to do here.
                    return Ok(None);
                }
                self.try_refund_due(rec, "b")
            }
            // C8: a swap stalled in a PRE-FUNDING state (`created`/`accepted`)
            // past the timeout is auto-aborted. Nothing is locked on-chain
            // before funding, so this loses no money — it just clears a
            // handshake the counterparty abandoned (init sent but never
            // accepted, or accept sent but the maker never funded). `abort`
            // marks the record `Aborted` and best-effort relays an `abort` to
            // the counterparty. Guarded on `created_at > 0`: a record predating
            // the timestamp field deserializes to 0 and must NOT be judged
            // infinitely old.
            (_, State::Created | State::Accepted)
                if rec.created_at > 0
                    && local_now().saturating_sub(rec.created_at) >= PRE_FUNDING_TIMEOUT_SECS =>
            {
                self.abort(&rec.swap_id, "pre-funding handshake timed out")?;
                event(
                    "abort-timeout",
                    format!("no funding within {PRE_FUNDING_TIMEOUT_SECS}s; aborted"),
                )
            }
            _ => Ok(None),
        }
    }

    /// Refund leg `leg` if its timelock has matured and the HTLC is still
    /// unspent; otherwise do nothing (the next tick retries).
    fn try_refund_due(&self, rec: &SwapRecord, leg: &str) -> Result<Option<TickEvent>> {
        let (chain, txid, vout, locktime) = match leg {
            "a" => (&rec.chain_a, &rec.htlc_a_txid, rec.htlc_a_vout, rec.t1),
            _ => (&rec.chain_b, &rec.htlc_b_txid, rec.htlc_b_vout, rec.t2),
        };
        let (Some(txid), Some(vout)) = (txid.as_deref(), vout) else {
            return Ok(None);
        };
        let backend = self.backend(chain)?;
        if backend.tip_median_time()? < u64::from(locktime) {
            return Ok(None);
        }
        let params = self.swap_params(rec)?;
        let htlc_spk = match leg {
            "a" => params.htlc_a()?.script_pubkey(),
            _ => params.htlc_b()?.script_pubkey(),
        };
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid)?,
            vout,
        };
        if backend.get_txout(&outpoint, &htlc_spk)?.is_none() {
            return Ok(None); // already spent (our refund or their redeem)
        }
        let updated = self.refund(&rec.swap_id)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "auto-refund".into(),
            detail: updated.final_txid.unwrap_or_default(),
        }))
    }

    /// RBF-replace our unconfirmed HTLC spend at an escalated fee
    /// (spec §7.4 mandates aggressive bumping near deadlines). Reuses the
    /// original destination. Once a higher fee would push the output
    /// under dust, falls back to rebroadcasting the existing tx in case
    /// mempools dropped it.
    fn maybe_bump(&self, rec: &SwapRecord, backend: &MultiBackend) -> Result<Option<TickEvent>> {
        let Some(tx_hex) = &rec.final_tx_hex else {
            return Ok(None); // record predates fee-bumping support
        };
        let old_tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
                .context("corrupt final_tx_hex")?;
        let params = self.swap_params(rec)?;
        let (htlc, chain, amount, is_redeem) = match (rec.role, rec.state) {
            (Role::Initiator, State::RedeemedB) => {
                (params.htlc_b()?, &rec.chain_b, rec.amount_b, true)
            }
            (Role::Participant, State::Completed) => {
                (params.htlc_a()?, &rec.chain_a, rec.amount_a, true)
            }
            (Role::Initiator, State::Refunded) => {
                (params.htlc_a()?, &rec.chain_a, rec.amount_a, false)
            }
            (Role::Participant, State::Refunded) => {
                (params.htlc_b()?, &rec.chain_b, rec.amount_b, false)
            }
            _ => return Ok(None),
        };

        let destination = old_tx.output[0].script_pubkey.clone();
        let old_fee = amount.saturating_sub(old_tx.output[0].value.to_sat());
        // ~50% escalation, at least the floor — comfortably clears the
        // BIP125 absolute-fee-increase requirement for our tx sizes.
        let new_fee = old_fee + (old_fee / 2).max(MIN_SPEND_FEE_SAT);
        if amount <= new_fee + DUST_LIMIT_SAT {
            let txid = backend.broadcast(&old_tx)?;
            return Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "rebroadcast".into(),
                detail: txid.to_string(),
            }));
        }

        let outpoint = old_tx.input[0].previous_output;
        let seed = self.store.seed()?;
        let key = seed.swap_secret_key(coin_of(chain)?, rec.swap_index)?;
        let new_tx = if is_redeem {
            let preimage = parse_hash(
                rec.preimage
                    .as_deref()
                    .context("no preimage for redeem bump")?,
            )?;
            build_redeem_tx(
                &htlc,
                outpoint,
                amount,
                destination,
                new_fee,
                &preimage,
                &key,
            )?
        } else {
            build_refund_tx(&htlc, outpoint, amount, destination, new_fee, &key)?
        };
        let txid = backend.broadcast(&new_tx)?;
        let mut updated = rec.clone();
        updated.final_txid = Some(txid.to_string());
        updated.final_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&new_tx));
        self.store.put(&updated)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "fee-bump".into(),
            detail: format!("{txid} (fee {old_fee} -> {new_fee} sat)"),
        }))
    }
}

// ---------------------------------------------------------------------
// Board-driven coordination (Corkboard offers + blind relay) — see
// crate::board for the flow. These methods are additive: the manual
// file-based handshake keeps working without any board.
// ---------------------------------------------------------------------

impl Engine {
    /// All configured boards (comma-separated URLs). Offers, takes,
    /// relay messages go to every board; mail is polled
    /// from every board — so two parties only need *one* board in
    /// common.
    fn boards(&self) -> Result<Vec<(String, Box<dyn crate::board::Noticeboard + '_>)>> {
        let mut boards: Vec<(String, Box<dyn crate::board::Noticeboard + '_>)> = Vec::new();
        if let Some(urls) = self.board_url.as_deref() {
            for url in urls.split(',').map(str::trim).filter(|u| !u.is_empty()) {
                boards.push((
                    url.to_string(),
                    Box::new(crate::board::BoardClient::new(url)),
                ));
            }
        }
        // One logical Nostr board aggregates all configured relays; its
        // cursor key is `relay_cursor:nostr`. The relay URLs are consumed by
        // the async service, not here — NostrBoard only reads/writes the
        // local buffers.
        let nostr_configured = self
            .nostr_relays
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if nostr_configured {
            boards.push((
                "nostr".to_string(),
                Box::new(crate::nostr_board::NostrBoard::new(&self.store)),
            ));
        }
        ensure!(
            !boards.is_empty(),
            "no boards configured (set --board-url and/or --nostr-relay)"
        );
        Ok(boards)
    }

    /// Offers from ONE configured board, for the browse view: the board named
    /// `sel` if it matches (an HTTP corkboard URL, or `"nostr"`), else the first
    /// configured. Distinct from the post/take fan-out (which hits every board)
    /// — the UI browses a single board at a time. Works for the HTTP corkboard
    /// and the Nostr board alike, since both implement [`Noticeboard`]. (This is
    /// what `boardlistoffers` calls; the old HTTP-only selector errored under a
    /// relays-only config.)
    pub fn list_board_offers(&self, sel: Option<&str>) -> Result<Vec<crate::messages::Envelope>> {
        let boards = self.boards()?;
        let chosen = match sel.map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => boards.iter().find(|(n, _)| n == name),
            None => boards.first(),
        }
        .with_context(|| match sel {
            Some(s) => format!("board '{s}' not configured"),
            None => "no boards configured".to_string(),
        })?;
        chosen.1.offers()
    }

    /// Seal to the recipient identity, then best-effort send to every
    /// board; success if any accepted. Board operators see only
    /// ciphertext addressed to a pubkey.
    fn relay_send_all(&self, to: &str, envelope: &Envelope) -> Result<()> {
        let blob = crate::board::seal_envelope(to, envelope)?;
        let mut last_err = None;
        let mut sent = false;
        for (_, board) in self.boards()? {
            match board.relay_send_blob(to, &blob) {
                Ok(()) => sent = true,
                Err(err) => last_err = Some(err),
            }
        }
        if sent {
            Ok(())
        } else {
            Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no boards")))
        }
    }

    fn identity(&self) -> Result<String> {
        Ok(self.store.seed()?.identity_pubkey()?.to_string())
    }

    /// The reference "now" for fixing absolute timelocks in board-driven
    /// swaps: the latest of our clock and both chains' MTP. A lagging
    /// local clock (or, on regtest, mocktime-advanced chains) must never
    /// produce an HTLC that is already refundable at creation.
    fn coordination_now(&self, chain_a: &ChainRef, chain_b: &ChainRef) -> Result<u64> {
        let mtp_a = self.backend(chain_a)?.tip_median_time()?;
        let mtp_b = self.backend(chain_b)?.tip_median_time()?;
        Ok(local_now().max(mtp_a).max(mtp_b))
    }

    /// Post a signed offer advert to the board. Returns the offer id.
    pub fn post_board_offer(
        &self,
        network: Network,
        give: (String, u64),
        get: (String, u64),
        t1_secs: u32,
        t2_secs: u32,
        ttl_secs: Option<u64>,
        protocol: Option<&str>,
    ) -> Result<String> {
        self.ensure_network_allowed(network)?;
        ensure!(t2_secs < t1_secs, "spec §7.1: T2 must be < T1");
        let proto = resolve_offer_protocol(&give.0, &get.0, network, protocol)?;
        let body = crate::board::OfferBody {
            protocol: proto,
            network: format!("{network:?}").to_lowercase(),
            give_asset: give.0,
            give_amount: give.1,
            get_asset: get.0,
            get_amount: get.1,
            t1_secs,
            t2_secs,
            ttl_secs,
            created: local_now(),
        };
        // Offer ids are random nonces — swaps don't exist yet.
        use bitcoin::secp256k1::rand::RngCore;
        let mut nonce = [0u8; 8];
        bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut nonce);
        let offer =
            self.signed_envelope("offer", &hex::encode(nonce), serde_json::to_value(&body)?)?;
        let mut offer_id = None;
        for (_, board) in self.boards()? {
            offer_id = Some(board.post_offer(&offer)?);
        }
        offer_id.context("no boards accepted the offer")
    }

    /// Withdraw an offer: signed revocation to every board (the listing
    /// disappears immediately) AND a local block, so a taker replaying
    /// the saved signed offer afterwards is refused. Withdrawing commits
    /// nothing — offers never lock funds.
    pub fn revoke_board_offer(&self, offer_id: &str) -> Result<()> {
        let revocation = self.signed_envelope("revoke", offer_id, serde_json::json!({}))?;
        self.store
            .meta_set(&format!("offer_revoked:{offer_id}"), "1")?;
        let mut last_err = None;
        for (_, board) in self.boards()? {
            if let Err(err) = board.revoke(&revocation) {
                last_err = Some(err);
            }
        }
        match last_err {
            Some(err) => {
                Err(err.context("local block recorded, but a board rejected the revocation"))
            }
            None => Ok(()),
        }
    }

    /// Take an offer from the board: remember it, signal interest to the
    /// maker through the relay (echoing the maker's signed offer so they
    /// can rebuild terms statelessly).
    pub fn take_board_offer(&self, offer_id: &str) -> Result<()> {
        let offer = self
            .boards()?
            .iter()
            .find_map(|(_, board)| {
                board
                    .offers()
                    .ok()?
                    .into_iter()
                    .find(|o| o.swap_id == offer_id)
            })
            .with_context(|| format!("offer {offer_id} not on any configured board"))?;
        messages::verify(&offer)?;
        let body: crate::board::OfferBody =
            serde_json::from_value(offer.body.clone()).context("malformed offer body")?;
        ensure!(
            body.protocol == crate::PROTOCOL_VERSION
                || body.protocol == crate::adaptor_swap::PROTOCOL_V2,
            "offer protocol {} unsupported",
            body.protocol
        );
        ensure!(!body.expired(local_now()), "offer has expired");
        ensure!(offer.from != self.identity()?, "that is our own offer");
        self.store
            .put_pending_take(offer_id, &serde_json::to_string(&offer)?, local_now())?;
        let take = self.signed_envelope(
            "take",
            offer_id,
            serde_json::json!({ "offer": serde_json::to_value(&offer)? }),
        )?;
        self.relay_send_all(&offer.from, &take)
    }

    /// Outstanding pending takes (post-`boardtake`, pre-record). Read-only; the
    /// UI renders these as "initiating" pre-swaps that resolve into a real swap
    /// once the maker inits, or vanish on reject/timeout.
    pub fn list_pending_takes(&self) -> Result<Vec<PendingTakeInfo>> {
        let mut out = Vec::new();
        for (offer_id, offer_json, created_at) in self.store.pending_takes_with_age()? {
            let offer: Envelope = serde_json::from_str(&offer_json)?;
            out.push(PendingTakeInfo {
                offer_id,
                from: offer.from,
                body: offer.body,
                created_at,
            });
        }
        Ok(out)
    }

    // -----------------------------------------------------------------
    // Private (off-market) offers — PRIVATE_OFFERS.md. A private offer is
    // the SAME signed `offer` envelope a board offer is, built and stored
    // locally, but NEVER posted to a board. It is handed to a friend as a
    // "slip" (pact_proto::slip) over their own chat. The friend's
    // `take_offer_slip` relays a `take` straight to the maker's mailbox, so
    // the existing take->init->accept->swap path runs unchanged. The only
    // difference from `post_board_offer` is: no HTTP POST, and a local copy
    // kept under `private_offer:<id>` so the maker can list/cancel and the
    // take handler's revoke/served guards apply.
    // -----------------------------------------------------------------

    /// Build + sign a private offer (identical envelope to `post_board_offer`),
    /// store it locally, and return a pasteable slip. Does NOT touch any board.
    pub fn make_private_offer(
        &self,
        network: Network,
        give: (String, u64),
        get: (String, u64),
        t1_secs: u32,
        t2_secs: u32,
        ttl_secs: Option<u64>,
        protocol: Option<&str>,
    ) -> Result<String> {
        self.ensure_network_allowed(network)?;
        ensure!(give.0 != get.0, "give and get must be different coins");
        ensure!(t2_secs < t1_secs, "spec §7.1: T2 must be < T1");
        // Reject unknown coins / unsupported pairs up front, exactly as a board
        // offer would be (so a slip never advertises a pair the engine can't run).
        let chain_a = ChainRef {
            coin_id: give.0.clone(),
            network,
        };
        let chain_b = ChainRef {
            coin_id: get.0.clone(),
            network,
        };
        ensure_pair_supported(&chain_a, &chain_b)?;
        let proto = resolve_offer_protocol(&give.0, &get.0, network, protocol)?;

        let body = crate::board::OfferBody {
            protocol: proto,
            network: format!("{network:?}").to_lowercase(),
            give_asset: give.0,
            give_amount: give.1,
            get_asset: get.0,
            get_amount: get.1,
            t1_secs,
            t2_secs,
            ttl_secs,
            created: local_now(),
        };
        // Offer ids are random nonces — no swap exists yet (same as the board).
        use bitcoin::secp256k1::rand::RngCore;
        let mut nonce = [0u8; 8];
        bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut nonce);
        let offer =
            self.signed_envelope("offer", &hex::encode(nonce), serde_json::to_value(&body)?)?;
        // Store locally so the incoming `take` is recognized (the take handler
        // reconstructs the offer from the take and verifies our own sig, so it
        // needs NO lookup — but the `offer_revoked`/`offer_served` guards and
        // list/cancel below read this), and so `list_private_offers` can show it.
        self.store.meta_set(
            &format!("private_offer:{}", offer.swap_id),
            &serde_json::to_string(&offer)?,
        )?;
        pact_proto::slip::encode_slip(&offer)
    }

    /// Take an offer delivered as a slip: decode + verify, run the same
    /// guards `take_board_offer` runs, then relay the `take` to the maker.
    /// This is `take_board_offer` with the offer sourced from the slip blob
    /// instead of a board GET — the take body still echoes the maker's full
    /// signed offer, so the maker proceeds with zero local state.
    pub fn take_offer_slip(&self, slip: &str) -> Result<()> {
        // decode_slip already rejects unknown prefix / bad base64 / non-offer /
        // bad signature, so the envelope here is a verified `offer`.
        let offer = pact_proto::slip::decode_slip(slip)?;
        let body: crate::board::OfferBody =
            serde_json::from_value(offer.body.clone()).context("malformed offer body")?;
        ensure!(
            body.protocol == crate::PROTOCOL_VERSION
                || body.protocol == crate::adaptor_swap::PROTOCOL_V2,
            "offer protocol {} unsupported",
            body.protocol
        );
        ensure!(!body.expired(local_now()), "offer has expired");
        ensure!(
            offer.from != self.identity()?,
            "that is our own private offer"
        );
        // Same pair-support gate as a board take (network from the signed body).
        let network = match body.network.as_str() {
            "regtest" => Network::Regtest,
            "testnet" => Network::Testnet,
            "mainnet" => Network::Mainnet,
            other => bail!("unsupported network in slip: {other}"),
        };
        self.ensure_network_allowed(network)?;
        let chain_a = ChainRef {
            coin_id: body.give_asset.clone(),
            network,
        };
        let chain_b = ChainRef {
            coin_id: body.get_asset.clone(),
            network,
        };
        ensure_pair_supported(&chain_a, &chain_b)?;

        self.store.put_pending_take(
            &offer.swap_id,
            &serde_json::to_string(&offer)?,
            local_now(),
        )?;
        let take = self.signed_envelope(
            "take",
            &offer.swap_id,
            serde_json::json!({ "offer": serde_json::to_value(&offer)? }),
        )?;
        self.relay_send_all(&offer.from, &take)
    }

    /// The locally-stored private offers (those still outstanding). Mirrors the
    /// fields the board offer cards show. Corrupt rows are skipped, not fatal.
    pub fn list_private_offers(&self) -> Result<Vec<PrivateOfferInfo>> {
        let mut out = Vec::new();
        for (_key, json) in self.store.meta_with_prefix("private_offer:")? {
            let Ok(offer) = serde_json::from_str::<Envelope>(&json) else {
                continue;
            };
            let Ok(body) = serde_json::from_value::<crate::board::OfferBody>(offer.body.clone())
            else {
                continue;
            };
            // A cancelled offer keeps its row only until the next cancel deletes
            // it; defensively hide any that carry a revoke marker.
            if self
                .store
                .meta_get(&format!("offer_revoked:{}", offer.swap_id))?
                .is_some()
            {
                continue;
            }
            let expiry = if body.created == 0 {
                0
            } else {
                body.created + body.ttl_secs.unwrap_or(24 * 3600)
            };
            // Compute `expired` before moving body's String fields into the struct.
            let expired = body.expired(local_now());
            out.push(PrivateOfferInfo {
                offer_id: offer.swap_id,
                give_asset: body.give_asset,
                give_amount: body.give_amount,
                get_asset: body.get_asset,
                get_amount: body.get_amount,
                t1_secs: body.t1_secs,
                t2_secs: body.t2_secs,
                created: body.created,
                expiry,
                expired,
            });
        }
        Ok(out)
    }

    /// Cancel a private offer: set the same `offer_revoked:<id>` marker the
    /// board-revoke path sets (so the `take` handler rejects any late take that
    /// still holds our signed slip), and drop the local row. There is no board
    /// to notify — a private offer was never listed anywhere.
    pub fn cancel_private_offer(&self, offer_id: &str) -> Result<()> {
        ensure!(
            self.store
                .meta_get(&format!("private_offer:{offer_id}"))?
                .is_some(),
            "no private offer {offer_id}"
        );
        self.store
            .meta_set(&format!("offer_revoked:{offer_id}"), "1")?;
        self.store.meta_del(&format!("private_offer:{offer_id}"))?;
        Ok(())
    }

    /// One coordination pass: drain our relay mail and act on it. Chain actions
    /// stay in tick(); this layer only moves envelopes. Errors on one message
    /// never block the rest, and the cursor always advances (no poison-message
    /// loops).
    pub fn sync_board(&self) -> Vec<TickEvent> {
        let mut events = Vec::new();
        let boards = match self.boards() {
            Ok(boards) => boards,
            Err(_) => return events, // no board configured: nothing to do
        };
        // A message that fails transiently (e.g. `funded` arriving before
        // its confirmation) must NOT be consumed: keep the cursor, retry
        // next pass, and process strictly in order per board. A poison
        // message is skipped only after MAX_ATTEMPTS.
        const MAX_ATTEMPTS: u32 = 10;
        for (url, board) in &boards {
            let result: Result<()> = (|| {
                let cursor_key = format!("relay_cursor:{url}");
                let cursor: i64 = self
                    .store
                    .meta_get(&cursor_key)?
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let poll = self.signed_envelope(
                    "relay_poll",
                    "-",
                    serde_json::json!({ "since_id": cursor }),
                )?;
                let identity = self.store.seed()?.identity_keypair()?;
                let mail = board.relay_poll(&poll)?;
                for (id, blob) in mail {
                    let envelope = match crate::board::open_envelope(&identity, &blob) {
                        Ok(envelope) => envelope,
                        Err(_) => {
                            // Undecryptable junk mail: skip, cursor advances.
                            self.store.meta_set(&cursor_key, &id.to_string())?;
                            continue;
                        }
                    };
                    match self.handle_relay_envelope(&envelope) {
                        Ok(Some(event)) => events.push(event),
                        Ok(None) => {}
                        Err(err) => {
                            let retry_key = format!("relay_retry:{url}:{id}");
                            let attempts: u32 = self
                                .store
                                .meta_get(&retry_key)?
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0)
                                + 1;
                            if attempts < MAX_ATTEMPTS {
                                self.store.meta_set(&retry_key, &attempts.to_string())?;
                                events.push(TickEvent {
                                    swap_id: envelope.swap_id.clone(),
                                    action: "relay-retry".into(),
                                    detail: format!("attempt {attempts}: {err:#}"),
                                });
                                return Ok(()); // keep cursor + ordering; retry next pass
                            }
                            events.push(TickEvent {
                                swap_id: envelope.swap_id.clone(),
                                action: "relay-error".into(),
                                detail: format!("gave up after {attempts} attempts: {err:#}"),
                            });
                        }
                    }
                    self.store.meta_set(&cursor_key, &id.to_string())?;
                }
                Ok(())
            })();
            if let Err(err) = result {
                events.push(TickEvent {
                    swap_id: "-".into(),
                    action: "error".into(),
                    detail: format!("board {url}: {err:#}"),
                });
            }
        }
        events
    }

    /// Tell a rejected taker the offer is gone (instead of silence, which
    /// would leave their pending take dangling forever).
    fn reject_take(&self, taker: &str, offer_id: &str, reason: &str) -> Result<()> {
        let abort =
            self.signed_envelope("abort", offer_id, serde_json::json!({ "reason": reason }))?;
        self.relay_send_all(taker, &abort)
    }

    /// C11: find the pending take an incoming `init` fulfils. Prefer the
    /// offer_id the maker echoed in the init body (`echoed_offer_id`), so two
    /// concurrent takes from the SAME maker each resolve to their own take
    /// instead of cross-matching (which made `init_matches_offer` reject the
    /// mismatched one). Falls back to matching by maker identity alone when the
    /// init omits the offer_id (pre-C11 makers / direct boardless inits) —
    /// correct whenever there is only one pending take with that maker. The
    /// maker-identity check is always applied as a guard so a stray offer_id
    /// can never bind an init to a different maker's take.
    fn match_pending_take(
        &self,
        from: &str,
        echoed_offer_id: Option<&str>,
    ) -> Result<Option<(String, Envelope)>> {
        for (offer_id, offer_json) in self.store.pending_takes()? {
            let offer: Envelope = serde_json::from_str(&offer_json)?;
            let hit = match echoed_offer_id {
                Some(id) => offer_id == id && offer.from == from,
                None => offer.from == from,
            };
            if hit {
                return Ok(Some((offer_id, offer)));
            }
        }
        Ok(None)
    }

    // ---- v2 (pact-htlc-v2) board-driven autopilot ----

    fn adaptor_my_leg_funded(&self, rec: &AdaptorSwapRecord) -> bool {
        match rec.role {
            Role::Initiator => rec.funding_a_txid.is_some(),
            Role::Participant => rec.funding_b_txid.is_some(),
        }
    }
    fn adaptor_my_nonces_sent(&self, swap: &str) -> bool {
        matches!(self.store.nonce_session(swap, "redeem_a"), Ok(Some(_)))
    }
    fn adaptor_my_partial_sent(&self, swap: &str) -> bool {
        matches!(
            self.store.nonce_session(swap, "redeem_a"),
            Ok(Some(s)) if s.state == crate::store::NonceState::Consumed
        )
    }

    /// After a v2 handshake message is applied (`recv_adaptor`), advance the
    /// swap one step and relay the next message — the unattended board
    /// autopilot, mirroring v1. Idempotent / order-independent: emits at most
    /// one outgoing message per call from the record + nonce state. Funding is
    /// gated on `auto_fund` (it commits money); nonce/sign/assemble are safe to
    /// automate (no new funds); redeem is the scheduler's job (`tick`).
    fn drive_adaptor_relay(
        &self,
        msg_type: &str,
        rec: &AdaptorSwapRecord,
        counterparty: &str,
    ) -> Result<Option<TickEvent>> {
        let swap = rec.swap_id.as_str();
        let ev = |action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: swap.into(),
                action: action.into(),
                detail,
            }))
        };
        let both_funded = rec.funding_a_txid.is_some() && rec.funding_b_txid.is_some();

        // 1. Fund my leg: initiator on `accept`; participant once leg A is in.
        if self.auto_fund && !self.adaptor_my_leg_funded(rec) {
            let ready = match rec.role {
                Role::Initiator => msg_type == "accept",
                Role::Participant => rec.funding_a_txid.is_some(),
            };
            if ready {
                let fr = self.adaptor_fund(swap)?;
                self.relay_send_all(counterparty, &fr)?;
                return ev("adaptor-fund", "broadcast + funding_ready".into());
            }
        }

        // 2. Both funded: exchange public nonces (initiator opens; participant
        //    answers once it holds the initiator's).
        if both_funded
            && !self.adaptor_my_nonces_sent(swap)
            && (rec.role == Role::Initiator || rec.their_pubnonce_a.is_some())
        {
            let n = self.adaptor_nonces(swap)?;
            self.relay_send_all(counterparty, &n)?;
            return ev("adaptor-nonces", "sent public nonces".into());
        }

        // 3. Both nonce sets in: send my partial adaptor signatures.
        if self.adaptor_my_nonces_sent(swap)
            && rec.their_pubnonce_a.is_some()
            && !self.adaptor_my_partial_sent(swap)
        {
            let p = self.adaptor_sign(swap)?;
            self.relay_send_all(counterparty, &p)?;
            // fall through: if the counterparty partial is already in, assemble.
        }

        // 4. Both partials in: assemble + verify (state -> Signed).
        if rec.their_partial_a.is_some()
            && self.adaptor_my_partial_sent(swap)
            && rec.adaptor_sig_a.is_none()
        {
            let r = self.adaptor_assemble(swap)?;
            return ev("adaptor-assembled", format!("state {:?}", r.state));
        }
        if self.adaptor_my_partial_sent(swap) {
            return ev("adaptor-signed", "partial adaptor sig sent".into());
        }
        ev("adaptor-recv", msg_type.into())
    }

    fn handle_relay_envelope(&self, envelope: &Envelope) -> Result<Option<TickEvent>> {
        let event = |swap_id: &str, action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: swap_id.into(),
                action: action.into(),
                detail,
            }))
        };
        match envelope.msg_type.as_str() {
            // We are the maker: someone took our offer.
            "take" => {
                let me = self.identity()?;
                let (offer, body) = crate::board::offer_from_take(envelope, &me)?;
                // Withdrawn or expired offers are refused even though the
                // taker holds our valid signature — revocation is enforced
                // here, not just on the board listing.
                if self
                    .store
                    .meta_get(&format!("offer_revoked:{}", offer.swap_id))?
                    .is_some()
                {
                    self.reject_take(&envelope.from, &offer.swap_id, "offer withdrawn")?;
                    return event(&offer.swap_id, "take-rejected", "offer withdrawn".into());
                }
                if body.expired(local_now()) {
                    self.reject_take(&envelope.from, &offer.swap_id, "offer expired")?;
                    return event(&offer.swap_id, "take-rejected", "offer expired".into());
                }
                // Fixed-size offers, no partial fills: first take wins.
                let served_key = format!("offer_served:{}", offer.swap_id);
                if self.store.meta_get(&served_key)?.is_some() {
                    self.reject_take(&envelope.from, &offer.swap_id, "offer no longer available")?;
                    return event(
                        &offer.swap_id,
                        "take-rejected",
                        "offer already served".into(),
                    );
                }
                let network = match body.network.as_str() {
                    "regtest" => Network::Regtest,
                    "testnet" => Network::Testnet,
                    other => bail!("unsupported network in offer: {other}"),
                };
                // Coin ids come straight from the (signed) offer body; the
                // registry/backend routing validates them (offer() rejects
                // unknown coins or unsupported pairs).
                let chain_a = ChainRef {
                    coin_id: body.give_asset.clone(),
                    network,
                };
                let chain_b = ChainRef {
                    coin_id: body.get_asset.clone(),
                    network,
                };
                let now = self.coordination_now(&chain_a, &chain_b)? as u32;
                let give = (body.give_asset.clone(), body.give_amount);
                let get = (body.get_asset.clone(), body.get_amount);
                let (t1, t2) = (now + body.t1_secs, now + body.t2_secs);
                // v2 (pact-htlc-v2) offers build an adaptor init; v1 the HTLC
                // init. The taker branches the same way on the init protocol.
                let (swap_id, init) = if body.protocol == crate::adaptor_swap::PROTOCOL_V2 {
                    let (mut rec, init) = self.adaptor_init(network, give, get, t1, t2)?;
                    rec.counterparty_identity = Some(envelope.from.clone()); // pin taker
                    self.store.put_adaptor(&rec)?;
                    (rec.swap_id, init)
                } else {
                    let (mut rec, init) = self.offer(network, give, get, t1, t2, None, None)?;
                    rec.counterparty_identity = Some(envelope.from.clone()); // pin taker
                    self.store.put(&rec)?;
                    (rec.swap_id, init)
                };
                self.store.meta_set(&served_key, &swap_id)?;
                // C11: stamp the originating offer_id into the init body and
                // re-sign, so the taker can match this init to the exact
                // pending take even when it holds several with us. `offer()`
                // builds the body without it (it has no board context); we add
                // it here where `offer.swap_id` is known. Re-signing over the
                // same swap_id + amended body keeps every downstream check
                // (`accept` deserialization, `init_matches_offer`) valid.
                let mut init = init;
                init.body["offer_id"] = serde_json::Value::String(offer.swap_id.clone());
                messages::sign(&mut init, &self.store.seed()?.identity_keypair()?)?;
                self.relay_send_all(&envelope.from, &init)?;
                // C5: maker auto-revoke-on-commit. Committing to a swap is the
                // mechanism by which the offer becomes "no longer available":
                // we post the signed `boardrevoke` so the listing disappears
                // for everyone (shown as "withdrawn", never "taken by X" — the
                // board never learns who took it, preserving the content-blind
                // bulletin model). This is best-effort: even if it fails, the
                // local `offer_served`/`offer_revoked` guards above reject any
                // late take, and C8's take timeout + board liveness cleanup are
                // the backstop for a maker that crashes between commit and
                // revoke.
                if let Err(err) = self.revoke_board_offer(&offer.swap_id) {
                    // Non-fatal: late takes are rejected above anyway.
                    eprintln!("warning: could not delist served offer: {err:#}");
                }
                event(&swap_id, "take->init", format!("offer {}", offer.swap_id))
            }
            // We are the taker: the maker sent the formal init.
            "init" => {
                // C11: prefer matching on the offer_id the maker echoed back,
                // so two concurrent takes from the SAME maker each land on
                // their own pending take instead of cross-matching (which made
                // `init_matches_offer` reject the wrong one). Pre-C11 makers
                // and direct (boardless) inits omit it; fall back to the old
                // identity match (correct whenever there is only one pending
                // take with this maker).
                let echoed_offer_id = envelope.body["offer_id"].as_str();
                let (offer_id, offer) =
                    self.match_pending_take(&envelope.from, echoed_offer_id)?
                        .context("init from a maker we have no pending take with")?;
                let body: crate::board::OfferBody = serde_json::from_value(offer.body.clone())?;
                // The maker must honor their own advert. Compare against
                // the same chain-aware "now" the maker used.
                let chain_a: ChainRef = serde_json::from_value(envelope.body["chain_a"].clone())
                    .context("init without chain_a")?;
                let chain_b: ChainRef = serde_json::from_value(envelope.body["chain_b"].clone())
                    .context("init without chain_b")?;
                let now = self.coordination_now(&chain_a, &chain_b)?;
                crate::board::init_matches_offer(&envelope.body, &body, now)?;
                // Branch on the init protocol: v2 builds an adaptor accept.
                let is_v2 =
                    envelope.body["protocol"].as_str() == Some(crate::adaptor_swap::PROTOCOL_V2);
                let (swap_id, accept) = if is_v2 {
                    let (rec, accept) = self.adaptor_accept(envelope)?;
                    (rec.swap_id, accept)
                } else {
                    let (rec, accept) = self.accept(envelope)?;
                    (rec.swap_id, accept)
                };
                self.store.remove_pending_take(&offer_id)?;
                self.relay_send_all(&envelope.from, &accept)?;
                event(&swap_id, "init->accept", format!("offer {offer_id}"))
            }
            // A maker telling us our take was rejected, before any swap
            // record exists: clean up the pending take so the user is not
            // left waiting on a dead handshake.
            "abort" if self.store.get(&envelope.swap_id).is_err() => {
                let pending: Vec<_> = self
                    .store
                    .pending_takes()?
                    .into_iter()
                    .filter(|(offer_id, offer_json)| {
                        *offer_id == envelope.swap_id
                            && serde_json::from_str::<Envelope>(offer_json)
                                .map(|offer| offer.from == envelope.from)
                                .unwrap_or(false)
                    })
                    .collect();
                if pending.is_empty() {
                    return Ok(None); // junk abort for nothing we know
                }
                for (offer_id, _) in pending {
                    self.store.remove_pending_take(&offer_id)?;
                }
                let reason = envelope.body["reason"]
                    .as_str()
                    .unwrap_or("rejected")
                    .to_string();
                event(&envelope.swap_id, "take-failed", reason)
            }
            // v2 (pact-htlc-v2) handshake messages route to the adaptor
            // autopilot; the swap_id lives in the adaptor_swaps table.
            "funding_ready" | "nonces" | "partial_sigs" => {
                let rec = self.recv_adaptor(envelope)?;
                let counterparty = rec
                    .counterparty_identity
                    .clone()
                    .context("no counterparty pinned")?;
                self.drive_adaptor_relay(envelope.msg_type.as_str(), &rec, &counterparty)
            }
            // Protocol messages: apply, then keep the ball rolling. `accept`
            // is shared between v1 and v2 (disambiguated by which swap table
            // holds the swap_id).
            "accept" | "funded" | "redeemed" | "abort" => {
                if self.store.get_adaptor(&envelope.swap_id).is_ok() {
                    if envelope.msg_type == "abort" {
                        return event(&envelope.swap_id, "recv", "abort".into());
                        // advisory; timelocks protect
                    }
                    let rec = self.recv_adaptor(envelope)?;
                    let counterparty = rec
                        .counterparty_identity
                        .clone()
                        .context("no counterparty pinned")?;
                    return self.drive_adaptor_relay(
                        envelope.msg_type.as_str(),
                        &rec,
                        &counterparty,
                    );
                }
                let record = self.recv(envelope)?;
                let counterparty = record
                    .counterparty_identity
                    .clone()
                    .context("no counterparty pinned")?;
                let should_fund = self.auto_fund
                    && matches!(
                        (record.role, record.state),
                        (Role::Initiator, State::Accepted) | (Role::Participant, State::FundedA)
                    );
                if should_fund {
                    let (funded_record, funded_env) = self.fund(&record.swap_id)?;
                    self.relay_send_all(&counterparty, &funded_env)?;
                    return event(
                        &funded_record.swap_id,
                        "auto-fund",
                        format!("after {}", envelope.msg_type),
                    );
                }
                event(&record.swap_id, "recv", envelope.msg_type.clone())
            }
            other => bail!("unexpected relay message type {other:?}"),
        }
    }

    /// Core-wallet view for the wallet tab. These pass through to the
    /// primary (wallet-qualified Core RPC) backend — the user's own
    /// node wallet, NOT the hot pactd seed. A pactd-seed light wallet
    /// (for Electrum-only users) is future bdk work.
    pub fn wallet_balance(&self, network: Network, coin_id: &str) -> Result<u64> {
        self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?
        .wallet_balance()
    }

    pub fn wallet_address(&self, network: Network, coin_id: &str) -> Result<String> {
        self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?
        .wallet_new_address()
    }

    pub fn wallet_send(
        &self,
        network: Network,
        coin_id: &str,
        address: &str,
        amount_sat: u64,
    ) -> Result<String> {
        let backend = self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?;
        // The address must belong to this chain — catches pasting a BTC
        // address into the POCX send form before money moves.
        backend.params().parse_address(address)?;
        backend.wallet_send(address, amount_sat)
    }

    /// Live fee rate (sat/vB) for a configured coin, or the same conservative
    /// fallback the backends use when a coin is unconfigured/unreachable. The
    /// `bool` is `true` when the rate is the fallback (the UI flags it as a
    /// guess) rather than a live estimate. Never errors — a fee *preview* must
    /// not fail just because one node is down.
    fn fee_rate_or_fallback(&self, network: Network, coin_id: &str) -> (u64, bool) {
        // Mirrors the per-backend fallback (chain.rs FALLBACK_SAT_PER_VB).
        const FALLBACK_SAT_PER_VB: u64 = 10;
        let chain = ChainRef {
            coin_id: coin_id.to_string(),
            network,
        };
        match self.backend(&chain).and_then(|b| b.fee_rate_sat_per_vb()) {
            Ok(rate) => (rate, false),
            Err(_) => (FALLBACK_SAT_PER_VB, true),
        }
    }

    /// Fee preview for a prospective swap (C3 / `estimateswapfees`). Exposes
    /// the same numbers the engine already uses to size HTLC spends — it does
    /// NOT build or broadcast anything.
    ///
    /// Legs are determined by the give/get sides, NOT by `role`: whoever you
    /// are, you fund the coin you *give* (the unhappy-path `refund` is the
    /// alternative to that funding being swept) and you `redeem` the coin you
    /// *get*. So `give_coin`/`get_coin` are from THIS user's perspective and
    /// the returned legs are always the ones this user pays. `role`/`protocol`
    /// are accepted for forward-compat (adaptor swaps will have other legs) but
    /// do not change the HTLC leg set today; documented assumption.
    ///
    /// Corkboard charges nothing, so `platform_fee_sat` is hard-wired 0.
    pub fn estimate_swap_fees(
        &self,
        network: Network,
        give_coin: &str,
        get_coin: &str,
    ) -> Result<Value> {
        ensure!(
            give_coin != get_coin,
            "give and get must be different coins"
        );
        // Validate both coins are in the registry (network-appropriate) so the
        // preview rejects nonsense pairs the same way `offer` would.
        chain_params(&ChainRef {
            coin_id: give_coin.to_string(),
            network,
        })?;
        chain_params(&ChainRef {
            coin_id: get_coin.to_string(),
            network,
        })?;

        let (give_rate, give_fallback) = self.fee_rate_or_fallback(network, give_coin);
        let (get_rate, get_fallback) = self.fee_rate_or_fallback(network, get_coin);

        let leg = |name: &str, vbytes: u64, rate: u64| serde_json::json!({ "name": name, "vbytes": vbytes, "fee_sat": spend_fee_sat(rate, vbytes) });

        Ok(serde_json::json!({
            // ALWAYS 0 — the Corkboard is a noticeboard, not an exchange: no
            // matching, no execution, no fees. This field reinforces that.
            "platform_fee_sat": 0,
            "give": {
                "coin_id": give_coin,
                "fee_rate_sat_per_vb": give_rate,
                "fee_rate_is_fallback": give_fallback,
                "legs": [
                    leg("fund", FUND_TX_VSIZE, give_rate),
                    // Unhappy-path alternative to redeem-on-the-other-chain.
                    leg("refund", REFUND_TX_VSIZE, give_rate),
                ],
            },
            "get": {
                "coin_id": get_coin,
                "fee_rate_sat_per_vb": get_rate,
                "fee_rate_is_fallback": get_fallback,
                "legs": [
                    leg("redeem", REDEEM_TX_VSIZE, get_rate),
                ],
            },
        }))
    }

    /// Abort a swap before any funding: marks it aborted locally and
    /// tells the counterparty (advisory). Refused once our HTLC is
    /// funded — from then on, refund is the only way out (spec §8.1).
    pub fn abort(&self, swap: &str, reason: &str) -> Result<SwapRecord> {
        let mut rec = self.store.get(swap)?;
        let our_leg_funded = match rec.role {
            Role::Initiator => rec.htlc_a_txid.is_some(),
            Role::Participant => rec.htlc_b_txid.is_some(),
        };
        ensure!(
            !our_leg_funded,
            "cannot abort: our HTLC is funded — use refund after the timelock instead"
        );
        rec.state = State::Aborted;
        self.store.put(&rec)?;
        if let Some(counterparty) = &rec.counterparty_identity {
            if self.board_url.is_some() {
                let abort = self.signed_envelope(
                    "abort",
                    &rec.swap_id,
                    serde_json::json!({ "reason": reason }),
                )?;
                let _ = self.relay_send_all(counterparty, &abort);
            }
        }
        Ok(rec)
    }
}

/// The scriptPubKey our final spend pays (output 0 of the stored tx) —
/// the script hint Electrum backends need to locate the transaction.
fn spend_spk(rec: &SwapRecord) -> Option<bitcoin::ScriptBuf> {
    let bytes = hex::decode(rec.final_tx_hex.as_deref()?).ok()?;
    let tx: bitcoin::Transaction = bitcoin::consensus::encode::deserialize(&bytes).ok()?;
    Some(tx.output.first()?.script_pubkey.clone())
}

/// One scheduler action (or error) on one swap.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TickEvent {
    pub swap_id: String,
    pub action: String,
    pub detail: String,
}

/// A locally-stored private (off-market) offer, for the maker's
/// "My private offers" list. Mirrors the board offer fields the UI cards show.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PrivateOfferInfo {
    pub offer_id: String,
    pub give_asset: String,
    pub give_amount: u64,
    pub get_asset: String,
    pub get_amount: u64,
    pub t1_secs: u32,
    pub t2_secs: u32,
    /// Unix creation time (seconds), from the signed offer body.
    pub created: u64,
    /// Unix expiry (created + ttl); 0 when the offer carries no expiry.
    pub expiry: u64,
    /// Whether the offer's ttl has already lapsed (slip no longer takeable).
    pub expired: bool,
}

/// One outstanding take awaiting the maker's init (post-`boardtake`, before any
/// swap record exists). Surfaced so the UI can show an "initiating" pre-swap
/// immediately. `offer_id` equals the eventual swap's `swap_id`, so the UI can
/// dedupe the pre-swap against the real record once it lands.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PendingTakeInfo {
    pub offer_id: String,
    /// Maker identity (the offer's signer).
    pub from: String,
    /// The signed offer body (give/get assets + amounts, timelocks, protocol).
    pub body: Value,
    /// Unix time (seconds) the take was recorded — drives the take-timeout.
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(tag: &str, passphrase: Option<&str>) -> (Engine, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("libswap-engine-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::init(&dir, passphrase).unwrap();
        (
            Engine::open(&dir, passphrase, BTreeMap::new()).unwrap(),
            dir,
        )
    }

    fn offer_on(engine: &Engine, network: Network, t1: u32, t2: u32) -> Result<()> {
        engine
            .offer(
                network,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                t1,
                t2,
                None,
                None,
            )
            .map(|_| ())
    }

    #[test]
    fn board_offers_default_to_htlc() {
        // The suite defaults to v1 (HTLC) whenever the pair supports it — every
        // network, including Taproot↔Taproot pairs. v2 is opt-in, not the default.
        assert_eq!(
            board_offer_protocol("btcx", "btc", Network::Regtest),
            "pact-htlc-v1"
        );
        assert_eq!(
            board_offer_protocol("btcx", "btc", Network::Testnet),
            "pact-htlc-v1"
        );
        assert_eq!(
            board_offer_protocol("btcx", "btc", Network::Mainnet),
            "pact-htlc-v1"
        );
        assert_eq!(
            board_offer_protocol("btcx", "doge", Network::Regtest),
            "pact-htlc-v1"
        );

        // …but opting into v2 is still allowed for a Taproot pair off-mainnet,
        // and rejected on mainnet (audit gate) — so the default flip didn't break
        // the v2 path. (resolve_offer_protocol relies on this allowance.)
        assert!(adaptor_offer_allowed("btcx", "btc", Network::Regtest));
        assert!(adaptor_offer_allowed("btcx", "btc", Network::Testnet));
        assert!(!adaptor_offer_allowed("btcx", "btc", Network::Mainnet));
        assert!(!adaptor_offer_allowed("btcx", "doge", Network::Regtest));
    }

    #[test]
    fn adaptor_handshake_v2_routes_and_agrees() {
        use crate::adaptor_swap::AdaptorSwapParams;
        use crate::params::POCX_REGTEST;
        use bitcoin::secp256k1::{PublicKey, Secp256k1};
        use bitcoin::XOnlyPublicKey;

        let (alice, ad) = engine_with("v2-alice", None);
        let (bob, bd) = engine_with("v2-bob", None);
        let now = local_now() as u32;
        let (t1, t2) = (now + 40_000, now + 20_000);

        let (_arec, init) = alice
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2,
            )
            .unwrap();
        let ib: crate::messages::InitV2Body = serde_json::from_value(init.body.clone()).unwrap();
        assert_eq!(init.msg_type, "init");
        assert_eq!(ib.protocol, "pact-htlc-v2");

        let (_brec, accept) = bob.adaptor_accept(&init).unwrap();
        let ab: crate::messages::AcceptV2Body =
            serde_json::from_value(accept.body.clone()).unwrap();

        // Both sides reconstruct identical legs from the exchanged keys.
        let secp = Secp256k1::new();
        let params = AdaptorSwapParams {
            amount_a: ib.amount_a,
            amount_b: ib.amount_b,
            t1: ib.t1,
            t2: ib.t2,
            alice_swap_a: ib.alice_swap_a.parse::<PublicKey>().unwrap(),
            alice_swap_b: ib.alice_swap_b.parse::<PublicKey>().unwrap(),
            bob_swap_a: ab.bob_swap_a.parse::<PublicKey>().unwrap(),
            bob_swap_b: ab.bob_swap_b.parse::<PublicKey>().unwrap(),
            alice_refund_a: ib.alice_refund_a.parse::<XOnlyPublicKey>().unwrap(),
            bob_refund_b: ab.bob_refund_b.parse::<XOnlyPublicKey>().unwrap(),
            adaptor_point: ib.adaptor_point.parse::<PublicKey>().unwrap(),
        };
        params.validate_structure().unwrap();
        assert!(params
            .leg_a(&secp)
            .unwrap()
            .address(&secp, &POCX_REGTEST)
            .unwrap()
            .starts_with("rpocx1p"));

        // Protocol gate: a v1 `offer` init must be rejected by adaptor_accept.
        let (_rec, v1_init) = alice
            .offer(
                Network::Regtest,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                t1,
                t2,
                None,
                None,
            )
            .unwrap();
        assert!(bob.adaptor_accept(&v1_init).is_err());

        // Mainnet is gated.
        assert!(alice
            .adaptor_init(
                Network::Mainnet,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2
            )
            .is_err());

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    #[test]
    fn per_coin_confirmation_depth_overrides_default() {
        let (mut engine, dir) = engine_with("confs", None);
        let btc = ChainRef {
            coin_id: "btc".into(),
            network: Network::Regtest,
        };
        // No override → the network/spacing default (regtest = 1).
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 1);
        // An explicit per-coin depth wins.
        engine.coin_confirmations.insert("btc".into(), 4);
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 4);
        // The view reports (effective, default) for the setup UI.
        assert_eq!(
            engine
                .coin_confirmations_view(Network::Regtest, "btc")
                .unwrap(),
            (4, 1)
        );
        // A bogus 0 is clamped up to a safe floor of 1 (never "act on 0 confs").
        engine.coin_confirmations.insert("btc".into(), 0);
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn adaptor_records_carry_per_coin_confirmation_depth() {
        // Depth is local safety policy: each party sets N_a/N_b from its OWN
        // coin config, so the two records can differ and need no wire exchange.
        let (mut alice, ad) = engine_with("v2-confs-alice", None);
        let (mut bob, bd) = engine_with("v2-confs-bob", None);
        alice.coin_confirmations.insert("btc".into(), 5);
        bob.coin_confirmations.insert("btcx".into(), 7);
        let now = local_now() as u32;
        let (t1, t2) = (now + 40_000, now + 20_000);
        let (arec, init) = alice
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2,
            )
            .unwrap();
        // chain_a = pocx (regtest default 1), chain_b = btc (Alice's override 5).
        assert_eq!((arec.n_a, arec.n_b), (1, 5));
        let (brec, _accept) = bob.adaptor_accept(&init).unwrap();
        // Bob resolves from his config: pocx override 7, btc default 1.
        assert_eq!((brec.n_a, brec.n_b), (7, 1));

        // Migration: a pre-depth record (no n_a/n_b/final_tx_*_hex) still loads,
        // defaulting to 0 / None (the old no-extra-gate behaviour).
        let mut v = serde_json::to_value(&arec).unwrap();
        let obj = v.as_object_mut().unwrap();
        for k in ["n_a", "n_b", "final_tx_a_hex", "final_tx_b_hex"] {
            obj.remove(k);
        }
        let back: crate::store::AdaptorSwapRecord = serde_json::from_value(v).unwrap();
        assert_eq!((back.n_a, back.n_b), (0, 0));
        assert!(back.final_tx_a_hex.is_none() && back.final_tx_b_hex.is_none());

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    /// Full v2 handshake LIFECYCLE through persistence, two engines in-process
    /// (no chain backend): init -> accept -> funding_ready (simulated outpoints)
    /// -> nonces -> partial_sigs -> assemble, reaching `Signed` with verified
    /// adaptor signatures on both legs.
    #[test]
    fn adaptor_lifecycle_handshake_to_signed() {
        let (alice, ad) = engine_with("v2-lc-alice", None);
        let (bob, bd) = engine_with("v2-lc-bob", None);
        let now = local_now() as u32;
        let (t1, t2) = (now + 40_000, now + 20_000);

        // init / accept.
        let (arec, init) = alice
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2,
            )
            .unwrap();
        let id = arec.swap_id.clone();
        let (_brec, accept) = bob.adaptor_accept(&init).unwrap();
        alice.recv_adaptor(&accept).unwrap();
        assert_eq!(
            alice.store.get_adaptor(&id).unwrap().state,
            AdaptorState::Accepted
        );

        // funding_ready: Alice funds A (pocx), Bob funds B (btc) — simulated
        // outpoints (the chain-free recorder; adaptor_fund would wallet_send).
        let fa = alice
            .adaptor_funding_ready(&id, &"aa".repeat(32), 0)
            .unwrap();
        let fb = bob.adaptor_funding_ready(&id, &"bb".repeat(32), 1).unwrap();
        bob.recv_adaptor(&fa).unwrap();
        alice.recv_adaptor(&fb).unwrap();

        // nonces, then partial sigs.
        let na = alice.adaptor_nonces(&id).unwrap();
        let nb = bob.adaptor_nonces(&id).unwrap();
        bob.recv_adaptor(&na).unwrap();
        alice.recv_adaptor(&nb).unwrap();

        let pa = alice.adaptor_sign(&id).unwrap();
        let pb = bob.adaptor_sign(&id).unwrap();
        bob.recv_adaptor(&pa).unwrap();
        alice.recv_adaptor(&pb).unwrap();

        // Both assemble identical, valid adaptor signatures and reach Signed.
        let ar = alice.adaptor_assemble(&id).unwrap();
        let br = bob.adaptor_assemble(&id).unwrap();
        assert_eq!(ar.state, AdaptorState::Signed);
        assert_eq!(br.state, AdaptorState::Signed);
        assert!(ar.adaptor_sig_a.is_some() && ar.adaptor_sig_b.is_some());
        // Both parties derived the SAME aggregate adaptor signatures.
        assert_eq!(ar.adaptor_sig_a, br.adaptor_sig_a);
        assert_eq!(ar.adaptor_sig_b, br.adaptor_sig_b);

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    #[test]
    fn mainnet_is_refused() {
        let (engine, dir) = engine_with("mainnet", Some("pw"));
        let now = local_now() as u32;
        let err = offer_on(&engine, Network::Mainnet, now + 10 * 3600, now + 5 * 3600)
            .unwrap_err()
            .to_string();
        assert!(err.contains("mainnet is disabled"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn testnet_allows_unencrypted_seed() {
        // Relaxed gate: a plaintext seed on testnet now WARNS but is
        // permitted (it no longer hard-fails like it used to). A valid
        // profile offer must succeed entirely offline.
        let (engine, dir) = engine_with("testnet-plain", None);
        assert!(!engine.store.seed_is_encrypted().unwrap());
        let now = local_now() as u32;
        let (record, _) = engine
            .offer(
                Network::Testnet,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                now + 10 * 3600,
                now + 5 * 3600,
                None,
                None,
            )
            .expect("plaintext testnet offer is permitted (with a warning)");
        assert_eq!(record.chain_a.network, Network::Testnet);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn testnet_with_encrypted_seed_enforces_profile() {
        let (engine, dir) = engine_with("testnet-enc", Some("pw"));
        let now = local_now() as u32;
        // Too-short T2 violates §7.3.
        assert!(offer_on(&engine, Network::Testnet, now + 10 * 3600, now + 3600).is_err());
        // Valid profile: offer succeeds entirely offline (no RPC needed),
        // with the §7.3 confirmation defaults baked into the init body.
        let (record, _) = engine
            .offer(
                Network::Testnet,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                now + 10 * 3600,
                now + 5 * 3600,
                None,
                None,
            )
            .unwrap();
        assert_eq!((record.n_a, record.n_b), (10, 3));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn offer_rejects_unknown_coin() {
        // Exercises the coin_id -> registry path in the engine: an offer for
        // a coin that is not shipped is refused before any RPC, with a clear
        // message (the capability pair resolver cannot resolve it).
        let (engine, dir) = engine_with("unknown-coin", None);
        let err = engine
            .offer(
                Network::Regtest,
                ("doge".into(), 100),
                ("btc".into(), 100),
                1_700_000_002,
                1_700_000_001,
                None,
                None,
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("doge"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn estimate_swap_fees_shape_and_fallback() {
        // No coins configured -> each side uses the fallback rate (flagged),
        // and the preview never errors on an unreachable node.
        let (engine, dir) = engine_with("fee-preview", None);
        let v = engine
            .estimate_swap_fees(Network::Regtest, "btcx", "btc")
            .unwrap();

        // Corkboard charges nothing — this is contractually 0, always.
        assert_eq!(v["platform_fee_sat"], 0);

        let give = &v["give"];
        let get = &v["get"];
        assert_eq!(give["coin_id"], "btcx");
        assert_eq!(get["coin_id"], "btc");
        // Fallback rate (chain.rs FALLBACK_SAT_PER_VB), flagged as a guess.
        assert_eq!(give["fee_rate_sat_per_vb"], 10);
        assert_eq!(give["fee_rate_is_fallback"], true);
        assert_eq!(get["fee_rate_is_fallback"], true);

        // give = fund + refund; get = redeem. Names + non-negative fees.
        let give_legs = give["legs"].as_array().unwrap();
        let get_legs = get["legs"].as_array().unwrap();
        assert_eq!(give_legs.len(), 2);
        assert_eq!(get_legs.len(), 1);
        assert_eq!(give_legs[0]["name"], "fund");
        assert_eq!(give_legs[1]["name"], "refund");
        assert_eq!(get_legs[0]["name"], "redeem");
        for leg in give_legs.iter().chain(get_legs.iter()) {
            assert!(leg["vbytes"].as_u64().unwrap() > 0);
            assert!(leg["fee_sat"].as_u64().unwrap() >= MIN_SPEND_FEE_SAT);
        }
        // 10 sat/vB * 160 vB fund = 1600 sat (above the 1000 floor).
        assert_eq!(give_legs[0]["fee_sat"], 10 * FUND_TX_VSIZE);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn estimate_swap_fees_rejects_bad_pair() {
        let (engine, dir) = engine_with("fee-bad-pair", None);
        // Same coin both sides, and an unshipped coin, are both refused.
        assert!(engine
            .estimate_swap_fees(Network::Regtest, "btc", "btc")
            .is_err());
        assert!(engine
            .estimate_swap_fees(Network::Regtest, "doge", "btc")
            .is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn configured_coins_in_registry_order() {
        let dir = std::env::temp_dir().join(format!("libswap-coins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::init(&dir, None).unwrap();
        // Insert out of registry order; configured_coins normalizes to it.
        let mut coins = BTreeMap::new();
        coins.insert("btc".to_string(), "http://x".to_string());
        coins.insert("btcx".to_string(), "http://y".to_string());
        let engine = Engine::open(&dir, None, coins).unwrap();
        assert_eq!(engine.configured_coins(), vec!["btcx", "btc"]);

        // An offer for an unconfigured coin fails the moment a backend is
        // needed, with a message naming the coin (no panic, no RPC attempt for
        // the missing one). Here ltc is not even shipped, so it's caught earlier.
        let only_pocx = {
            let mut c = BTreeMap::new();
            c.insert("btcx".to_string(), "http://y".to_string());
            Engine::open(&dir, None, c).unwrap()
        };
        let err = only_pocx
            .wallet_balance(Network::Regtest, "btc")
            .unwrap_err()
            .to_string();
        assert!(err.contains("btc") && err.contains("backend"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn regtest_skips_profile_minimums() {
        let (engine, dir) = engine_with("regtest", None);
        // Tiny regtest timelocks are fine (spec §7.5); structure still holds.
        offer_on(&engine, Network::Regtest, 1_700_000_002, 1_700_000_001).unwrap();
        assert!(offer_on(&engine, Network::Regtest, 1_700_000_001, 1_700_000_001).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A minimal stored pending-take envelope: the matcher only deserializes
    /// it and reads `from`, so the body/sig can be empty here.
    fn pending_offer_from(maker: &str) -> String {
        serde_json::to_string(&Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: "x".into(),
            from: maker.into(),
            body: serde_json::json!({}),
            sig: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn c11_init_matches_the_right_pending_take_for_same_maker() {
        // Two concurrent takes with the SAME maker. The init echoes one
        // offer_id; the matcher must pick THAT pending take, not "first with
        // this identity" (the pre-C11 bug that cross-matched).
        let (engine, dir) = engine_with("c11-same-maker", None);
        let maker = "maker-identity-hex";
        engine
            .store
            .put_pending_take("offer-A", &pending_offer_from(maker), 1)
            .unwrap();
        engine
            .store
            .put_pending_take("offer-B", &pending_offer_from(maker), 2)
            .unwrap();

        let (id, offer) = engine
            .match_pending_take(maker, Some("offer-B"))
            .unwrap()
            .expect("offer-B matches");
        assert_eq!(id, "offer-B");
        assert_eq!(offer.from, maker);

        let (id_a, _) = engine
            .match_pending_take(maker, Some("offer-A"))
            .unwrap()
            .expect("offer-A matches");
        assert_eq!(id_a, "offer-A");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn c11_falls_back_to_identity_and_guards_against_wrong_maker() {
        let (engine, dir) = engine_with("c11-fallback", None);
        engine
            .store
            .put_pending_take("offer-A", &pending_offer_from("bob"), 1)
            .unwrap();

        // No echoed offer_id (pre-C11 / direct init): identity match still works.
        let (id, _) = engine
            .match_pending_take("bob", None)
            .unwrap()
            .expect("identity match");
        assert_eq!(id, "offer-A");

        // A stray/forged offer_id from a DIFFERENT maker never binds to bob's
        // take — the identity guard rejects it.
        assert!(engine
            .match_pending_take("carol", Some("offer-A"))
            .unwrap()
            .is_none());
        // Unknown maker with no echo: no match.
        assert!(engine.match_pending_take("carol", None).unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn c8_prunes_stale_pending_takes_only() {
        // C8: a pending take older than the timeout is abandoned (with a
        // `take-timeout` event); a fresh one is left alone.
        let (engine, dir) = engine_with("c8-prune", None);
        let now = local_now();
        let stale = now.saturating_sub(PRE_FUNDING_TIMEOUT_SECS + 60);
        let fresh = now; // just taken
        engine
            .store
            .put_pending_take("stale", &pending_offer_from("m"), stale)
            .unwrap();
        engine
            .store
            .put_pending_take("fresh", &pending_offer_from("m"), fresh)
            .unwrap();

        let mut events = Vec::new();
        engine.prune_stale_pending_takes(&mut events).unwrap();

        let remaining: Vec<_> = engine
            .store
            .pending_takes()
            .unwrap()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(remaining, vec!["fresh".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].swap_id, "stale");
        assert_eq!(events[0].action, "take-timeout");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn c8_pre_c8_pending_take_is_pruned_immediately() {
        // A row migrated from a pre-C8 db has created_at = 0, so it reads as
        // ancient and is cleaned up on the first tick after upgrade.
        let (engine, dir) = engine_with("c8-migrated", None);
        engine
            .store
            .put_pending_take("old", &pending_offer_from("m"), 0)
            .unwrap();
        let mut events = Vec::new();
        engine.prune_stale_pending_takes(&mut events).unwrap();
        assert!(engine.store.pending_takes().unwrap().is_empty());
        assert_eq!(events.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn private_offer_make_list_cancel() {
        // make_private_offer returns a valid slip, stores the offer locally so
        // it lists, and cancel removes it + arms the revoke guard the `take`
        // handler reads — all without any board.
        let (engine, dir) = engine_with("private-make", None);

        let slip = engine
            .make_private_offer(
                Network::Regtest,
                ("btcx".into(), 100),
                ("btc".into(), 50),
                1_700_000_002,
                1_700_000_001,
                None,
                None,
            )
            .unwrap();
        assert!(slip.starts_with("pactoffer1:"), "{slip}");
        // The slip decodes to our own signed offer.
        let offer = pact_proto::slip::decode_slip(&slip).unwrap();
        assert_eq!(offer.from, engine.identity().unwrap());

        let listed = engine.list_private_offers().unwrap();
        assert_eq!(listed.len(), 1);
        let info = &listed[0];
        assert_eq!(info.offer_id, offer.swap_id);
        assert_eq!((info.give_asset.as_str(), info.give_amount), ("btcx", 100));
        assert_eq!((info.get_asset.as_str(), info.get_amount), ("btc", 50));
        assert!(!info.expired);

        // Cancel: gone from the list, and the revoke marker is set so a late
        // take that still holds the slip is rejected by the take handler.
        engine.cancel_private_offer(&offer.swap_id).unwrap();
        assert!(engine.list_private_offers().unwrap().is_empty());
        assert!(engine
            .store
            .meta_get(&format!("offer_revoked:{}", offer.swap_id))
            .unwrap()
            .is_some());
        // Cancelling something that does not exist errors.
        assert!(engine.cancel_private_offer("deadbeef").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn private_offer_rejects_bad_pair_and_timelocks() {
        let (engine, dir) = engine_with("private-bad", None);
        // T2 must be < T1.
        assert!(engine
            .make_private_offer(
                Network::Regtest,
                ("btcx".into(), 1),
                ("btc".into(), 1),
                5,
                5,
                None,
                None
            )
            .is_err());
        // Same coin both sides.
        assert!(engine
            .make_private_offer(
                Network::Regtest,
                ("btc".into(), 1),
                ("btc".into(), 1),
                2,
                1,
                None,
                None
            )
            .is_err());
        // Unknown coin.
        assert!(engine
            .make_private_offer(
                Network::Regtest,
                ("doge".into(), 1),
                ("btc".into(), 1),
                2,
                1,
                None,
                None
            )
            .is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn take_offer_slip_rejects_own_and_bad_slips() {
        let (engine, dir) = engine_with("private-take", None);
        // A garbage slip is rejected by the codec before anything else.
        assert!(engine.take_offer_slip("not-a-slip").is_err());
        // Our own private offer cannot be self-taken (mirrors take_board_offer).
        let slip = engine
            .make_private_offer(
                Network::Regtest,
                ("btcx".into(), 100),
                ("btc".into(), 50),
                1_700_000_002,
                1_700_000_001,
                None,
                None,
            )
            .unwrap();
        let err = engine.take_offer_slip(&slip).unwrap_err().to_string();
        assert!(err.contains("our own"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
