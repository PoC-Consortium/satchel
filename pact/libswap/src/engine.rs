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
use bitcoin::secp256k1::{PublicKey, SecretKey};
use bitcoin::{OutPoint, ScriptBuf};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use crate::adaptor_swap::AdaptorState;
use crate::chain::{ChainBackend, MultiBackend, SendFee};
use crate::htlc::extract_preimage;
use crate::keys::{hash_preimage, swap_id, PactSeed};
use crate::messages::{self, AbortBody, AcceptBody, ChainRef, Envelope, FundedBody, InitBody};
use crate::params::{ChainParams, Network};
use crate::registry;
use crate::store::{AdaptorSwapRecord, Store, SwapRecord};
use crate::swap::{
    build_redeem_tx, build_refund_tx, spend_fee_sat, Role, State, SwapParams, DUST_LIMIT_SAT,
    FUND_TX_VSIZE, HTLC_SPEND_SEQUENCE, REDEEM_TX_VSIZE, REFUND_TX_VSIZE,
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
    /// the local `nostr_*` buffers (spec/protocol.md §8.8).
    pub nostr_relays: Option<String>,
    /// Fund our HTLC leg automatically during board-driven swaps. OFF by
    /// default: funding commits real money, and an auto-funding maker can
    /// be griefed into locking funds until T1 by takers who never fund.
    /// Per-trade caps are the roadmap mitigation.
    pub auto_fund: bool,
    /// Unified fee-bump policy (see [`crate::fee_policy`]). Loaded from this
    /// merchant's store at [`Engine::open`] (or the default if never set), and
    /// changed at runtime via [`Engine::set_fee_bump`] (pactd `setfeepolicy`).
    /// Defaults reproduce the historical hardcoded consts, so behaviour is
    /// unchanged until an operator overrides it.
    pub fee_bump: crate::fee_policy::FeeBumpPolicy,
    /// Live, in-memory progress snapshot per active swap (observability only —
    /// never ledger truth). Rebuilt every [`Engine::tick`] from the data the
    /// tick already gathers, and served verbatim by the `swapprogress` RPC so
    /// the UI shows confirmation depth + the latest scheduler action without a
    /// node call per poll. Ephemeral: empty after a restart until the next tick
    /// repopulates it.
    progress: Mutex<HashMap<String, SwapProgress>>,
    /// Per-coin nodeless (bdk) wallets, opened lazily by [`Engine::backend`]
    /// for coins configured with Electrum URLs only
    /// (docs/NODELESS_WALLET.md D2/D5). Stateful — sync position, revealed
    /// indexes, sqlite store — so it lives here rather than being rebuilt
    /// per backend construction.
    wallet_manager: crate::wallet_bdk::WalletManager,
    /// Long-lived Electrum connections, one per configured server (issue
    /// #87) — shared by every engine call and the per-coin sync workers
    /// instead of a fresh TCP+TLS handshake per call. Lazy + self-healing,
    /// so pooling never pins a dead socket.
    electrum_pool: crate::chain::ElectrumPool,
    /// Sticky per-coin choice of which configured Electrum servers are the
    /// ACTIVE views (issue #98). Everything not selected is cold standby:
    /// no pooled connection (the pool prunes it on the next `get`), no
    /// reads, promoted only when an active slot frees up — so a 10+-server
    /// list adds backup depth, never latency.
    server_set: crate::server_health::ServerSet,
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

/// Ensure a pair can run a v2 adaptor swap: both legs Taproot-capable
/// (spec v2; see spec/protocol-v2.md).
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
            "{}<->{}: v2 adaptor swaps are not available on {}",
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

/// Marker context for handshake errors that are DETERMINISTIC given the same
/// envelope — validation and parse failures that can never succeed on retry.
/// The relay loop gives up immediately on these (one clear event instead of
/// ten silent `relay-retry`s), and the taker's init path turns them into a
/// reasoned abort to the maker. Attached via [`permanent_err`], detected via
/// [`is_permanent`]. Born of the 2026-07-08 mainnet incident, where a
/// config-rejected init retried 10× invisibly and then reported "no init
/// within 900s" — an init that had arrived ten times.
#[derive(Debug)]
struct PermanentError;

impl std::fmt::Display for PermanentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("permanent handshake failure (will not retry)")
    }
}

/// Tag an error as [`PermanentError`] (deterministic — retrying is useless).
fn permanent_err(e: anyhow::Error) -> anyhow::Error {
    e.context(PermanentError)
}

/// Whether `err` carries the [`PermanentError`] marker anywhere in its chain.
fn is_permanent(err: &anyhow::Error) -> bool {
    err.downcast_ref::<PermanentError>().is_some()
}

/// `ensure!` whose failure is tagged [`PermanentError`] — for deterministic
/// validation of a received envelope (same input → same failure, retrying is
/// useless). Transient checks (chain/backend/seed access) keep plain
/// `ensure!`/`?` so the relay loop's retry still covers them.
macro_rules! ensure_permanent {
    ($cond:expr, $($arg:tt)*) => {
        if !($cond) {
            return Err(permanent_err(anyhow::anyhow!($($arg)*)));
        }
    };
}

/// Allowed confirmation-depth band for `chain` — spec §7.3 as amended for the
/// rc12 recut: floor **2** on mainnet/testnet (a single stale block is routine
/// on every chain, depth-2 reorgs are rare even at 2-min spacing; 0/1-conf
/// trading stays disallowed), cap **the chain default** (the trustless
/// standard is the maximum — anything deeper only stalls the swap toward its
/// timelocks). Regtest is exempt (§7.5): floor 1, uncapped, so the e2e suite
/// can drive arbitrary depths.
pub fn confirmation_bounds(params: &ChainParams) -> (u32, u32) {
    match params.network {
        Network::Regtest => (1, u32::MAX),
        _ => (2, default_confirmations(params)),
    }
}

/// Ensure a confirmation depth sits inside [`confirmation_bounds`] for its
/// chain. Used both for OUR values (backstop — [`Engine::confirmations_for`]
/// already clamps config into the band) and for the counterparty's advisory
/// values from init/accept (an out-of-band advertised depth is a foreseeable
/// liveness stall, rejected up-front).
fn ensure_confs_in_bounds(chain: &ChainRef, n: u32, leg: &str) -> Result<()> {
    let (floor, cap) = confirmation_bounds(chain_params(chain)?);
    ensure!(
        n >= floor && n <= cap,
        "spec §7.3: {leg} for {} must be within {floor}..={cap} (got {n}) — the chain default is the maximum",
        chain.coin_id
    );
    Ok(())
}

/// Spec §7.3 network-profile minimums (regtest is exempt, §7.5). Durations
/// are checked against the local clock at offer/accept time; confirmation
/// depths against the per-chain [`confirmation_bounds`]. Each side validates
/// only its OWN depths here — the counterparty's advisory copies are checked
/// separately where they arrive.
fn validate_profile(
    chain_a: &ChainRef,
    chain_b: &ChainRef,
    t1: u32,
    t2: u32,
    n_a: u32,
    n_b: u32,
) -> Result<()> {
    if chain_a.network == Network::Regtest {
        return Ok(());
    }
    let now = local_now();
    // Guard the ordering BEFORE the `t1 - t2` subtraction below: without it a
    // caller with t2 >= t1 (e.g. a misconfigured offer builder) underflows the
    // u32 in release builds and wraps to a huge value that passes the ≥ 4h check,
    // silently accepting an inverted (unsafe) timelock ordering.
    ensure!(t2 < t1, "spec §7.1: T2 must be < T1");
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
    ensure_confs_in_bounds(chain_a, n_a, "N_A")?;
    ensure_confs_in_bounds(chain_b, n_b, "N_B")?;
    Ok(())
}

/// Spec §7.4 action-deadline margins — the lead-time before a timelock by
/// which a party must have *acted*, so the action confirms before the
/// counterparty's window opens. Returned as `(fund, reveal, redeem_a)`:
/// - `fund`: Bob must broadcast his chain-B funding no later than `T2 − 3h`.
/// - `reveal`: Alice must broadcast her chain-B redeem (revealing `s`) no
///   later than `T2 − 2h`; a redeem lingering past `T2` reveals `s` while Bob
///   can already refund chain B, letting him take *both* legs.
/// - `redeem_a`: Bob must broadcast his chain-A redeem before `T1 − 1h`.
///
/// These are the normative flat numbers from the spec (calibrated for the
/// mainnet/testnet profile, accounting for BIP113 MTP lag + ~2h adversarial
/// skew, §7.2). Regtest is exempt (§7.5): the e2e suite drives swaps to
/// completion in seconds, so all margins are zero and behaviour is unchanged.
pub(crate) fn action_margins(network: Network) -> (u64, u64, u64) {
    if network == Network::Regtest {
        (0, 0, 0)
    } else {
        (3 * 3600, 2 * 3600, 3600)
    }
}

/// The conservative "now" for a §7.4 action-deadline gate against a timelock
/// on one chain: the later of our NTP-synced wall clock and that chain's MTP.
/// MTP *lags* wall-clock (§7.2), so for a "stop acting in time" deadline,
/// trusting MTP alone is unsafe (it would let us act too late); taking the max
/// means neither a lagging chain nor a slow local clock can push us past the
/// deadline. Mirrors `coordination_now`'s philosophy, applied per-leg.
///
/// On regtest we keep the historical pure-MTP behaviour: the e2e chains start
/// with an MTP at the 2011 genesis time, and the suite relies on that lag, so
/// folding in the (2026) wall clock there would spuriously trip the gates.
pub(crate) fn deadline_clock(network: Network, local: u64, chain_mtp: u64) -> u64 {
    if network == Network::Regtest {
        chain_mtp
    } else {
        local.max(chain_mtp)
    }
}

/// Deadline-aware market estimate parameters for a **redeem** bump (spec §7.4
/// "MUST fee-bump aggressively"; issues #47/#48). Given the time remaining until
/// the redeem's confirm-by deadline, pick the estimator's `(conf_target,
/// conservative)`: keep the cheap "normal" tier (6, economical) when there is
/// plenty of time, and escalate to tighter/robuster targets as the timelock
/// nears — because a redeem that fails to confirm before its deadline loses the
/// leg, so the going fast rate is value-justified insurance, not overpay (the
/// value cap in `claim_feerate` still bounds the absolute fee). `conf_target = 1`
/// is notoriously flaky in Core, so it is reserved for the final stretch; the
/// middle bands use 3/2 with the conservative estimate mode for a robust bump.
pub(crate) fn redeem_conf_target(remaining_secs: u64) -> (u16, bool) {
    match remaining_secs {
        r if r > 6 * 3600 => (6, false), // plenty of time: today's cheap baseline
        r if r > 2 * 3600 => (3, true),  // closer: robust conservative estimate
        r if r > 3600 => (2, true),      // closer still
        _ => (1, true),                  // final stretch: fastest, worth almost any fee
    }
}

/// Is an action whose lead margin is `margin` still safe to broadcast at
/// conservative time `clock`, given absolute deadline `deadline`? True iff
/// `clock + margin < deadline`. Pure (no clock/backend) so the §7.4 timing
/// logic is unit-testable without a node.
pub(crate) fn action_safe(clock: u64, margin: u64, deadline: u32) -> bool {
    clock + margin < u64::from(deadline)
}

/// Spec §7.3/§7.4 sanity for an offer's *relative* timelock offsets (seconds
/// from the future take time), enforced when advertising/creating an offer so
/// we never publish a swap that the taker's accept-time `validate_profile`
/// would reject — or one that leaves no room for the §7.4 action margins.
/// Regtest is exempt (§7.5). The UI's Short/Medium/Long presets all satisfy
/// this; this guards the CLI and any future caller.
fn validate_offer_offsets(network: Network, t1_secs: u32, t2_secs: u32) -> Result<()> {
    ensure!(t2_secs < t1_secs, "spec §7.1: T2 must be < T1");
    if network == Network::Regtest {
        return Ok(());
    }
    let (fund_margin, _, _) = action_margins(network);
    ensure!(
        u64::from(t2_secs) >= fund_margin,
        "T2 must be ≥ {}h out so Bob can fund before the §7.4 deadline (got {}h)",
        fund_margin / 3600,
        t2_secs / 3600
    );
    ensure!(
        t1_secs - t2_secs >= 4 * 3600,
        "spec §7.3: T1 − T2 must be ≥ 4h (got {}h)",
        (t1_secs - t2_secs) / 3600
    );
    Ok(())
}

/// Min-relay floor (sat/vB) for the cooperative-redeem feerate negotiated into
/// the init. The committed redeem can't be RBF'd but IS CPFP-bumpable, so we
/// commit at market with no over-provision and let this floor (and the nurse)
/// catch the bottom. On regtest the node can't estimate, so the redeem lands on
/// market's fallback (≈1), i.e. this floor.
pub(crate) const MIN_REDEEM_FEERATE: u64 = 1;

// The v2 cooperative redeem is committed at live market and can't be RBF'd; the
// deadline-aware CPFP child lifts it if the market rises (no over-provision knob).

/// Upper bound on a negotiated redeem feerate (sat/vB) — the **protocol** bound
/// (spec v2 §5), distinct from the local `FeeBumpPolicy::max_feerate_sat_vb` bump
/// ceiling (which happens to equal it). Caps the initiator's committed rate AND
/// lets the participant reject an init that sets an absurd rate to grief the
/// counterparty (whose redeem fee would eat its output). Matches the estimator's
/// own clamp.
pub(crate) const MAX_REDEEM_FEERATE: u64 = 500;

/// Fallback redeem feerate for non-regtest when the initiator has no live
/// estimator to ask — conservative, but not catastrophically low.
const ADAPTOR_REDEEM_FEERATE_FALLBACK: u64 = 20;

// Funding-fee headroom for the pre-flight fundability gate
// ([`Engine::ensure_can_fund`]). The funding tx is the ONLY wallet-funded action
// in a swap — redeem/refund/bump fees all come out of the output being spent,
// never the spendable balance, but funding draws it. So above the bare swap amount
// we reserve the worst-case funding fee: the ceiling a funding bump would chase,
// `ceiling = min(reservation_mult × live feerate, max_feerate_sat_vb)`, times
// FUNDING_VSIZE_EST. The reservation multiplier is now
// `FeeBumpPolicy::funding.reservation_mult` (default 3); see crate::fee_policy.
/// Padded vsize (vB) of a typical funding tx (1–2-in, 2-out segwit) used to turn
/// the ceiling feerate into a sat reserve. Over-estimated so the reserve is never
/// short.
const FUNDING_VSIZE_EST: u64 = 250;
/// Feerate (sat/vB) assumed for the headroom when the live estimator can't be
/// reached. Also the floor of the ceiling clamp.
const FUNDING_FEERATE_FALLBACK: u64 = 20;

/// Estimated vsize of the self-funded CPFP child that bumps a stuck cooperative
/// redeem (v2+): a 1-in (wallet-owned sweep output) 1-out wallet tx. Slightly
/// over-estimated so the realised package feerate meets or beats the target.
pub(crate) const CPFP_CHILD_VSIZE: u64 = 150;

/// The child fee (sat) that lifts a stuck cooperative redeem's PACKAGE feerate
/// to `target` sat/vB, or `None` when the parent already pays at least `target`
/// on its own (nothing to bump). `parent_fee` is the redeem's committed fee;
/// the package spans both vsizes. The child pays **exactly** the top-up needed
/// to reach `target` — no minimum-fee floor: we bump to market, and the caller's
/// dust check (`child_value > DUST_LIMIT_SAT`) already rejects an output too
/// small to fund the child. The committed redeem fee can't be RBF'd, so a child
/// is the only lever — see spec/protocol-v2.md. Pure, so the CPFP fee policy is
/// unit-testable without a node.
pub(crate) fn cpfp_child_fee_kvb(
    parent_fee: u64,
    parent_vsize: u64,
    target_kvb: u64,
) -> Option<u64> {
    let parent_feerate_kvb = parent_fee.saturating_mul(1000) / parent_vsize.max(1);
    if target_kvb <= parent_feerate_kvb {
        return None; // the parent already clears the target unaided
    }
    let package_vsize = parent_vsize + CPFP_CHILD_VSIZE;
    // sat/kvB × vsize → sat, rounded UP so the realised package feerate meets
    // (never undershoots) the target at the estimator's native resolution.
    let desired_package_fee = target_kvb.saturating_mul(package_vsize).div_ceil(1000);
    Some(desired_package_fee.saturating_sub(parent_fee))
}

/// The sat/kvB rate the v1 funding RBF nurse should bump to, or `None` when the
/// market hasn't risen above what the funding already pays (no bump warranted).
/// All inputs sat/kvB (the estimator's native resolution) — so `old_feerate_kvb`
/// is the funding tx's TRUE feerate (fee×1000/vsize), never truncated to a whole
/// sat/vB. The offered rate is floored to `old + incr`: a BIP125 replacement must
/// beat the old feerate by the node's incremental relay fee (Rule 4), and the
/// old code's whole-sat/vB offer landed just below the node's fractional minimum
/// (a 1.004 sat/vB tx, offered "2", needed 2.004) and re-offered the same value
/// every tick → funding stranded for blocks. Pure, so that floor is unit-tested.
pub(crate) fn funding_bump_rate_kvb(
    old_feerate_kvb: u64,
    market_kvb: u64,
    incr_kvb: u64,
    ceiling_kvb: u64,
    reservation_mult: u64,
) -> Option<u64> {
    // Chase market, bounded by the policy ceiling AND the funds-gate reservation
    // (× old_feerate, the headroom that gate set aside).
    let target_kvb = market_kvb
        .min(ceiling_kvb)
        .min(reservation_mult.saturating_mul(old_feerate_kvb));
    if target_kvb <= old_feerate_kvb {
        return None; // market hasn't risen above what we already pay
    }
    Some(target_kvb.max(old_feerate_kvb.saturating_add(incr_kvb)))
}

/// Default confirmation requirement per chain — the fallback when the operator
/// hasn't set a per-coin depth (see [`Engine::confirmations_for`]): regtest → 1;
/// fast chains (<5-min blocks, e.g. BTCX's 2-min spacing) → 10; slower chains
/// (≥5-min blocks, e.g. Bitcoin's 10-min) → 6, the classic Bitcoin finality rule.
pub fn default_confirmations(chain: &ChainParams) -> u32 {
    match (chain.network, chain.target_spacing_secs < 300) {
        (Network::Regtest, _) => 1,
        (_, true) => 10,
        (_, false) => 6,
    }
}

/// The progress phase a maker (initiator) surfaces while in the v2 `Signed`
/// state. Pure (no chain I/O) so the two-phase display is unit-testable.
///
/// Leg B's funding txid is BUILT at accept, so it exists from the moment we
/// reach `Signed` — but the taker only BROADCASTS leg B once our leg A is
/// `n_a`-deep (`adaptor_leg_a_confirmed`). The phase must therefore be driven by
/// OBSERVATION of the output, never by the txid merely being set: until leg B
/// is seen on the network the honest wait is our own leg A burying, not the
/// taker's lock. `leg_b_seen` is `None` until the leg-B outpoint is visible;
/// the mempool counts as `Some(0)` — v1's maker flips to `their_lock` on the
/// taker's `funded` message at broadcast time, so gating v2 on the first
/// confirmation instead made it lag v1 by exactly one block. `leg_a_confs` is
/// `None` when our leg-A funding isn't recorded yet.
#[derive(Debug, PartialEq, Eq)]
enum MakerSignedPhase {
    /// Leg B is observed (mempool or deeper), burying toward `n_b` → "their
    /// lock confirming".
    TheirLockB,
    /// Leg B not observed yet; our leg A buries toward `n_a` → "your lock confirming".
    OurLockA,
    /// Our leg A is `n_a`-deep; now awaiting the taker's leg-B broadcast.
    AwaitingLock,
    /// Nothing of ours is on-chain to anchor a count to yet.
    AwaitingLockUnanchored,
}

fn maker_signed_phase(
    leg_b_seen: Option<u32>,
    leg_a_confs: Option<u32>,
    n_a: u32,
) -> MakerSignedPhase {
    if leg_b_seen.is_some() {
        MakerSignedPhase::TheirLockB
    } else {
        match leg_a_confs {
            Some(c) if c < n_a => MakerSignedPhase::OurLockA,
            Some(_) => MakerSignedPhase::AwaitingLock,
            None => MakerSignedPhase::AwaitingLockUnanchored,
        }
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

/// Swap key for a v1 record (spec §4.2): the initiator's rides its local BIP32
/// counter (`hash_h` — and so the swap id — derive from the deterministic
/// preimage at that index); the participant's is anchored to `hash_h` itself,
/// which it knows before deriving any key and which sits in both on-chain HTLC
/// scripts — so it is re-derivable from the chain alone, with no counter to
/// collide across machines sharing the seed.
fn v1_swap_key(seed: &PactSeed, rec: &SwapRecord, coin: u32) -> Result<SecretKey> {
    match rec.swap_index {
        Some(i) => seed.swap_secret_key(coin, i),
        None => seed.swap_secret_key_anchored(coin, &parse_hash(&rec.hash_h)?),
    }
}

/// The v2 participant's key anchor: the compressed adaptor point `T` — the
/// value the v2 swap id itself is derived from (spec v2 §3.3).
fn v2_anchor(rec: &AdaptorSwapRecord) -> Result<[u8; 33]> {
    Ok(parse_pubkey(&rec.adaptor_point, "adaptor point")?.serialize())
}

/// Swap (MuSig2 signer) key for a v2 record — the v2 analog of [`v1_swap_key`].
fn v2_swap_key(seed: &PactSeed, rec: &AdaptorSwapRecord, coin: u32) -> Result<SecretKey> {
    match rec.swap_index {
        Some(i) => seed.swap_secret_key(coin, i),
        None => seed.swap_secret_key_anchored(coin, &v2_anchor(rec)?),
    }
}

/// Refund (CLTV tapleaf) key for a v2 record — same counter/anchored split.
fn v2_refund_key(seed: &PactSeed, rec: &AdaptorSwapRecord, coin: u32) -> Result<SecretKey> {
    match rec.swap_index {
        Some(i) => seed.refund_secret_key(coin, i),
        None => seed.refund_secret_key_anchored(coin, &v2_anchor(rec)?),
    }
}

/// One decoded rescue snapshot (#54) — either protocol's record.
enum RescuedRecord {
    V1(Box<SwapRecord>),
    V2(Box<AdaptorSwapRecord>),
}

impl RescuedRecord {
    fn swap_id(&self) -> &str {
        match self {
            Self::V1(r) => &r.swap_id,
            Self::V2(r) => &r.swap_id,
        }
    }

    /// Terminal snapshots are never adopted — a missed tombstone is harmless.
    fn terminal(&self) -> bool {
        match self {
            Self::V1(r) => matches!(r.state, State::Completed | State::Refunded | State::Aborted),
            Self::V2(r) => matches!(
                r.state,
                AdaptorState::Completed | AdaptorState::Refunded | AdaptorState::Aborted
            ),
        }
    }
}

impl Engine {
    pub fn open(
        data_dir: &Path,
        passphrase: Option<&str>,
        coins: BTreeMap<String, String>,
    ) -> Result<Self> {
        let store = Store::open(data_dir, passphrase)?;
        // A previously CLI/RPC-set policy survives restart; else the default.
        let fee_bump = store.fee_policy()?.unwrap_or_default();
        Ok(Self {
            store,
            coins,
            coin_confirmations: BTreeMap::new(),
            board_url: None,
            nostr_relays: None,
            auto_fund: false,
            fee_bump,
            progress: Mutex::new(HashMap::new()),
            wallet_manager: crate::wallet_bdk::WalletManager::new(data_dir),
            electrum_pool: crate::chain::ElectrumPool::new(),
            server_set: crate::server_health::ServerSet::new(),
        })
    }

    /// Update the live fee-bump policy and persist it for this merchant (pactd
    /// `setfeepolicy`). Validated before it takes effect; the persisted value is
    /// reloaded on the next [`Engine::open`].
    pub fn set_fee_bump(&mut self, policy: crate::fee_policy::FeeBumpPolicy) -> Result<()> {
        let policy = policy.validated()?;
        self.store.set_fee_policy(&policy)?;
        self.fee_bump = policy;
        Ok(())
    }

    fn backend(&self, chain: &ChainRef) -> Result<MultiBackend> {
        let urls = self.coins.get(&chain.coin_id).with_context(|| {
            format!(
                "coin {:?} has no chain-data backend configured — set it up in Satchel \
                 (or pass --coin {0}=<url>)",
                chain.coin_id
            )
        })?;
        let params = chain_params(chain)?;
        let first = urls.split(',').map(str::trim).find(|u| !u.is_empty());
        let backend = match first {
            // No Core-RPC primary ⇒ nodeless mode (docs/NODELESS_WALLET.md D5).
            Some(url) if !url.starts_with("http://") => {
                self.nodeless_backend(&chain.coin_id, params, urls)?
            }
            _ => self.node_backend(&chain.coin_id, params, urls)?,
        };
        backend.verify_chain()?;
        Ok(backend)
    }

    /// Core-RPC-primary backend list (the node-backed shape): the primary —
    /// and any further Core URLs — are stateless per-call HTTP clients;
    /// Electrum secondaries come from the shared pool so their TCP+TLS
    /// connections persist across calls (issue #87). Only the ACTIVE
    /// Electrum views join (issue #98) — the rest of the configured list
    /// is cold standby, promoted by the `ServerSet` when a slot frees up.
    fn node_backend(
        &self,
        coin_id: &str,
        params: &'static ChainParams,
        urls: &str,
    ) -> Result<MultiBackend> {
        let urls: Vec<&str> = urls
            .split(',')
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .collect();
        for url in &urls {
            anyhow::ensure!(
                url.starts_with("http://")
                    || url.starts_with("tcp://")
                    || url.starts_with("ssl://"),
                "unsupported backend URL scheme in {url:?} (http:// | tcp:// | ssl://)"
            );
        }
        let electrum: Vec<&str> = urls
            .iter()
            .copied()
            .filter(|u| u.starts_with("tcp://") || u.starts_with("ssl://"))
            .collect();
        let views = self
            .server_set
            .select(coin_id, &electrum, crate::server_health::ACTIVE_VIEWS);
        let mut backends: Vec<Box<dyn ChainBackend>> = Vec::new();
        for url in &urls {
            if url.starts_with("http://") {
                backends.push(Box::new(crate::chain::CoreRpcBackend::new(params, url)?));
            } else if views.contains(url) {
                backends.push(Box::new(
                    self.electrum_pool.get(params, coin_id, url, &views)?,
                ));
            }
        }
        MultiBackend::from_backends(backends)
    }

    /// Electrum-only URL list ⇒ nodeless mode: the primary becomes a
    /// [`crate::wallet_bdk::BdkWalletBackend`] (bdk wallet from the Pact
    /// mnemonic's BIP-86 branch over the ELECTED home server, #99); the
    /// active view servers join as independent chain views. A locked — or
    /// absent — seed keeps chain reads working and surfaces
    /// through `wallet_locked`, exactly like an encrypted, locked Core
    /// wallet.
    fn nodeless_backend(
        &self,
        coin_id: &str,
        params: &'static ChainParams,
        urls: &str,
    ) -> Result<MultiBackend> {
        let urls: Vec<&str> = urls
            .split(',')
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .collect();
        anyhow::ensure!(
            urls.iter()
                .all(|u| u.starts_with("tcp://") || u.starts_with("ssl://")),
            "nodeless coin {coin_id}: a Core-RPC (http://) URL must come FIRST in the \
             backend list to be the funding wallet — Electrum-first lists must be \
             Electrum-only"
        );
        // A single lying/withholding server must not be our only chain view
        // while real funds move (spec §10). Test networks may run on one.
        if params.network == Network::Mainnet {
            anyhow::ensure!(
                urls.len() >= 2,
                "nodeless coin {coin_id} on mainnet needs at least 2 Electrum servers \
                 (independent chain views) — add a second URL"
            );
        }
        // Active set (issue #98) + home election (#99): the wallet HOME is
        // ELECTED — sticky on the incumbent, re-homed only when it is
        // inside a failure backoff window. The sync worker follows via
        // `ensure_worker`/`set_chain` (its subscriptions are pinned to the
        // connection instance and rebuild with a full resync on the new
        // socket); the bdk store itself is server-agnostic (genesis-checked
        // at load, checkpoint reconciliation in `chain_update`). The view
        // slots are picked separately; everything else in the list is cold
        // standby with no pooled connection at all.
        let home = self
            .server_set
            .select_home(coin_id, &urls)
            .expect("nonempty url list");
        let view_candidates: Vec<&str> = urls.iter().copied().filter(|u| *u != home).collect();
        let views = self.server_set.select(
            coin_id,
            &view_candidates,
            crate::server_health::ACTIVE_VIEWS,
        );
        let live: Vec<&str> = std::iter::once(home).chain(views.iter().copied()).collect();
        // The primary chain view: the coin's pooled long-lived connection.
        // The sync worker dials the same server on its OWN connection (see
        // WalletManager::ensure_worker) so each socket has one caller
        // domain — engine RPCs here (serialized by the registry lock), the
        // worker there.
        let primary = self.electrum_pool.get(params, coin_id, home, &live)?;
        let view_backends: Vec<std::sync::Arc<crate::chain::ElectrumBackend>> = views
            .iter()
            .map(|url| self.electrum_pool.get(params, coin_id, url, &live))
            .collect::<Result<_>>()?;
        let wallet = match self.store.seed() {
            Ok(seed) => {
                let handle = self.wallet_manager.open(coin_id, params, &seed)?;
                let worker = self
                    .wallet_manager
                    .ensure_worker(coin_id, params, home, &handle)?;
                Some((handle, worker))
            }
            Err(_) => None, // locked or absent seed: chain-reads-only backend
        };
        let mut backends: Vec<Box<dyn ChainBackend>> =
            vec![Box::new(crate::wallet_bdk::BdkWalletBackend::new(
                params,
                primary,
                view_backends.clone(),
                wallet,
            ))];
        for view in view_backends {
            backends.push(Box::new(view));
        }
        MultiBackend::from_backends(backends)
    }

    /// Seconds since the coin's nodeless wallet cache was last confirmed
    /// against its home server — `None` for node-backed coins or before
    /// the worker's first completed pass. The "balance as of" signal
    /// (#99): it stops advancing while the home is unreachable.
    pub fn wallet_sync_age_secs(&self, coin_id: &str) -> Option<u64> {
        self.wallet_manager.sync_age(coin_id).map(|d| d.as_secs())
    }

    /// Passive health snapshots for one coin's configured Electrum servers,
    /// in configured order (pactd `serverstatus`, the Network page's data).
    /// A pure in-memory read of the shared health cells — it never dials,
    /// never probes: servers this run has not touched report `untested`
    /// (issue #98). Core-RPC (`http://`) entries are skipped — the health
    /// registry covers Electrum transports only.
    pub fn server_status(
        &self,
        coin_id: &str,
    ) -> Result<Vec<crate::server_health::HealthSnapshot>> {
        let urls = self
            .coins
            .get(coin_id)
            .with_context(|| format!("coin {coin_id:?} is not configured"))?;
        let urls: Vec<&str> = urls
            .split(',')
            .map(str::trim)
            .filter(|u| u.starts_with("tcp://") || u.starts_with("ssl://"))
            .collect();
        let mut snaps = crate::server_health::coin_snapshots(coin_id, &urls);
        // Roles from the sticky maps — a PEEK, not an election: display
        // reads must never move routing state. A coin that has not routed
        // this run shows no roles, which is the honest answer.
        let home = self.server_set.current_home(coin_id);
        let views = self.server_set.current_views(coin_id);
        let routed = home.is_some() || !views.is_empty();
        for snap in &mut snaps {
            snap.role = if home.as_deref() == Some(snap.url.as_str()) {
                Some("wallet".to_string())
            } else if views.iter().any(|v| v == &snap.url) {
                Some("view".to_string())
            } else if routed {
                Some("standby".to_string())
            } else {
                None
            };
        }
        Ok(snaps)
    }

    /// Live reachability gate for both legs of a swap: each coin's node must be
    /// reachable **and** serve the right chain (genesis check, via
    /// [`Self::backend`]) right now. Run at the network-facing swap-initiation
    /// entry points (`post_board_offer`, `take_board_offer`, `take_offer_slip`)
    /// so advertising or taking a swap with a down node is refused up front with
    /// a clear message, rather than failing later mid-swap. The pure envelope
    /// builders (`offer`/`accept`/`make_private_offer`) don't touch a node, so
    /// they're not gated here — funding still fails loudly if a chain is down.
    /// Mirrors the per-coin check the UI shows in `listcoins`.
    fn ensure_chains_live(&self, chains: &[&ChainRef]) -> Result<()> {
        for c in chains {
            let backend = self.backend(c).with_context(|| {
                format!(
                    "chain {} is unreachable — check that its node is running and \
                     configured in Satchel before starting a swap",
                    c.coin_id
                )
            })?;
            backend.tip_height().with_context(|| {
                format!(
                    "chain {} is unreachable — check that its node is running and \
                     configured in Satchel before starting a swap",
                    c.coin_id
                )
            })?;
            // The quorum reads above tolerate a dead server behind healthy
            // siblings — right for display, but the FUNDING WALLET's own
            // server must answer before we commit to a swap it will have
            // to fund (issue #98; the wallet re-homes in #99).
            backend.wallet_view_live().with_context(|| {
                format!(
                    "the {} wallet's server is unreachable — the swap could not be \
                     funded; check the coin's first backend URL or wait for it to recover",
                    c.coin_id
                )
            })?;
        }
        Ok(())
    }

    /// Coin ids with a configured chain-data backend (display order: the
    /// shipped registry order, then any extras). Drives `listcoins`/`listpairs`.
    pub fn configured_coins(&self) -> Vec<String> {
        let mut ordered: Vec<String> = registry::all()
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

    /// The Core-wallet name a coin's primary backend is scoped to, parsed from
    /// its configured URL (`…/wallet/<name>`). `None` when the URL carries no
    /// wallet path (the node's *default* wallet — i.e. wallet ops are NOT
    /// explicitly scoped). Ground truth for display: it reflects exactly the
    /// endpoint every wallet RPC for this coin hits. The primary is the first
    /// comma-separated backend (the wallet-qualified Core RPC).
    pub fn coin_wallet(&self, coin_id: &str) -> Option<String> {
        let url = self.coins.get(coin_id)?;
        let primary = url.split(',').next().unwrap_or(url);
        let after = primary.split("/wallet/").nth(1)?;
        let name = after.split(['/', '?', '#']).next().unwrap_or(after);
        (!name.is_empty()).then(|| name.to_string())
    }

    /// Whether `coin_id` is configured NODELESS (docs/NODELESS_WALLET.md D5):
    /// its backend list has no Core-RPC primary, so the wallet is the bdk one
    /// derived from the Pact seed. Mirrors the [`Engine::backend`] dispatch
    /// without building a backend; the UI keys the send/receive/activity
    /// surface (and the "pact seed" wallet label) off this.
    pub fn coin_nodeless(&self, coin_id: &str) -> bool {
        let Some(urls) = self.coins.get(coin_id) else {
            return false;
        };
        match urls.split(',').map(str::trim).find(|u| !u.is_empty()) {
            Some(url) => !url.starts_with("http://"),
            None => false,
        }
    }

    /// The confirmation depth (reorg-safety / finality) to require for `chain`:
    /// the operator's per-coin setting if present, else the network/spacing
    /// [`default_confirmations`] heuristic. The single source of truth for
    /// N_a/N_b across v1 and v2. Operator values are CLAMPED into
    /// [`confirmation_bounds`] — a legacy or hand-edited config can therefore
    /// never produce a depth the handshake validation would reject (the
    /// failure mode of the 2026-07-08 mainnet incident, where a taker's
    /// btc=2 setting silently killed every v2 take under the old ≥6 floor).
    pub fn confirmations_for(&self, chain: &ChainRef) -> Result<u32> {
        let params = chain_params(chain)?;
        let (floor, cap) = confirmation_bounds(params);
        if let Some(n) = self.coin_confirmations.get(&chain.coin_id) {
            return Ok((*n).clamp(floor, cap));
        }
        Ok(default_confirmations(params))
    }

    /// The cooperative-redeem feerate (sat/vB) the initiator fixes at init for
    /// `chain` (M2): the **live market** rate, floored at [`MIN_REDEEM_FEERATE`]
    /// (min-relay) and clamped to the **protocol** [`MAX_REDEEM_FEERATE`] (NOT the
    /// local `max_feerate_sat_vb` bump ceiling). Committed at market with no
    /// over-provision — the committed fee can't be RBF'd, but the deadline-aware
    /// CPFP child lifts it if the market climbs while it's pending. On regtest the
    /// node can't estimate, so market lands on its ≈1 fallback (= the floor). The
    /// value is negotiated into the init and must pass the counterparty's protocol
    /// validation (§2 init check); a conservative fallback applies when no backend
    /// is reachable. Only the initiator calls this; the participant adopts the
    /// value from the signed init, so the two never diverge.
    fn adaptor_redeem_feerate(&self, chain: &ChainRef) -> u64 {
        match self.backend(chain).and_then(|b| b.fee_rate_sat_per_vb()) {
            Ok(rate) => rate.clamp(MIN_REDEEM_FEERATE, MAX_REDEEM_FEERATE),
            Err(_) => ADAPTOR_REDEEM_FEERATE_FALLBACK,
        }
    }

    /// The effective confirmation depth per *configured* coin, for `listcoins`
    /// (so the setup UI can show the value in force, its default, and the
    /// allowed floor). Returns `(effective, default, min)` for the given coin
    /// on `network` — `default` doubles as the maximum (see
    /// [`confirmation_bounds`]), `effective` is clamped like
    /// [`Self::confirmations_for`].
    pub fn coin_confirmations_view(
        &self,
        network: Network,
        coin_id: &str,
    ) -> Result<(u32, u32, u32)> {
        let chain = ChainRef {
            coin_id: coin_id.to_string(),
            network,
        };
        let params = chain_params(&chain)?;
        let default = default_confirmations(params);
        let (floor, cap) = confirmation_bounds(params);
        let effective = self
            .coin_confirmations
            .get(coin_id)
            .copied()
            .map(|n| n.clamp(floor, cap))
            .unwrap_or(default);
        Ok((effective, default, floor))
    }

    /// Live connection probe for a *configured* coin: verifies the backend
    /// serves the right chain (genesis check, via `backend`) and
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

    /// Network admission policy: regtest is free; testnet and mainnet permit an
    /// unencrypted seed but warn. Post-#120 "unencrypted" means the obfuscation
    /// fallback (no OS keystore) or a legacy plaintext seed — a keystore or
    /// passphrase seed is encrypted and passes silently. Mainnet is open for
    /// both v1 (HTLC) and v2+ (adaptor) swaps.
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
                // Mainnet is enabled for both v1 (HTLC) and v2+ (adaptor;
                // registry::ADAPTOR_MAINNET_ENABLED). An unencrypted hot seed is
                // far riskier here than on testnet — warn loudly, but permit it
                // (Bitcoin-Core-style: your funds, your responsibility).
                if !self.store.seed_is_encrypted()? {
                    eprintln!(
                        "warning: running MAINNET with an UNENCRYPTED seed — anyone with \
                         file/host access gets your transit keys + identity, and these are \
                         REAL FUNDS. Encrypting the seed is strongly recommended."
                    );
                }
                Ok(())
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

    /// Hard pre-flight for the gated paths (board post / take / private take):
    /// the leg we'll lock (the maker's `give`, the taker's `get`) must already be
    /// covered by the core wallet, INCLUDING funding-fee headroom. Called only
    /// after the chain-up gate (`ensure_chains_live`), so the node is reachable
    /// and the balance read should succeed — and unlike the old best-effort form
    /// this REFUSES when it can't, rather than silently letting an un-fundable
    /// swap onto the board. The pure envelope builders (`offer`,
    /// `make_private_offer`) deliberately do NOT call this — they must work
    /// offline; funding is hard-gated again at `fund` time.
    ///
    /// Headroom: we reserve `amount + ceiling × FUNDING_VSIZE_EST`, where
    /// `ceiling = min(MULT × live feerate, MAX_REDEEM_FEERATE)`. The funding tx is
    /// the only wallet-funded action (redeem/refund/bump fees come out of the
    /// output), so this is the only place wallet headroom matters — and sizing it
    /// to the funding-bump ceiling means an exact-balance offer can't pass here
    /// then fail at fund time, nor can a fee spike between post and broadcast.
    fn ensure_can_fund(&self, network: Network, coin_id: &str, amount: u64) -> Result<()> {
        let chain = ChainRef {
            coin_id: coin_id.to_string(),
            network,
        };
        let balance = self.wallet_balance_for(&chain)?;
        let fee_headroom = self.funding_fee_headroom(&chain);
        let needed = amount.saturating_add(fee_headroom);
        ensure!(
            balance >= needed,
            "insufficient {coin_id} balance to fund this swap: have {balance} sat, \
             need ~{needed} sat ({amount} to lock + ~{fee_headroom} funding-fee headroom)"
        );
        self.ensure_wallet_unlocked(&chain)?;
        Ok(())
    }

    /// Gate: refuse if the node's wallet is encrypted+locked. It would pass the
    /// balance read above but fail to SIGN the funding tx (RPC -13), stranding
    /// the swap at fund time. Checked at take (taker's get-leg) and post (maker's
    /// give-leg) so neither side commits to a swap it can't fund.
    fn ensure_wallet_unlocked(&self, chain: &ChainRef) -> Result<()> {
        // Fail-open on a probe error: the balance read already gates node
        // reachability, so don't block trading on a transient getwalletinfo hiccup.
        let locked = self.backend(chain)?.wallet_locked().unwrap_or(false);
        ensure!(
            !locked,
            "your {} wallet is locked — unlock it (walletpassphrase) before trading \
             and keep it unlocked until the swap completes. A locked wallet can read \
             your balance but cannot sign the funding transaction.",
            chain.coin_id
        );
        Ok(())
    }

    /// Read the live core-wallet balance (sat) for `chain`, with a friendly
    /// error if the node/wallet isn't reachable.
    fn wallet_balance_for(&self, chain: &ChainRef) -> Result<u64> {
        self.backend(chain)?.wallet_balance().with_context(|| {
            format!(
                "couldn't read {} balance to confirm this swap is fundable \
                 — is the node up and a wallet loaded?",
                chain.coin_id
            )
        })
    }

    /// Worst-case funding-fee headroom (sat) reserved per funding tx for `chain`.
    /// Sized to the funding-bump ceiling the nurse may chase, so an exact-balance
    /// offer can't pass a fund gate then fail at fund time, nor can a fee spike
    /// between post and broadcast.
    fn funding_fee_headroom(&self, chain: &ChainRef) -> u64 {
        let live = self
            .backend(chain)
            .and_then(|b| {
                let ct = b.funding_conf_target();
                b.fee_rate_for(ct, false)
            })
            .unwrap_or(FUNDING_FEERATE_FALLBACK);
        // The clamp is written panic-safe: `u64::clamp` panics if `min > max`,
        // and a low `max_feerate_sat_vb` (< FUNDING_FEERATE_FALLBACK) would
        // otherwise crash here, so the floor drops with the ceiling.
        let max_feerate = self.fee_bump.max_feerate_sat_vb;
        let ceiling = live
            .saturating_mul(self.fee_bump.funding.reservation_mult)
            .clamp(FUNDING_FEERATE_FALLBACK.min(max_feerate), max_feerate);
        ceiling.saturating_mul(FUNDING_VSIZE_EST)
    }

    /// Sum of give-leg amounts (and the offer count) across this maker's
    /// still-live offers whose `give` leg is `coin_id` on `network`. Used by the
    /// post-time cumulative funds gate.
    fn committed_give_for_coin(&self, network: Network, coin_id: &str) -> Result<(u64, usize)> {
        let net_str = format!("{network:?}").to_lowercase();
        let mut sum = 0u64;
        let mut count = 0usize;
        for o in self.store.my_offers_live()? {
            // Skip anything that won't parse — a malformed local row shouldn't
            // block a legitimate new offer.
            let Ok(env) = serde_json::from_str::<Envelope>(&o.envelope) else {
                continue;
            };
            let Ok(body) = serde_json::from_value::<crate::board::OfferBody>(env.body) else {
                continue;
            };
            if body.give_asset == coin_id && body.network == net_str {
                sum = sum.saturating_add(body.give_amount);
                count += 1;
            }
        }
        Ok((sum, count))
    }

    /// Like [`Self::ensure_can_fund`], but also charges the give-leg amounts the
    /// maker has ALREADY committed across their still-live offers in the same
    /// coin. Nothing is locked on-chain (offers are advertisements, funded only
    /// when taken), so without this a maker could post many offers that each
    /// individually fit their balance but together exceed it; a taker who
    /// commits against the surplus would then see the maker's fund fail at take
    /// time. Best-effort — in-flight takes aren't subtracted and cross-coin
    /// balances aren't netted — but it stops the obvious cumulative-overcommit
    /// case. The new offer isn't in the store yet, so it's charged separately.
    fn ensure_can_fund_new_offer(
        &self,
        network: Network,
        coin_id: &str,
        amount: u64,
    ) -> Result<()> {
        let chain = ChainRef {
            coin_id: coin_id.to_string(),
            network,
        };
        let balance = self.wallet_balance_for(&chain)?;
        let fee_headroom = self.funding_fee_headroom(&chain);
        let (committed, n_live) = self.committed_give_for_coin(network, coin_id)?;
        // Every live offer plus this new one needs its own funding-fee headroom
        // when taken, so headroom scales with the offer count.
        let n_offers = (n_live as u64).saturating_add(1);
        let total_amount = amount.saturating_add(committed);
        let needed = total_amount.saturating_add(fee_headroom.saturating_mul(n_offers));
        ensure!(
            balance >= needed,
            "insufficient {coin_id} balance to advertise this offer: have {balance} sat, \
             need ~{needed} sat ({amount} for this offer + {committed} already committed \
             across {n_live} live offer(s) + ~funding-fee headroom). Withdraw or let some \
             offers expire first."
        );
        self.ensure_wallet_unlocked(&chain)?;
        Ok(())
    }

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
        // No fund check here: `offer` is a pure envelope builder (works offline).
        // Fundability is hard-gated where money is actually committed — board
        // post / take (`ensure_can_fund`, after the chain-up gate) and `fund`.
        let n_a = match n_a {
            Some(n) => n,
            None => self.confirmations_for(&chain_a)?,
        };
        let n_b = match n_b {
            Some(n) => n,
            None => self.confirmations_for(&chain_b)?,
        };
        validate_profile(&chain_a, &chain_b, t1, t2, n_a, n_b)?;

        let seed = self.store.seed()?;
        let index = self.store.next_swap_index()?;
        let preimage = seed.preimage(index)?;
        let hash_h = hash_preimage(&preimage);
        let id = swap_id(&hash_h);

        let alice_refund_pubkey_a = seed.swap_pubkey(coin_of(&chain_a)?, index)?.to_string();
        let alice_redeem_pubkey_b = seed.swap_pubkey(coin_of(&chain_b)?, index)?.to_string();

        let body = InitBody {
            protocol: crate::PROTOCOL_VERSION.into(),
            wire: crate::WIRE_V1,
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
            swap_index: Some(index),
            chain_a,
            chain_b,
            amount_a: give.1,
            amount_b: get.1,
            hash_h: hex::encode(hash_h),
            t1,
            t2,
            n_a,
            n_b,
            their_n_a: None,
            their_n_b: None,
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
            last_action_height: 0,
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
        // Everything below up to the seed access is DETERMINISTIC validation
        // of the received init — failures are tagged permanent so the relay
        // loop fails fast (reasoned abort to the maker) instead of retrying.
        let body: InitBody = serde_json::from_value(init.body.clone())
            .context("malformed init body")
            .map_err(permanent_err)?;
        ensure_permanent!(
            body.protocol == crate::PROTOCOL_VERSION,
            "unknown protocol {} (we speak {})",
            body.protocol,
            crate::PROTOCOL_VERSION
        );
        ensure_permanent!(
            body.wire == crate::WIRE_V1,
            "peer speaks {} wire v{}, this build speaks v{} — both sides must run compatible releases",
            body.protocol,
            body.wire,
            crate::WIRE_V1
        );
        chain_params(&body.chain_a).map_err(permanent_err)?;
        chain_params(&body.chain_b).map_err(permanent_err)?;
        ensure_permanent!(
            body.chain_a.network == body.chain_b.network,
            "both legs must be on the same network tier"
        );
        self.ensure_network_allowed(body.chain_a.network)
            .map_err(permanent_err)?;
        ensure_permanent!(
            body.chain_a.coin_id != body.chain_b.coin_id,
            "chains must differ"
        );
        ensure_pair_supported(&body.chain_a, &body.chain_b).map_err(permanent_err)?;
        ensure_permanent!(body.t2 < body.t1, "spec §7.1: T2 must be < T1");
        ensure_permanent!(
            body.amount_a > 0 && body.amount_b > 0,
            "amounts must be positive"
        );
        // Per-side depth ownership (wire v2, rc12 recut): we derive OUR OWN
        // n_a/n_b from local config — the maker's body values are advisory
        // (display only), never adopted. Both sets must sit in the §7.3 band;
        // an out-of-band advisory value is a foreseeable liveness stall and
        // is refused before any state exists.
        let n_a = self.confirmations_for(&body.chain_a)?;
        let n_b = self.confirmations_for(&body.chain_b)?;
        validate_profile(&body.chain_a, &body.chain_b, body.t1, body.t2, n_a, n_b)
            .map_err(permanent_err)?;
        ensure_confs_in_bounds(&body.chain_a, body.n_a, "advisory N_A").map_err(permanent_err)?;
        ensure_confs_in_bounds(&body.chain_b, body.n_b, "advisory N_B").map_err(permanent_err)?;
        let hash_h = parse_hash(&body.hash_h).map_err(permanent_err)?;
        ensure_permanent!(
            init.swap_id == swap_id(&hash_h),
            "swap_id does not match hash_h (spec §4.4)"
        );
        parse_pubkey(&body.alice_refund_pubkey_a, "alice refund A").map_err(permanent_err)?;
        parse_pubkey(&body.alice_redeem_pubkey_b, "alice redeem B").map_err(permanent_err)?;

        let seed = self.store.seed()?;
        // Participant keys are anchored to the swap's hash H (spec §4.2): no
        // local counter is allocated — the same seed derives the same keys from
        // the init alone on any machine, and two different swaps can never
        // share a key (the initiator guarantees H unique via ITS counter).
        let bob_redeem_pubkey_a = seed
            .swap_pubkey_anchored(coin_of(&body.chain_a)?, &hash_h)?
            .to_string();
        let bob_refund_pubkey_b = seed
            .swap_pubkey_anchored(coin_of(&body.chain_b)?, &hash_h)?
            .to_string();

        let record = SwapRecord {
            swap_id: init.swap_id.clone(),
            role: Role::Participant,
            state: State::Accepted,
            created_at: local_now(),
            swap_index: None,
            chain_a: body.chain_a,
            chain_b: body.chain_b,
            amount_a: body.amount_a,
            amount_b: body.amount_b,
            hash_h: body.hash_h,
            t1: body.t1,
            t2: body.t2,
            n_a,
            n_b,
            their_n_a: Some(body.n_a),
            their_n_b: Some(body.n_b),
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
            last_action_height: 0,
        };
        self.store.put(&record)?;
        let _ = self.snapshot_v1(&record); // rescue snapshot at accept (#54)
        let body = AcceptBody {
            wire: crate::WIRE_V1,
            bob_redeem_pubkey_a,
            bob_refund_pubkey_b,
            n_a,
            n_b,
        };
        let envelope =
            self.signed_envelope("accept", &init.swap_id, serde_json::to_value(&body)?)?;
        Ok((record, envelope))
    }

    /// v2 (pact-htlc-v2) initiator: build the adaptor-swap `init` (spec v2 §7).
    /// Reserves the swap index (so the v2 keys + adaptor secret are claimed)
    /// and returns the signed `InitV2` envelope. Runs on every network (v2+ is
    /// mainnet-enabled). The full stateful lifecycle (funding, redeem,
    /// scheduler) is driven here on top of the crypto/tx flow in `adaptor_engine`.
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
        // Fresh core-wallet sweep address for the leg Alice will redeem (B),
        // communicated so both parties co-sign the identical redeem tx and the
        // proceeds land in a spendable core wallet. Best-effort: empty (→ the
        // deterministic swap-key fallback) if there's no node to ask.
        let alice_sweep_b = self
            .backend(&chain_b)
            .and_then(|b| b.wallet_new_address())
            .unwrap_or_default();
        // M2: fix the unbumpable cooperative-redeem feerates now, from live
        // estimators (over-provisioned), and carry them in the signed init so
        // both parties build byte-identical redeem txs.
        let redeem_feerate_a = self.adaptor_redeem_feerate(&chain_a);
        let redeem_feerate_b = self.adaptor_redeem_feerate(&chain_b);
        // OUR per-side confirmation depths (spec §7.3, local policy) — carried
        // in the init as ADVISORY values so the participant's "waiting for
        // them" display is exact; the participant derives its own gates.
        let (n_a, n_b) = (
            self.confirmations_for(&chain_a)?,
            self.confirmations_for(&chain_b)?,
        );
        let body = crate::messages::InitV2Body {
            protocol: crate::adaptor_swap::PROTOCOL_V2.into(),
            wire: crate::WIRE_V2,
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
            alice_sweep_b: alice_sweep_b.clone(),
            redeem_feerate_a,
            redeem_feerate_b,
            n_a,
            n_b,
            offer_id: None,
        };
        let id = crate::keys::swap_id_v2(&adaptor_point);
        // v2 inherits v1 §7.3: enforce the full timelock profile (δ = T1−T2 ≥ 4h
        // is the safety-critical one — the window in which the participant must
        // confirm its unbumpable key-path leg-A redeem after the secret is
        // revealed), not just T2 < T1. Matches v1's take/accept discipline.
        validate_profile(&chain_a, &chain_b, t1, t2, n_a, n_b)?;
        let rec = AdaptorSwapRecord {
            swap_id: id.clone(),
            role: Role::Initiator,
            state: AdaptorState::Created,
            created_at: local_now(),
            swap_index: Some(index),
            chain_a,
            chain_b,
            amount_a,
            amount_b,
            t1,
            t2,
            n_a,
            n_b,
            their_n_a: None,
            their_n_b: None,
            adaptor_point: adaptor_point.to_string(),
            alice_swap_a: body.alice_swap_a.clone(),
            alice_swap_b: body.alice_swap_b.clone(),
            alice_refund_a: body.alice_refund_a.clone(),
            bob_swap_a: None,
            bob_swap_b: None,
            bob_refund_b: None,
            sweep_a: None,
            sweep_b: (!alice_sweep_b.is_empty()).then(|| alice_sweep_b.clone()),
            redeem_feerate_a,
            redeem_feerate_b,
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
            last_action_height: 0,
            funding_b_tx_hex: None,
            funding_b_broadcast: false,
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
        // Everything below up to the sweep/seed access is DETERMINISTIC
        // validation of the received init — failures are tagged permanent so
        // the relay loop fails fast (reasoned abort to the maker) instead of
        // retrying ten times in silence.
        let body: crate::messages::InitV2Body = serde_json::from_value(init.body.clone())
            .context("malformed init-v2 body")
            .map_err(permanent_err)?;
        ensure_permanent!(
            body.protocol == crate::adaptor_swap::PROTOCOL_V2,
            "unknown protocol {} (we speak {})",
            body.protocol,
            crate::adaptor_swap::PROTOCOL_V2
        );
        // Wire gate BEFORE any key material is derived: an epoch mismatch
        // would otherwise surface as a partial-signature failure deep in the
        // MuSig2 handshake (or worse, divergent redeem sighashes).
        ensure_permanent!(
            body.wire == crate::WIRE_V2,
            "peer speaks {} wire v{}, this build speaks v{} — both sides must run compatible releases",
            body.protocol,
            body.wire,
            crate::WIRE_V2
        );
        ensure_permanent!(
            body.chain_a.network == body.chain_b.network,
            "both legs must be on the same network"
        );
        self.ensure_network_allowed(body.chain_a.network)
            .map_err(permanent_err)?;
        ensure_permanent!(
            body.chain_a.coin_id != body.chain_b.coin_id,
            "chains must differ"
        );
        ensure_adaptor_supported(&body.chain_a, &body.chain_b).map_err(permanent_err)?;
        ensure_permanent!(body.t2 < body.t1, "spec v2 §6: T2 must be < T1");
        ensure_permanent!(
            body.amount_a > 0 && body.amount_b > 0,
            "amounts must be positive"
        );
        // M2: reject an init that sets an absurd redeem feerate — the redeem fee
        // is committed and eats the claimer's own output, so a malicious maker
        // could grief us. Bounds-check (don't clamp: both parties must use the
        // exact same value or the MuSig2 sighashes won't match).
        ensure_permanent!(
            (1..=MAX_REDEEM_FEERATE).contains(&body.redeem_feerate_a)
                && (1..=MAX_REDEEM_FEERATE).contains(&body.redeem_feerate_b),
            "init sets an invalid redeem feerate (must be 1..={MAX_REDEEM_FEERATE} sat/vB) — refusing (spec v2 §5)"
        );
        ensure_permanent!(
            init.swap_id
                == crate::keys::swap_id_v2(
                    &parse_pubkey(&body.adaptor_point, "adaptor point").map_err(permanent_err)?
                ),
            "swap_id does not match the adaptor point (spec v2 §3.3)"
        );
        parse_pubkey(&body.alice_swap_a, "alice swap A").map_err(permanent_err)?;
        parse_pubkey(&body.alice_swap_b, "alice swap B").map_err(permanent_err)?;
        body.alice_refund_a
            .parse::<bitcoin::XOnlyPublicKey>()
            .context("alice refund A")
            .map_err(permanent_err)?;

        // Carry Alice's leg-B sweep address through, and mint our own (Bob's)
        // fresh sweep address for the leg we redeem (A). Best-effort: empty →
        // the deterministic swap-key fallback.
        let alice_sweep_b = body.alice_sweep_b.clone();
        let bob_sweep_a = self
            .backend(&body.chain_a)
            .and_then(|b| b.wallet_new_address())
            .unwrap_or_default();

        let seed = self.store.seed()?;
        // v2 participant keys are anchored to the adaptor point T (spec §4.2)
        // — the value the swap id itself derives from — instead of a local
        // counter. Parsing T here also validates it before we sign anything.
        let anchor = parse_pubkey(&body.adaptor_point, "adaptor point")?.serialize();
        // OUR per-side depths (spec §7.3, local policy) — the initiator's
        // body values are advisory (display only), never adopted.
        let (n_a, n_b) = (
            self.confirmations_for(&body.chain_a)?,
            self.confirmations_for(&body.chain_b)?,
        );
        let body_out = crate::messages::AcceptV2Body {
            wire: crate::WIRE_V2,
            bob_swap_a: seed
                .swap_pubkey_anchored(coin_of(&body.chain_a)?, &anchor)?
                .to_string(),
            bob_swap_b: seed
                .swap_pubkey_anchored(coin_of(&body.chain_b)?, &anchor)?
                .to_string(),
            bob_refund_b: seed
                .refund_xonly_pubkey_anchored(coin_of(&body.chain_b)?, &anchor)?
                .to_string(),
            bob_sweep_a: bob_sweep_a.clone(),
            n_a,
            n_b,
        };
        // SECURITY BOUNDARY (spec v2 §8 inherits v1 §7.3 / §8.3): the participant
        // holds real funds against an untrusted counterparty, so it MUST validate
        // the absolute t1/t2 in the received init against its own clock — above
        // all δ = T1−T2 ≥ 4h. Without this a hostile maker could send a normal T2
        // but a tiny δ; the participant would fund leg B and then be unable to
        // confirm its unbumpable leg-A redeem before T1 → loses leg B.
        validate_profile(&body.chain_a, &body.chain_b, body.t1, body.t2, n_a, n_b)
            .map_err(permanent_err)?;
        // The initiator's advisory depths must sit in the §7.3 band too — an
        // out-of-band value is a foreseeable liveness stall (they'd act far
        // later than any honest profile allows), refused before state exists.
        ensure_confs_in_bounds(&body.chain_a, body.n_a, "advisory N_A").map_err(permanent_err)?;
        ensure_confs_in_bounds(&body.chain_b, body.n_b, "advisory N_B").map_err(permanent_err)?;
        let rec = AdaptorSwapRecord {
            swap_id: init.swap_id.clone(),
            role: Role::Participant,
            state: AdaptorState::Accepted,
            created_at: local_now(),
            swap_index: None,
            chain_a: body.chain_a,
            chain_b: body.chain_b,
            amount_a: body.amount_a,
            amount_b: body.amount_b,
            t1: body.t1,
            t2: body.t2,
            n_a,
            n_b,
            their_n_a: Some(body.n_a),
            their_n_b: Some(body.n_b),
            adaptor_point: body.adaptor_point,
            alice_swap_a: body.alice_swap_a,
            alice_swap_b: body.alice_swap_b,
            alice_refund_a: body.alice_refund_a,
            bob_swap_a: Some(body_out.bob_swap_a.clone()),
            bob_swap_b: Some(body_out.bob_swap_b.clone()),
            bob_refund_b: Some(body_out.bob_refund_b.clone()),
            sweep_a: (!bob_sweep_a.is_empty()).then(|| bob_sweep_a.clone()),
            sweep_b: (!alice_sweep_b.is_empty()).then(|| alice_sweep_b.clone()),
            redeem_feerate_a: body.redeem_feerate_a,
            redeem_feerate_b: body.redeem_feerate_b,
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
            last_action_height: 0,
            funding_b_tx_hex: None,
            funding_b_broadcast: false,
        };
        self.store.put_adaptor(&rec)?;
        let _ = self.snapshot_v2(&rec); // rescue snapshot at accept (#54)
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
        // M2: the cooperative redeem fee is the per-chain feerate negotiated at
        // init (see adaptor_redeem_feerate), NOT a hardcoded constant. Both
        // parties read the same stored value so the tx (and its sighash) is
        // identical. Always set at init/accept — a 0 here is a construction bug.
        let feerate = if leg_tag == "redeem_b" {
            rec.redeem_feerate_b
        } else {
            rec.redeem_feerate_a
        };
        ensure!(feerate > 0, "redeem feerate not set on the swap record");
        let fee = spend_fee_sat(feerate, crate::taproot::KEYPATH_REDEEM_VSIZE);
        let (leg, chain, amount, claimer, txid, vout, sweep) = if leg_tag == "redeem_b" {
            (
                p.leg_b(secp)?,
                &rec.chain_b,
                rec.amount_b,
                p.alice_swap_b,
                rec.funding_b_txid.as_deref(),
                rec.funding_b_vout,
                rec.sweep_b.as_deref(),
            )
        } else {
            (
                p.leg_a(secp)?,
                &rec.chain_a,
                rec.amount_a,
                p.bob_swap_a,
                rec.funding_a_txid.as_deref(),
                rec.funding_a_vout,
                rec.sweep_a.as_deref(),
            )
        };
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid.context("no funding txid for leg yet")?)?,
            vout: vout.context("no funding vout for leg yet")?,
        };
        // Sweep to the communicated fresh core-wallet address when present;
        // otherwise the deterministic swap-key destination. Both parties agree
        // on which, since the address travels in the signed init/accept.
        let dest = match sweep {
            Some(addr) if !addr.is_empty() => chain_params(chain)?.parse_address(addr)?,
            _ => adaptor_redeem_dest(chain, &claimer)?,
        };
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
        let my_scalar = crate::musig::seckey_to_scalar(&v2_swap_key(&seed, rec, coin)?)?;
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
        // Rescue snapshot at Signed (#54): the record now carries the assembled
        // adaptor signatures — the one datum that is neither seed- nor
        // chain-derivable — so a rescued machine can COMPLETE, not just refund.
        let _ = self.snapshot_v2(&rec);
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
                // Replay-safe (#54): state is MONOTONIC. A rescued node re-reads
                // relay history from a fresh cursor, so a re-delivered accept
                // must be a silent no-op — regressing a Signed record to
                // Accepted re-opened the handshake (dead-ended by nonce-safety)
                // and starved the scheduler redeem. v1's recv() gets the same
                // safety from its strict `accept in state` ensure.
                if rec.state != AdaptorState::Created {
                    return Ok(rec);
                }
                let b: crate::messages::AcceptV2Body =
                    serde_json::from_value(envelope.body.clone())
                        .context("malformed accept-v2 body")?;
                ensure!(
                    b.wire == crate::WIRE_V2,
                    "peer speaks pact-htlc-v2 wire v{}, this build speaks v{} — both sides must run compatible releases",
                    b.wire,
                    crate::WIRE_V2
                );
                parse_pubkey(&b.bob_swap_a, "bob swap A")?;
                parse_pubkey(&b.bob_swap_b, "bob swap B")?;
                b.bob_refund_b
                    .parse::<bitcoin::XOnlyPublicKey>()
                    .context("bob refund B")?;
                // The participant's advisory depths (display: "waiting for
                // them" shows the depth they actually act at). Out-of-band =
                // foreseeable liveness stall, refused (spec §7.3 band).
                ensure_confs_in_bounds(&rec.chain_a, b.n_a, "advisory N_A")?;
                ensure_confs_in_bounds(&rec.chain_b, b.n_b, "advisory N_B")?;
                rec.their_n_a = Some(b.n_a);
                rec.their_n_b = Some(b.n_b);
                rec.bob_swap_a = Some(b.bob_swap_a);
                rec.bob_swap_b = Some(b.bob_swap_b);
                rec.bob_refund_b = Some(b.bob_refund_b);
                rec.sweep_a = (!b.bob_sweep_a.is_empty()).then_some(b.bob_sweep_a);
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
        // Initiator snapshots once at accept (#54); Signed is snapshotted in
        // adaptor_assemble for both roles.
        if envelope.msg_type == "accept" {
            let _ = self.snapshot_v2(&rec);
        }
        Ok(rec)
    }

    /// Fund OUR leg's Taproot output via the core wallet, then emit
    /// `funding_ready` (spec v2 §7). Chain-touching: proven against live
    /// nodes (the in-process flow is covered by `adaptor_funding_ready`).
    pub fn adaptor_fund(&self, swap: &str) -> Result<Envelope> {
        let rec = self.store.get_adaptor(swap)?;
        // CRITICAL: the participant NEVER broadcasts leg B here — not even on the
        // manual RPC path. It builds + pre-signs leg B; the scheduler broadcasts it
        // only once the swap is `Signed` (σ_A held) AND leg A is verified on-chain
        // n_a-deep. This makes a hand-driven taker fail-SAFE: the obvious manual
        // order (fund → nonces → sign → assemble) would otherwise commit leg B
        // before σ_A exists — the exact fund-loss the autopilot fix closes. If a
        // manual taker never triggers the broadcast (`tick`), the swap merely
        // stalls and both refund; nothing is committed prematurely.
        if rec.role == Role::Participant {
            return self.adaptor_build_leg_b(swap);
        }
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(&rec)?;
        let leg = p.leg_a(&secp)?;
        let backend = self.backend(&rec.chain_a)?;
        let leg_spk = leg.script_pubkey(&secp)?;

        // Idempotency guard 1: a recorded pointer means leg A is already funded —
        // re-announce it, never fund twice (this also covers the still-unconfirmed
        // window that the confirmed-only find_funding below cannot see).
        if let (Some(txid), Some(vout)) = (rec.funding_a_txid.clone(), rec.funding_a_vout) {
            return self.adaptor_funding_ready(swap, &txid, vout);
        }
        // Idempotency guard 2 (locate-first, rc6 #2): adopt an existing confirmed
        // output at the leg address instead of funding again — covers a crash
        // between broadcast and the pointer persist, so a retry never double-funds.
        if let Some((op, info)) = backend.find_funding(&leg_spk)? {
            if info.value_sat == rec.amount_a {
                return self.adaptor_funding_ready(swap, &op.txid.to_string(), op.vout);
            }
        }

        // Initiator: broadcast leg A now. Safe — leg A is only claimable after the
        // initiator reveals `t` (which only it can do) and its refund is intact.
        let address = leg.address(&secp, backend.params())?;
        let txid = backend.wallet_send(
            &address,
            rec.amount_a,
            SendFee::Target(backend.funding_conf_target()),
        )?;
        let vout = backend.find_vout(&txid, &hex::encode(leg_spk.as_bytes()))?;
        self.adaptor_funding_ready(swap, &txid, vout)
    }

    /// CRITICAL two-phase leg-B funding for the board autopilot (spec v2 §7,
    /// xmr-btc-swap ordering): the participant BUILDS its leg-B funding tx but
    /// does NOT broadcast it. The redeems are pre-signed over this outpoint; the
    /// scheduler broadcasts it only after the swap is `Signed` (so the taker holds
    /// a verified σ_A) AND leg A is verified on-chain `n_a`-deep
    /// (`adaptor_tick_one`) — so the taker never commits leg B before it can
    /// guarantee claiming leg A. Persist the pointer AND the exact signed tx in
    /// one write so a crash between build and broadcast rebroadcasts the tx the
    /// adaptor signatures commit to, never a freshly re-selected one. Idempotent:
    /// a recorded pointer or an existing on-chain output is re-adopted.
    fn adaptor_build_leg_b(&self, swap: &str) -> Result<Envelope> {
        let rec = self.store.get_adaptor(swap)?;
        debug_assert_eq!(rec.role, Role::Participant);
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(&rec)?;
        let leg = p.leg_b(&secp)?;
        let backend = self.backend(&rec.chain_b)?;
        let leg_spk = leg.script_pubkey(&secp)?;
        // Idempotency: already built/broadcast (pointer set), or already on-chain.
        if let (Some(txid), Some(vout)) = (rec.funding_b_txid.clone(), rec.funding_b_vout) {
            return self.adaptor_funding_ready(swap, &txid, vout);
        }
        if let Some((op, info)) = backend.find_funding(&leg_spk)? {
            if info.value_sat == rec.amount_b {
                return self.adaptor_funding_ready(swap, &op.txid.to_string(), op.vout);
            }
        }
        let address = leg.address(&secp, backend.params())?;
        let (txid, vout, tx_hex) = backend.wallet_build_funding(&address, rec.amount_b)?;
        let mut rec2 = self.store.get_adaptor(swap)?;
        rec2.funding_b_txid = Some(txid.clone());
        rec2.funding_b_vout = Some(vout);
        rec2.funding_b_tx_hex = Some(tx_hex);
        self.store.put_adaptor(&rec2)?;
        let body = crate::messages::FundingReadyV2Body {
            chain: "b".into(),
            txid,
            vout,
        };
        self.signed_envelope("funding_ready", swap, serde_json::to_value(&body)?)
    }

    /// CRITICAL gate (spec v2 §6.1/§8, xmr-btc-swap ordering): is the initiator's
    /// leg A on-chain as the P2TR we reconstructed locally, paying exactly
    /// `amount_a`, and buried `n_a` deep? The participant gates BOTH building leg
    /// B (so it never pre-signs a redeem for a leg that may not exist) AND
    /// broadcasting it on this — it must be certain it can claim leg A before it
    /// commits leg B. A bare `funding_ready(A)` pointer is NOT enough: it is
    /// untrusted, and a 0-conf leg A can be double-spent out from under us.
    fn adaptor_leg_a_confirmed(&self, rec: &AdaptorSwapRecord) -> Result<bool> {
        let (Some(txid), Some(vout)) = (rec.funding_a_txid.as_deref(), rec.funding_a_vout) else {
            return Ok(false);
        };
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(rec)?;
        let spk_a = p.leg_a(&secp)?.script_pubkey(&secp)?;
        let op = OutPoint {
            txid: bitcoin::Txid::from_str(txid)?,
            vout,
        };
        Ok(match self.backend(&rec.chain_a)?.get_txout(&op, &spk_a)? {
            Some(txout) => {
                txout.confirmations >= u64::from(rec.n_a.max(1))
                    && txout.script_pubkey_hex == hex::encode(spk_a.as_bytes())
                    && txout.value_sat == rec.amount_a
            }
            None => false,
        })
    }

    /// Is the participant's BUILT leg-B funding provably UNCOMMITTED — the
    /// signed tx never reached the network? Leg B is built at accept (spec v2
    /// §7) but broadcast only after `Signed` + leg A `n_a`-deep, so a set
    /// `funding_b_txid` alone commits nothing. True only when every observable
    /// agrees: we never two-phase-released it, its outpoint is not on
    /// chain/in the mempool, and it was never SPENT (a spent leg B means the
    /// maker redeemed it and revealed `t` — the swap must claim leg A, never
    /// abort; covers a pre-rescue incarnation having broadcast it).
    /// Conservative: any read error answers "committed".
    fn adaptor_leg_b_uncommitted(&self, rec: &AdaptorSwapRecord) -> bool {
        if rec.role != Role::Participant || rec.funding_b_broadcast {
            return false;
        }
        let Some(txid) = rec.funding_b_txid.as_deref() else {
            return true; // never even built
        };
        let Some(vout) = rec.funding_b_vout else {
            return false; // malformed pointer (txid without vout): be conservative
        };
        (|| -> Result<bool> {
            let secp = bitcoin::secp256k1::Secp256k1::new();
            let p = self.adaptor_params(rec)?;
            let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
            let op = OutPoint {
                txid: bitcoin::Txid::from_str(txid)?,
                vout,
            };
            let backend = self.backend(&rec.chain_b)?;
            if backend.get_txout(&op, &spk_b)?.is_some() {
                return Ok(false); // live on chain or in the mempool
            }
            let from = self.funding_scan_from_height(&backend, txid, &spk_b)?;
            Ok(backend.find_spend_witness(&op, &spk_b, from)?.is_none())
        })()
        .unwrap_or(false)
    }

    /// Best-effort release of a built-but-never-broadcast leg-B funding's
    /// input reservation (Core: `lockunspent`; nodeless bdk: evict the
    /// persisted phantom tx). Callers have already established the tx is
    /// uncommitted via [`Self::adaptor_leg_b_uncommitted`]. Returns whether
    /// the release succeeded, for tick-event detail; failure is non-fatal
    /// (Core locks clear on node restart, and a retry costs nothing).
    fn adaptor_cancel_built_leg_b(&self, rec: &AdaptorSwapRecord) -> bool {
        let Some(hex) = rec.funding_b_tx_hex.as_deref() else {
            return true; // nothing was built — nothing reserved
        };
        if rec.role != Role::Participant || rec.funding_b_broadcast {
            return true;
        }
        self.backend(&rec.chain_b)
            .and_then(|b| b.wallet_cancel_funding(hex))
            .is_ok()
    }

    /// Bounded block-scan start for watching the counterparty's redeem of OUR
    /// funding (`find_spend_witness` fallback). The funding is our own wallet tx,
    /// so its confirmations are readable without `-txindex` (and stay readable
    /// after its output is spent) — turn that into the funding's block height so
    /// the block scan covers `[funding_height, tip]` instead of scanning from
    /// genesis (which `from_height = 0` would do on mainnet). Falls back to the
    /// tip when the funding isn't confirmed yet (no mined spend can exist then).
    fn funding_scan_from_height(
        &self,
        backend: &MultiBackend,
        funding_txid: &str,
        spk: &ScriptBuf,
    ) -> Result<u64> {
        let tip = backend.tip_height()?;
        let confs = backend.tx_confirmations(funding_txid, Some(spk))?;
        Ok(tip.saturating_sub(confs.saturating_sub(1)))
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
                // §7.4 reveal deadline (v2 inherits v1 §7.4): on the FIRST
                // reveal, refuse to broadcast the adapted leg-B redeem — which
                // exposes `t` — within 2h of T2 (margin 0 on regtest). Past it,
                // Bob could refund leg B and still extract `t` to take leg A.
                // A re-broadcast from RedeemedB (t already public) is exempt: we
                // MUST keep fee-bumping it to confirmation.
                if rec.state == AdaptorState::Signed {
                    let net = rec.chain_b.network;
                    let (_, reveal_margin, _) = action_margins(net);
                    let mtp = self.backend(&rec.chain_b)?.tip_median_time()?;
                    let now = deadline_clock(net, local_now(), mtp);
                    ensure!(
                        action_safe(now, reveal_margin, rec.t2),
                        "REFUSING to reveal t: now {now} is within {}h of T2 {} — \
                         wait for the T1 refund of leg A instead (spec §7.4)",
                        reveal_margin / 3600,
                        rec.t2
                    );
                }
                let t = crate::musig::seckey_to_scalar(
                    &seed.adaptor_secret(
                        rec.swap_index
                            .context("initiator record missing its swap index")?,
                    )?,
                )?;
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
                // §7.4: Bob MUST redeem leg A before `T1 − 1h` (margin 0 on
                // regtest) — past that his redeem races Alice's T1 refund, and
                // the v2 cooperative redeem is unbumpable, so racing is futile.
                // Mirrors the v1 participant guard in `redeem`.
                let net = rec.chain_a.network;
                let (_, _, redeem_a_margin) = action_margins(net);
                let mtp_a = self.backend(&rec.chain_a)?.tip_median_time()?;
                let now = deadline_clock(net, local_now(), mtp_a);
                ensure!(
                    action_safe(now, redeem_a_margin, rec.t1),
                    "now {now} is within {}h of T1 {} — redeem would race Alice's refund (spec §7.4)",
                    redeem_a_margin / 3600,
                    rec.t1
                );
                let p = self.adaptor_params(&rec)?;
                let leg_b = p.leg_b(&secp)?;
                let outpoint_b = OutPoint {
                    txid: bitcoin::Txid::from_str(
                        rec.funding_b_txid.as_deref().context("no leg-B funding")?,
                    )?,
                    vout: rec.funding_b_vout.context("no leg-B vout")?,
                };
                let backend_b = self.backend(&rec.chain_b)?;
                let leg_b_spk = leg_b.script_pubkey(&secp)?;
                let from_b = self.funding_scan_from_height(
                    &backend_b,
                    rec.funding_b_txid.as_deref().context("no leg-B funding")?,
                    &leg_b_spk,
                )?;
                let witness = backend_b
                    .find_spend_witness(&outpoint_b, &leg_b_spk, from_b)?
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
        let _ = self.tombstone_swap(&rec.swap_id); // terminal: drop rescue snapshot (#54)
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
        // Least-advanced backend MTP for refund readiness (M6) — see refund().
        let mtp = backend.tip_median_time_min()?;
        ensure!(
            mtp >= u64::from(leg.locktime),
            "too early to refund: MTP {mtp} < T {}",
            leg.locktime
        );
        let refund_kp = v2_refund_key(&seed, &rec, coin)?.keypair(&secp);
        let dest = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        // A1: initial spend priced at the value-capped claim target (refund is a
        // claim spend — bounded by the leg value, not the funding fee ceiling).
        let fee = spend_fee_sat(
            self.fee_bump.claim_feerate(
                backend.fee_rate_sat_per_vb()?,
                amount,
                crate::taproot::SCRIPTPATH_REFUND_VSIZE,
            ),
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
        let _ = self.tombstone_swap(&rec.swap_id); // terminal: drop rescue snapshot (#54)
        Ok(rec)
    }

    /// Unattended auto-refund for a funded-but-unfinished v2 swap (spec §9.5):
    /// if OUR leg is funded and its single-key CLTV timelock has matured, sweep
    /// it back. Covers the case where funding was broadcast but the handshake
    /// then stalled before `Signed` — `adaptor_tick_one` otherwise ignores such
    /// records, leaving the leg locked until a human intervenes. No adaptor
    /// signature exists in that state, so neither leg can be cooperatively spent:
    /// the refund races nothing. Returns `None` when our leg was never funded
    /// (nothing to reclaim) or the timelock is not yet mature.
    fn adaptor_refund_if_due(&self, rec: &AdaptorSwapRecord) -> Result<Option<TickEvent>> {
        let (chain, txid_o, timelock) = match rec.role {
            Role::Initiator => (&rec.chain_a, &rec.funding_a_txid, rec.t1),
            Role::Participant => (&rec.chain_b, &rec.funding_b_txid, rec.t2),
        };
        if txid_o.is_none() {
            return Ok(None); // our leg was never funded — nothing to reclaim
        }
        let mtp = self.backend(chain)?.tip_median_time_min()?;
        if mtp < u64::from(timelock) {
            return Ok(None); // timelock not yet mature — keep waiting
        }
        // The participant's pointer may be a BUILT leg B that was never
        // two-phase-released (spec v2 §7) — a pre-Signed handshake that died
        // with the tx still in our wallet. There is no on-chain leg to refund
        // (the refund would error-loop on a nonexistent outpoint); once the
        // same maturity the refund would need has passed, release the built
        // tx's input reservation and go terminal. Same retry rule as the
        // §7.4 dead-end: terminal only once the release succeeds.
        if rec.role == Role::Participant && self.adaptor_leg_b_uncommitted(rec) {
            if !self.adaptor_cancel_built_leg_b(rec) {
                return Ok(None); // release failed (locked seed?) — retry next tick
            }
            let mut dead = rec.clone();
            dead.state = AdaptorState::Aborted;
            self.store.put_adaptor(&dead)?;
            let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
            return Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "abort-unbroadcast".into(),
                detail: "handshake died with leg B built but never broadcast; \
                         reserved inputs released, aborted"
                    .into(),
            }));
        }
        let r = self.adaptor_refund(&rec.swap_id)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "adaptor-refund".into(),
            detail: format!("stalled swap refunded; state {:?}", r.state),
        }))
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
    ///   scheduler re-broadcasts it; the single-key CLTV refund RBF-bumps the fee
    ///   (deterministic re-sign), and the cooperative MuSig2 redeem — which can't
    ///   be RBF'd — is CPFP-bumped with a self-funded child (v2+; see
    ///   [`Self::adaptor_keep_moving`]).
    fn adaptor_tick_one(&self, rec: &AdaptorSwapRecord) -> Result<Option<TickEvent>> {
        use AdaptorState::*;
        let ev = |action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: action.into(),
                detail,
            }))
        };
        // C8 (v2 twin of the v1 pre-funding timeout-abort): a handshake
        // stalled strictly BEFORE any funding can exist (`Signed` is where
        // funding starts, so it is excluded — a counterparty's funding may be
        // in flight there) past the timeout is aborted LOCALLY, with NO
        // envelope: the counterparty's own clock clears their side
        // (minimal-relay-traffic decision, 2026-07-03). Nothing is locked
        // on-chain, so this loses no money. The funding-pointer guard is
        // belt-and-braces; `created_at == 0` (pre-timestamp records) must
        // not be judged infinitely old.
        if matches!(rec.state, Created | Accepted | NoncesExchanged)
            && rec.funding_a_txid.is_none()
            && rec.funding_b_txid.is_none()
            && rec.created_at > 0
            && local_now().saturating_sub(rec.created_at) >= PRE_FUNDING_TIMEOUT_SECS
        {
            let mut dead = rec.clone();
            dead.state = Aborted;
            self.store.put_adaptor(&dead)?;
            let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
            return ev(
                "abort-timeout",
                format!("no funding within {PRE_FUNDING_TIMEOUT_SECS}s; aborted"),
            );
        }
        // Signed: drive redeem/refund. RedeemedB/Completed/Refunded: keep the
        // broadcast spend moving until it confirms. Anything else is inert.
        //
        // rc6 #2 NOTE: v2 funding intentionally has NO tick retry (yet). Unlike
        // v1, a failed v2 funding leaves an HONEST, recoverable `Accepted`
        // (funding=None) — resumable by a relay re-drive or a manual `adaptor_fund`
        // RPC — so it is a liveness gap, not a stranding bug. A correct tick retry
        // needs the counterparty identity threaded into the tick (not on
        // `AdaptorSwapRecord` today) to relay `funding_ready`, plus a locate-first
        // idempotency guard on the Taproot funding (today's is pointer-based).
        // Deferred to a focused follow-up.
        if !matches!(rec.state, Signed | RedeemedB | Completed | Refunded) {
            // Not yet Signed (e.g. funded, then the handshake stalled). We can't
            // drive the redeem, but we MUST still auto-refund our own funded leg
            // once its timelock matures — unattended-recovery invariant (§9.5).
            return self.adaptor_refund_if_due(rec);
        }
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(rec)?;

        // Rescue / chain-watch rediscovery (#54): the v2 tick is otherwise
        // pointer-based (pointers arrive via `funding_ready` messages). A node
        // restored from its seed alone — snapshot taken at `Signed`, before any
        // funding — has NEITHER pointer, so without this it could not even refund
        // a leg it already funded (stranded funds). While `Signed`, rediscover any
        // missing pointer by its derived leg script before deriving `both_funded`.
        // Idempotent: `find_funding` matches script+amount and a missing output
        // just leaves the pointer `None`; once found it is persisted and skipped.
        let mut owned = rec.clone();
        if owned.state == Signed {
            if owned.funding_a_txid.is_none() {
                let spk_a = p.leg_a(&secp)?.script_pubkey(&secp)?;
                if let Some((op, info)) = self.backend(&owned.chain_a)?.find_funding(&spk_a)? {
                    if info.value_sat == owned.amount_a {
                        owned.funding_a_txid = Some(op.txid.to_string());
                        owned.funding_a_vout = Some(op.vout);
                        self.store.put_adaptor(&owned)?;
                    }
                }
            }
            if owned.funding_b_txid.is_none() {
                let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                if let Some((op, info)) = self.backend(&owned.chain_b)?.find_funding(&spk_b)? {
                    if info.value_sat == owned.amount_b {
                        owned.funding_b_txid = Some(op.txid.to_string());
                        owned.funding_b_vout = Some(op.vout);
                        self.store.put_adaptor(&owned)?;
                    }
                }
            }
        }
        let rec = &owned;

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
                // Nurse our own (leg-A) funding while unconfirmed — CPFP it up to
                // market if it went out under-priced.
                if rec.state == Signed {
                    if let Some(ev) = self.maybe_bump_funding_v2(rec, "a")? {
                        return Ok(Some(ev));
                    }
                }
                // Redeem leg B (reveal t) once Bob's leg-B funding is n_b deep
                // and we are still before T2. The depth gate is the reveal's
                // reorg safety: never publish t against a funding that can still
                // reorg away.
                if rec.state == Signed && both_funded {
                    let backend_b = self.backend(&rec.chain_b)?;
                    let net = rec.chain_b.network;
                    let (_, reveal_margin, _) = action_margins(net);
                    let now = deadline_clock(net, local_now(), backend_b.tip_median_time()?);
                    if action_safe(now, reveal_margin, rec.t2) {
                        let op_b = outpoint(&rec.funding_b_txid, rec.funding_b_vout)?;
                        let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                        match backend_b.get_txout(&op_b, &spk_b)? {
                            // §6.1 parity with v1: verify the located output is the
                            // leg-B P2TR we reconstructed AND pays exactly amount_b,
                            // AND is n_b deep — before revealing t. (The pre-signed
                            // key-path sighash already binds script+amount, so a
                            // mismatch is self-protecting, but check explicitly so a
                            // mis-funding aborts cleanly instead of looping — and so
                            // `t` is never revealed against a wrong output.)
                            Some(txout)
                                if txout.confirmations >= u64::from(rec.n_b.max(1))
                                    && txout.script_pubkey_hex == hex::encode(spk_b.as_bytes())
                                    && txout.value_sat == rec.amount_b =>
                            {
                                let r = self.adaptor_redeem(&rec.swap_id)?;
                                return ev(
                                    "adaptor-redeem-b",
                                    format!("revealed t; state {:?}", r.state),
                                );
                            }
                            Some(_) => return Ok(None), // shallow / wrong script or value — wait
                            None => return Ok(None), // not yet funded/visible — wait (T1 protects leg A)
                        }
                    }
                }
                // Else reclaim leg A after T1 if it is still unspent. Only while
                // Signed: once we've revealed t (RedeemedB/Completed) leg A is
                // the counterparty's to claim — v1 parity (it does not reclaim
                // after redeeming either).
                if rec.state == Signed && rec.funding_a_txid.is_some() {
                    let mtp_a = self.backend(&rec.chain_a)?.tip_median_time_min()?; // M6 refund readiness
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
                // Rescue self-heal (#54): a taker restored at `Signed` has no
                // pre-built leg B (it is built by `adaptor_fund`, after the Signed
                // snapshot). If rediscovery above found no leg-B output either, we
                // never funded — rebuild it now so the two-phase broadcast below can
                // proceed. Idempotent: `adaptor_build_leg_b` re-adopts a recorded
                // pointer or an existing on-chain output, never double-funds. Gated
                // on the §7.4 fund deadline so we don't reserve wallet inputs for a
                // leg we could no longer safely broadcast.
                if rec.state == Signed
                    && !rec.funding_b_broadcast
                    && rec.funding_b_tx_hex.is_none()
                    && rec.funding_b_txid.is_none()
                {
                    let net = rec.chain_b.network;
                    let (fund_margin, _, _) = action_margins(net);
                    let fundable = match self.backend(&rec.chain_b)?.tip_median_time() {
                        Ok(mtp) => {
                            action_safe(deadline_clock(net, local_now(), mtp), fund_margin, rec.t2)
                        }
                        Err(_) => true, // clock hiccup: don't block a still-fundable swap
                    };
                    if fundable {
                        self.adaptor_build_leg_b(&rec.swap_id)?;
                        return ev("adaptor-build-b", "rebuilt leg B (rescue self-heal)".into());
                    }
                }
                // CRITICAL two-phase broadcast (spec v2 §7): once the swap is
                // `Signed` (we hold a verified σ_A) AND leg A is verified on-chain
                // n_a-deep, broadcast our pre-built leg B. Until BOTH hold, leg B
                // stays unbroadcast — the taker never commits leg B before it is
                // certain it can claim leg A.
                if rec.state == Signed && !rec.funding_b_broadcast {
                    if let Some(hex) = rec.funding_b_tx_hex.as_deref() {
                        // §7.4 fund deadline (mirrors the manual `adaptor_fund`
                        // gate): never broadcast leg B within `fund_margin` of T2.
                        // This is the load-bearing gate for a RESCUED taker — a
                        // Signed snapshot re-adopted long after negotiation must
                        // NOT auto-commit leg B into a window too tight for Alice
                        // to redeem: leg B was never broadcast, so skipping costs
                        // nothing (Alice refunds leg A), while funding late risks
                        // Bob's funds. Conservative: an unreadable clock does not
                        // gate (a transient node hiccup must not strand a swap).
                        let net = rec.chain_b.network;
                        let (fund_margin, _, _) = action_margins(net);
                        if let Ok(mtp) = self.backend(&rec.chain_b)?.tip_median_time() {
                            let now = deadline_clock(net, local_now(), mtp);
                            if !action_safe(now, fund_margin, rec.t2) {
                                // Too late to fund leg B safely — and the clock
                                // only moves forward, so it stays too late: this
                                // swap can never proceed. Instead of idling here
                                // forever with the built funding's inputs
                                // reserved, release them and go terminal. Guarded
                                // on the tx being provably off-network (a leg B
                                // broadcast by a pre-rescue incarnation must keep
                                // driving the claim path instead).
                                // Terminal only once the release succeeds — a
                                // failed release (locked seed on a nodeless
                                // coin) retries next tick; the record must not
                                // go terminal with inputs still reserved.
                                if self.adaptor_leg_b_uncommitted(rec)
                                    && self.adaptor_cancel_built_leg_b(rec)
                                {
                                    let mut dead = rec.clone();
                                    dead.state = Aborted;
                                    self.store.put_adaptor(&dead)?;
                                    let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
                                    return ev(
                                        "abort-fund-deadline",
                                        "past the leg-B fund deadline (§7.4) with leg B \
                                         unbroadcast; reserved inputs released, aborted"
                                            .into(),
                                    );
                                }
                                return Ok(None);
                            }
                        }
                        if !self.adaptor_leg_a_confirmed(rec)? {
                            return Ok(None); // wait for leg A before committing leg B
                        }
                        let tx: bitcoin::Transaction =
                            bitcoin::consensus::encode::deserialize(&hex::decode(hex)?)
                                .context("corrupt funding_b_tx_hex")?;
                        let txid = self.backend(&rec.chain_b)?.broadcast(&tx)?;
                        let mut updated = rec.clone();
                        updated.funding_b_broadcast = true;
                        self.store.put_adaptor(&updated)?;
                        return Ok(Some(TickEvent {
                            swap_id: rec.swap_id.clone(),
                            action: "adaptor-fund-b".into(),
                            detail: format!("broadcast leg B {txid} (σ_A held, leg A confirmed)"),
                        }));
                    }
                }
                // Nurse our own (leg-B) funding while unconfirmed.
                if rec.state == Signed {
                    if let Some(ev) = self.maybe_bump_funding_v2(rec, "b")? {
                        return Ok(Some(ev));
                    }
                }
                // Claim leg A as soon as Alice's leg-B redeem reveals t. No
                // depth gate: once t is on chain it is valid even if that spend
                // later reorgs, so racing to redeem A is always correct — but
                // only while inside Bob's §7.4 redeem deadline (T1 − 1h, margin
                // 0 on regtest); past it the redeem races Alice's refund and
                // (being unbumpable) cannot win, so leave it (leg B is gone).
                if rec.state == Signed && both_funded {
                    let net = rec.chain_a.network;
                    let (_, _, redeem_a_margin) = action_margins(net);
                    let now = deadline_clock(
                        net,
                        local_now(),
                        self.backend(&rec.chain_a)?.tip_median_time()?,
                    );
                    if action_safe(now, redeem_a_margin, rec.t1) {
                        let op_b = outpoint(&rec.funding_b_txid, rec.funding_b_vout)?;
                        let spk_b = p.leg_b(&secp)?.script_pubkey(&secp)?;
                        let backend_b = self.backend(&rec.chain_b)?;
                        let from_b = self.funding_scan_from_height(
                            &backend_b,
                            rec.funding_b_txid.as_deref().context("no leg-B funding")?,
                            &spk_b,
                        )?;
                        if backend_b
                            .find_spend_witness(&op_b, &spk_b, from_b)?
                            .is_some()
                        {
                            let r = self.adaptor_redeem(&rec.swap_id)?;
                            return ev(
                                "adaptor-redeem-a",
                                format!("extracted t; state {:?}", r.state),
                            );
                        }
                    }
                }
                // Else reclaim leg B after T2 if still unspent (only while Signed,
                // i.e. before we've claimed leg A).
                if rec.state == Signed && rec.funding_b_txid.is_some() {
                    let mtp_b = self.backend(&rec.chain_b)?.tip_median_time_min()?; // M6 refund readiness
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
    /// - A cooperative **redeem** can't be RBF'd (its fee is sealed into the
    ///   pre-signed adaptor signature), so it is re-anchored in the mempool and
    ///   **CPFP-bumped** with a self-funded child spending its own sweep output
    ///   ([`Self::adaptor_cpfp_bump`], v2+). Unilateral, byte-identical redeem,
    ///   no protocol change; see spec/protocol-v2.md.
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
        let tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
                .context("corrupt final_tx_hex")?;
        let spk = tx.output[0].script_pubkey.clone();
        // Finality gate (#101): MIN over a responder quorum, never the
        // display max — a single lying view must not stop this nurse or
        // fake a Completed.
        let confs = backend.tx_confirmations_min(txid, Some(&spk))?;
        if confs >= u64::from(target_confs.max(1)) {
            // Confirmed deep enough — the spend is final.
            if complete_on_depth && rec.state != AdaptorState::Completed {
                let mut updated = rec.clone();
                updated.state = AdaptorState::Completed;
                self.store.put_adaptor(&updated)?;
                let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
                return Ok(Some(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "adaptor-completed".into(),
                    detail: txid.to_string(),
                }));
            }
            return Ok(None);
        }
        // Mined but shallow: a confirmed spend can't be RBF'd or usefully
        // CPFP-accelerated — just wait for depth (no per-tick churn).
        if confs >= 1 {
            return Ok(None);
        }
        // Step 0: act at most once per block (block-driven cadence).
        let tip_height = backend.tip_height()?;
        if tip_height == rec.last_action_height {
            return Ok(None);
        }
        if is_refund {
            return self.adaptor_bump_refund(rec, &backend, tx_hex, tip_height);
        }
        // Cooperative redeem: its committed fee can't be RBF'd (it's baked into
        // the pre-signed adaptor signature), so re-anchor the parent only if it
        // was evicted, and CPFP-bump it toward market with a self-funded child
        // spending its own wallet-owned sweep output (v2+). Unilateral — the
        // signed redeem stays byte-identical, no fresh MuSig2 round. Once per
        // block; in steady state an unchanged market makes the child a no-op
        // (CPFP returns None) or a self-rejecting same-fee replacement.
        let parent_txid = tx.compute_txid().to_string();
        if !backend.is_in_mempool(&parent_txid)? {
            backend.broadcast(&tx)?;
        }
        let amount = if chain.coin_id == rec.chain_b.coin_id {
            rec.amount_b
        } else {
            rec.amount_a
        };
        // Deadline-aware CPFP target (#48): initiator leg-B redeem confirms by T2;
        // participant leg-A redeem by T1 − redeem-a margin. Escalate as it nears.
        let deadline = if chain.coin_id == rec.chain_b.coin_id {
            u64::from(rec.t2)
        } else {
            u64::from(rec.t1).saturating_sub(action_margins(chain.network).2)
        };
        let now = deadline_clock(chain.network, local_now(), backend.tip_median_time()?);
        let (conf_target, conservative) = redeem_conf_target(deadline.saturating_sub(now));
        match self.adaptor_cpfp_bump(&backend, &tx, amount, conf_target, conservative) {
            Ok(Some(child)) => {
                let mut updated = rec.clone();
                updated.last_action_height = tip_height;
                self.store.put_adaptor(&updated)?;
                Ok(Some(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "adaptor-cpfp".into(),
                    detail: format!("{parent_txid} (redeem) bumped by child {child}"),
                }))
            }
            // No bump warranted (parent already pays the target / output too
            // small) — silent until the next block or a market rise.
            Ok(None) => Ok(None),
            // CPFP unavailable (e.g. redeem not swept to a wallet address). The
            // parent is re-anchored if it was evicted; surface without failing.
            Err(e) => Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "adaptor-rebroadcast".into(),
                detail: format!("{parent_txid} (CPFP unavailable: {e:#})"),
            })),
        }
    }

    /// CPFP-bump a stuck cooperative redeem (v2+): broadcast a self-funded child
    /// spending the redeem's own (wallet-owned sweep) output at a fee that lifts
    /// the package to the live feerate. Unilateral — the claimer owns the sweep
    /// output, so the signed redeem is untouched and the counterparty is not
    /// involved. The child RBF-signals, so a later tick at a higher feerate
    /// replaces it (escalation). `None` when no bump is warranted (parent already
    /// pays the target, or the output is too small to fund a child).
    fn adaptor_cpfp_bump(
        &self,
        backend: &MultiBackend,
        parent: &bitcoin::Transaction,
        amount: u64,
        conf_target: u16,
        conservative: bool,
    ) -> Result<Option<bitcoin::Txid>> {
        let parent_out = &parent.output[0];
        let parent_value = parent_out.value.to_sat();
        let parent_fee = amount.saturating_sub(parent_value);
        // Redeem CPFP is a claim spend: chase market bounded by the value at risk
        // (the leg amount), NOT the funding fee ceiling — near the redeem deadline
        // a spike above 500 sat/vB must still be payable (spec v2 §8). The market
        // estimate is deadline-aware (#48): `(conf_target, conservative)` escalate
        // as the redeem timelock nears, computed by the caller. The CPFP child is
        // funded out of the sweep output, so the package fee is also implicitly
        // hard-capped by `parent_value` (the dust check below).
        // sat/kvB (native estimator resolution). A CPFP child is not a BIP125
        // replacement, so no incremental-relay floor applies.
        let target_kvb = self.fee_bump.claim_feerate_kvb(
            backend.fee_rate_for_kvb(conf_target, conservative)?,
            amount,
            crate::taproot::KEYPATH_REDEEM_VSIZE,
        );
        let Some(child_fee) =
            cpfp_child_fee_kvb(parent_fee, crate::taproot::KEYPATH_REDEEM_VSIZE, target_kvb)
        else {
            return Ok(None); // parent already clears the target unaided
        };
        // CPFP budget is the sweep output value. If the child can't be funded to
        // the desired target, surface it — a silent under-bump near a deadline
        // otherwise reads as "we did everything" (#48 open question).
        let target_vb = target_kvb as f64 / 1000.0;
        let Some(child_value) = parent_value.checked_sub(child_fee) else {
            eprintln!(
                "warning: v2 redeem CPFP budget-limited (parent {}): sweep output {parent_value} sat \
                 cannot fund a child to reach {target_vb:.3} sat/vB (need {child_fee} sat)",
                parent.compute_txid()
            );
            return Ok(None); // output can't cover the child fee
        };
        if child_value <= DUST_LIMIT_SAT {
            eprintln!(
                "warning: v2 redeem CPFP budget-limited (parent {}): child would be dust at \
                 {target_vb:.3} sat/vB (sweep output {parent_value} sat)",
                parent.compute_txid()
            );
            return Ok(None);
        }
        let dest = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        let child = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: OutPoint {
                    txid: parent.compute_txid(),
                    vout: 0,
                },
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::from_consensus(HTLC_SPEND_SEQUENCE),
                witness: bitcoin::Witness::default(),
            }],
            output: vec![bitcoin::TxOut {
                value: bitcoin::Amount::from_sat(child_value),
                script_pubkey: dest,
            }],
        };
        let txid = backend.wallet_sign_send(&child, parent_value, &parent_out.script_pubkey)?;
        Ok(Some(txid))
    }

    /// v2 funding nurse: CPFP-bump our own unconfirmed funding (`leg` = "a"/"b")
    /// by spending its **change** output, leaving the funding outpoint UNCHANGED.
    /// RBF is forbidden here — the outpoint feeds the 2-of-2 MuSig2 adaptor sigs
    /// already exchanged with the counterparty, so changing the txid would
    /// invalidate them (re-doing the MuSig2 round needs the counterparty). Mirrors
    /// the redeem-side [`Self::adaptor_cpfp_bump`]. Liveness only: no change output
    /// (exact-UTXO funding) → can't CPFP → stall → refund. Returns an event only
    /// when it acts; no record change (the funding outpoint and refund stay valid).
    fn maybe_bump_funding_v2(
        &self,
        rec: &AdaptorSwapRecord,
        leg: &str,
    ) -> Result<Option<TickEvent>> {
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let p = self.adaptor_params(rec)?;
        let (chain, leg_obj, txid, vout) = match leg {
            "a" => (
                &rec.chain_a,
                p.leg_a(&secp)?,
                rec.funding_a_txid.as_deref(),
                rec.funding_a_vout,
            ),
            _ => (
                &rec.chain_b,
                p.leg_b(&secp)?,
                rec.funding_b_txid.as_deref(),
                rec.funding_b_vout,
            ),
        };
        let (Some(txid), Some(_vout)) = (txid, vout) else {
            return Ok(None); // our leg not funded yet
        };
        let leg_spk = leg_obj.script_pubkey(&secp)?;
        let backend = self.backend(chain)?;
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid)?,
            vout: _vout,
        };
        // Only nurse while the funding is unconfirmed.
        match backend.get_txout(&outpoint, &leg_spk)? {
            Some(txout) if txout.confirmations == 0 => {}
            _ => return Ok(None),
        }
        // Deadline gate against this leg's OWN refund timelock (leg A → T1, leg B
        // → T2) with the fund margin; past it, let it stall → refund.
        let deadline = if leg == "a" { rec.t1 } else { rec.t2 };
        let net = chain.network;
        let (fund_margin, _, _) = action_margins(net);
        let now = deadline_clock(net, local_now(), backend.tip_median_time()?);
        if !action_safe(now, fund_margin, deadline) {
            return Ok(None);
        }
        // The funding's change output is the CPFP budget. No change output
        // (exact-UTXO funding) → can't CPFP → stall → refund (acceptable).
        let Some((change_vout, change_value, change_spk)) =
            backend.wallet_change_output(txid, &leg_spk)?
        else {
            return Ok(None);
        };
        // If a CPFP child already spends this change output (it's spent in the
        // mempool), we have bumped this funding once already — don't churn a fresh
        // child (burning a new address + a guaranteed RBF-reject) every tick. One
        // CPFP to current market is the liveness win; a further spike past it stalls
        // → refund. If that child is later evicted, the change becomes spendable
        // again and the next tick re-bumps.
        let change_outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid)?,
            vout: change_vout,
        };
        if backend.get_txout(&change_outpoint, &change_spk)?.is_none() {
            return Ok(None);
        }
        // sat/kvB throughout (native estimator resolution): a CPFP child needs no
        // BIP125 increment (it doesn't replace the parent), so unlike the v1 RBF
        // funding nurse there is no Rule-4 floor here — just chase market, bounded
        // by the ceiling and the funds-gate reservation.
        let (parent_fee, parent_vsize) = backend.wallet_tx_fee_vsize(txid)?;
        let old_feerate_kvb = parent_fee.saturating_mul(1000) / parent_vsize.max(1);
        let market_kvb = backend.fee_rate_for_kvb(backend.funding_conf_target(), false)?;
        let target_kvb = market_kvb
            .min(self.fee_bump.max_feerate_sat_vb.saturating_mul(1000))
            .min(
                self.fee_bump
                    .funding
                    .reservation_mult
                    .saturating_mul(old_feerate_kvb),
            );
        if target_kvb <= old_feerate_kvb {
            return Ok(None);
        }
        let Some(child_fee) = cpfp_child_fee_kvb(parent_fee, parent_vsize, target_kvb) else {
            return Ok(None); // parent already clears the target
        };
        let Some(child_value) = change_value.checked_sub(child_fee) else {
            return Ok(None); // change can't cover the child fee
        };
        if child_value <= DUST_LIMIT_SAT {
            return Ok(None);
        }
        // A child spending the funding's change output → a fresh wallet address.
        let dest = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        let child = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: OutPoint {
                    txid: bitcoin::Txid::from_str(txid)?,
                    vout: change_vout,
                },
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::from_consensus(HTLC_SPEND_SEQUENCE),
                witness: bitcoin::Witness::default(),
            }],
            output: vec![bitcoin::TxOut {
                value: bitcoin::Amount::from_sat(child_value),
                script_pubkey: dest,
            }],
        };
        // A recoverable signing/broadcast failure is a graceful no-op (liveness).
        let child_txid = match backend.wallet_sign_send(&child, change_value, &change_spk) {
            Ok(t) => t,
            Err(e) => {
                return Ok(Some(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "funding-cpfp-skipped".into(),
                    detail: format!("leg {leg}: {e:#}"),
                }));
            }
        };
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "funding-cpfp-bump".into(),
            detail: format!(
                "leg {leg}: {child_txid} (package -> {:.3} sat/vB)",
                target_kvb as f64 / 1000.0
            ),
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
        tip_height: u64,
    ) -> Result<Option<TickEvent>> {
        let old_tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(old_tx_hex)?)
                .context("corrupt refund tx hex")?;
        let old_txid = old_tx.compute_txid().to_string();
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
        let old_feerate_kvb = old_fee.saturating_mul(1000) / REFUND_TX_VSIZE.max(1);
        // The v2 refund is a single-key RBF spend — same unified strategy as the
        // v1 redeem/refund (was a market-blind escalate()): market-tracking,
        // value-capped target, bump only when it clears BIP125 Rule 4, else
        // re-anchor the same tx only if it was evicted. All sat/kvB (native
        // estimator resolution).
        let market_kvb = backend.fee_rate_for_kvb(6, false)?;
        let target_kvb = self
            .fee_bump
            .claim_feerate_kvb(market_kvb, amount, REFUND_TX_VSIZE);
        let incr_kvb = backend.incremental_relay_feerate_kvb()?;
        // BIP125 Rule 4 also constrains the ABSOLUTE fee: the replacement must
        // pay at least `incr * vsize` MORE than the tx it evicts, else the node
        // rejects the RBF (-26). Compute both fees in sat (kvB × vsize, rounded
        // up) and floor new_fee to old_fee + incr·vsize. The gate below only asks
        // whether the market rose above what we pay (mirrors `maybe_bump`); the
        // increment lives in the absolute floor, not the gate.
        let new_fee = target_kvb
            .saturating_mul(REFUND_TX_VSIZE)
            .div_ceil(1000)
            .max(old_fee.saturating_add(incr_kvb.saturating_mul(REFUND_TX_VSIZE).div_ceil(1000)));
        let dustless = amount > new_fee + DUST_LIMIT_SAT;
        if target_kvb <= old_feerate_kvb || !dustless {
            if backend.is_in_mempool(&old_txid)? {
                return Ok(None);
            }
            let txid = backend.broadcast(&old_tx)?;
            return Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "adaptor-rebroadcast".into(),
                detail: txid.to_string(),
            }));
        }
        let outpoint = old_tx.input[0].previous_output;
        let refund_kp = v2_refund_key(&seed, rec, coin_of(chain)?)?.keypair(&secp);
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
        updated.last_action_height = tip_height;
        self.store.put_adaptor(&updated)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "adaptor-fee-bump".into(),
            detail: format!(
                "{txid} (refund fee {old_fee} -> {new_fee} sat, {:.3} -> {:.3} sat/vB)",
                old_feerate_kvb as f64 / 1000.0,
                target_kvb as f64 / 1000.0
            ),
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
                ensure!(
                    body.wire == crate::WIRE_V1,
                    "peer speaks pact-htlc-v1 wire v{}, this build speaks v{} — both sides must run compatible releases",
                    body.wire,
                    crate::WIRE_V1
                );
                parse_pubkey(&body.bob_redeem_pubkey_a, "bob redeem A")?;
                parse_pubkey(&body.bob_refund_pubkey_b, "bob refund B")?;
                // The taker's advisory depths (wire v2): display only, but an
                // out-of-band value is a foreseeable liveness stall — refuse.
                ensure_confs_in_bounds(&rec.chain_a, body.n_a, "advisory N_A")?;
                ensure_confs_in_bounds(&rec.chain_b, body.n_b, "advisory N_B")?;
                rec.their_n_a = Some(body.n_a);
                rec.their_n_b = Some(body.n_b);
                rec.bob_redeem_pubkey_a = Some(body.bob_redeem_pubkey_a);
                rec.bob_refund_pubkey_b = Some(body.bob_refund_pubkey_b);
                rec.state = State::Accepted;
                // Both HTLCs must now be constructible.
                self.swap_params(&rec)?;
            }
            "funded" => {
                // The `funded` message is a pointer HINT now, not the sole
                // authority. We record where the counterparty's HTLC funding is
                // (so tick() can skip the address scan) and advance synchronously
                // *iff* the output is already visible, verifies, and is confirmed
                // — the low-latency happy path. A not-yet-confirmed or missing
                // funding is NOT an error here, so it no longer exhausts relay
                // retries and drops the message; tick() (chain-watched) advances
                // it later, and also rediscovers the funding by its derivable
                // script if this message is ever lost.
                let body: FundedBody = serde_json::from_value(envelope.body.clone())
                    .context("malformed funded body")?;
                let outpoint = OutPoint {
                    txid: bitcoin::Txid::from_str(&body.txid).context("funded: bad txid")?,
                    vout: body.vout,
                };
                let params = self.swap_params(&rec)?;
                let (chain, htlc, amount, min_conf) = match body.chain.as_str() {
                    "a" => (&rec.chain_a, params.htlc_a()?, rec.amount_a, rec.n_a),
                    "b" => (&rec.chain_b, params.htlc_b()?, rec.amount_b, rec.n_b),
                    other => bail!("funded: unknown chain {other:?}"),
                };
                // Record the pointer regardless.
                match body.chain.as_str() {
                    "a" => {
                        rec.htlc_a_txid = Some(body.txid.clone());
                        rec.htlc_a_vout = Some(body.vout);
                    }
                    _ => {
                        rec.htlc_b_txid = Some(body.txid.clone());
                        rec.htlc_b_vout = Some(body.vout);
                    }
                }
                // §6.1: the message is a pointer, not a proof — verify the output
                // against the locally reconstructed script before advancing.
                let backend = self.backend(chain)?;
                let htlc_spk = htlc.script_pubkey();
                if let Some(txout) = backend.get_txout(&outpoint, &htlc_spk)? {
                    if txout.script_pubkey_hex == hex::encode(htlc_spk.as_bytes())
                        && txout.value_sat == amount
                        && txout.confirmations >= u64::from(min_conf)
                    {
                        match body.chain.as_str() {
                            "a" => rec.state = State::FundedA,
                            _ => {
                                rec.htlc_b_height =
                                    Some(backend.tip_height()?.saturating_sub(txout.confirmations));
                                rec.state = State::FundedB;
                            }
                        }
                    }
                }
            }
            // Advisory courtesy (spec §8.6). The preimage is authoritatively
            // extracted from the on-chain redeem witness (`extract_preimage`),
            // never trusted from a message — so this implementation neither sends
            // nor consumes it. Accept and ignore for interop: a third-party client
            // MAY still send it, and dropping it is safe (we learn `s` from chain).
            "redeemed" => {}
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
        // Initiator snapshots once at accept (#54); tombstone a pre-funding abort.
        if envelope.msg_type == "accept" {
            let _ = self.snapshot_v1(&rec);
        } else if rec.state == State::Aborted {
            let _ = self.tombstone_swap(&rec.swap_id);
        }
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
            // §7.4: Bob MUST NOT fund after `T2 − 3h` (margin 0 on regtest).
            // Funding later shrinks Alice's redeem window to nothing and just
            // wastes fees — she would abort and both would refund.
            let net = rec.chain_b.network;
            let (fund_margin, _, _) = action_margins(net);
            let mtp = self.backend(&rec.chain_b)?.tip_median_time()?;
            let now = deadline_clock(net, local_now(), mtp);
            ensure!(
                action_safe(now, fund_margin, rec.t2),
                "too late to fund: now {now} is within {}h of T2 {} (spec §7.4 fund deadline)",
                fund_margin / 3600,
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

        // Idempotency / retry-safety: if this leg is ALREADY funded on chain — a
        // prior attempt's broadcast we never persisted (a crash, or a fund retry
        // after a transient signing failure like a locked wallet → RPC -13) —
        // adopt that output instead of broadcasting a SECOND funding (which would
        // double-fund, real loss). `locate_funding` matches the HTLC by
        // script+amount (scantxoutset / stored pointer); confirmed-only, so the
        // residual double-fund window is just a crash in the seconds between
        // broadcast and the `put` below, before the tx is mined.
        let (txid, vout) = match self.locate_funding(&rec, leg)? {
            Some((op, _)) => (op.txid.to_string(), op.vout),
            None => {
                let address = htlc.address(backend.params())?;
                let txid = backend.wallet_send(
                    &address,
                    amount,
                    SendFee::Target(backend.funding_conf_target()),
                )?;
                let vout =
                    backend.find_vout(&txid, &hex::encode(htlc.script_pubkey().as_bytes()))?;
                (txid, vout)
            }
        };

        // L2: persist the funding pointer IMMEDIATELY after the broadcast,
        // before the refund-building RPCs below. A crash between `wallet_send`
        // and this write would otherwise leave the funding on-chain with no
        // local record (recoverable only by a chain re-scan / seed rebuild);
        // persisting first shrinks that window to this single `put`.
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

        // §6.3: sign the refund NOW and persist it too, so a scheduler can
        // reclaim funds after T with no keys re-derived and no human present.
        // A separate write: if it fails, the pointer above is already durable
        // (refund() rebuilds the refund from the seed as a fallback).
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(&txid)?,
            vout,
        };
        let seed = self.store.seed()?;
        let key = v1_swap_key(&seed, &rec, coin_of(&chain)?)?;
        let destination = backend
            .params()
            .parse_address(&backend.wallet_new_address()?)?;
        // A1: price the initial spend at the unified target (market, capped by
        // the value at risk + ceiling), not raw market floored at 1000 — so the
        // first broadcast is competitive and the nurse is a rare safety net.
        let fee = spend_fee_sat(
            self.fee_bump
                .claim_feerate(backend.fee_rate_sat_per_vb()?, amount, REFUND_TX_VSIZE),
            REFUND_TX_VSIZE,
        );
        let refund_tx = build_refund_tx(&htlc, outpoint, amount, destination, fee, &key)?;
        rec.refund_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&refund_tx));
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

                // §7.4 reveal deadline: Alice MUST NOT broadcast her redeem
                // after `T2 − 2h` (margin 0 on regtest). A redeem that lingers
                // in the mempool past T2 reveals s while Bob can already refund
                // chain B — he could then take *both* legs. Past the deadline we
                // refuse and fall back to the T1 refund of our own leg.
                let net = rec.chain_b.network;
                let (_, reveal_margin, _) = action_margins(net);
                let mtp = backend.tip_median_time()?;
                let now = deadline_clock(net, local_now(), mtp);
                ensure!(
                    action_safe(now, reveal_margin, rec.t2),
                    "REFUSING to redeem: now {now} is within {}h of T2 {} — \
                     revealing s now risks losing both legs; wait for the T1 refund (spec §7.4)",
                    reveal_margin / 3600,
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

                let preimage = seed.preimage(
                    rec.swap_index
                        .context("initiator record missing its swap index")?,
                )?;
                let key = v1_swap_key(&seed, &rec, coin_of(&rec.chain_b)?)?;
                let destination = backend
                    .params()
                    .parse_address(&backend.wallet_new_address()?)?;
                // A1: initial spend priced at the unified value-capped target.
                let fee = spend_fee_sat(
                    self.fee_bump.claim_feerate(
                        backend.fee_rate_sat_per_vb()?,
                        rec.amount_b,
                        REDEEM_TX_VSIZE,
                    ),
                    REDEEM_TX_VSIZE,
                );
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
                // §7.4: Bob MUST redeem chain A before `T1 − 1h` (margin 0 on
                // regtest) — past that, his redeem could race Alice's T1 refund.
                let net = rec.chain_a.network;
                let (_, _, redeem_a_margin) = action_margins(net);
                let backend_a = self.backend(&rec.chain_a)?;
                let mtp = backend_a.tip_median_time()?;
                let now = deadline_clock(net, local_now(), mtp);
                ensure!(
                    action_safe(now, redeem_a_margin, rec.t1),
                    "now {now} is within {}h of T1 {} — redeem would race Alice's refund (spec §7.4)",
                    redeem_a_margin / 3600,
                    rec.t1
                );

                let htlc = params.htlc_a()?;
                let key = v1_swap_key(&seed, &rec, coin_of(&rec.chain_a)?)?;
                let destination = backend_a
                    .params()
                    .parse_address(&backend_a.wallet_new_address()?)?;
                // A1: initial spend priced at the unified value-capped target.
                let fee = spend_fee_sat(
                    self.fee_bump.claim_feerate(
                        backend_a.fee_rate_sat_per_vb()?,
                        rec.amount_a,
                        REDEEM_TX_VSIZE,
                    ),
                    REDEEM_TX_VSIZE,
                );
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
        if rec.state == State::Completed {
            let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
        }
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
        // Refund readiness uses the *least*-advanced backend MTP (M6): only
        // broadcast once every backend — including the node that will mine —
        // will accept the locktime, avoiding a `non-final` rejection.
        let mtp = backend.tip_median_time_min()?;
        ensure!(
            mtp >= u64::from(locktime),
            "refund not yet valid: chain MTP {mtp} < T {locktime} (BIP113 lag is normal — retry later)"
        );
        let htlc_spk = htlc.script_pubkey();
        ensure!(
            backend.get_txout(&outpoint, &htlc_spk)?.is_some(),
            "HTLC already spent — check whether the counterparty redeemed (status/recv)"
        );
        // M7: never broadcast a refund that would race a counterparty redeem we
        // can already see. `get_txout` above (gettxout include_mempool, and
        // MultiBackend treats any "spent" view as spent) is the primary guard;
        // this scans the mempool + tip explicitly as a cross-backend backstop
        // (e.g. an Electrum view whose listunspent lags a mempool spend). If a
        // spend is visible, the swap is resolving on its own — leave it.
        if backend
            .find_spend_witness(&outpoint, &htlc_spk, backend.tip_height()?)?
            .is_some()
        {
            bail!(
                "HTLC already has a spend in the mempool (counterparty redeem?) — \
                 not broadcasting a competing refund (status/recv)"
            );
        }

        // Prefer the refund signed at funding time (§6.3); rebuilding from
        // seed + record is the recovery fallback for pre-§6.3 records.
        let tx = match &rec.refund_tx_hex {
            Some(tx_hex) => bitcoin::consensus::encode::deserialize::<bitcoin::Transaction>(
                &hex::decode(tx_hex).context("corrupt refund_tx_hex")?,
            )
            .context("corrupt refund_tx_hex")?,
            None => {
                let key = v1_swap_key(&seed, &rec, coin_of(&chain)?)?;
                let destination = backend
                    .params()
                    .parse_address(&backend.wallet_new_address()?)?;
                // A1: initial spend priced at the unified value-capped target.
                let fee = spend_fee_sat(
                    self.fee_bump.claim_feerate(
                        backend.fee_rate_sat_per_vb()?,
                        amount,
                        REFUND_TX_VSIZE,
                    ),
                    REFUND_TX_VSIZE,
                );
                build_refund_tx(&htlc, outpoint, amount, destination, fee, &key)?
            }
        };
        let txid = backend.broadcast(&tx)?;
        rec.final_txid = Some(txid.to_string());
        rec.final_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&tx));
        rec.state = State::Refunded;
        self.store.put(&rec)?;
        let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
        Ok(rec)
    }

    /// One scheduler pass over every swap — pactd runs this periodically
    /// (and `pactd --once` runs exactly one pass). Performs only chain
    /// actions, never messaging: auto-redeem when a redeem is safe and
    /// due, auto-refund once MTP passes T, bookkeeping when our final tx
    /// confirms. Errors on one swap never block the others.
    /// Find and verify the HTLC funding for leg `leg` ("a"/"b"): try the
    /// recorded pointer (from the `funded` message) first, else discover it by
    /// the locally reconstructed HTLC scriptPubKey (the chain-watched fallback,
    /// so a lost `funded` message can't stall the swap). Returns (outpoint,
    /// confirmations) for an output whose script AND value match the agreed
    /// HTLC, or None if none is visible yet. The value+script match makes a
    /// wrong pointer or a stray same-address payment harmless.
    fn locate_funding(&self, rec: &SwapRecord, leg: &str) -> Result<Option<(OutPoint, u64)>> {
        let params = self.swap_params(rec)?;
        let (chain, htlc, amount, txid, vout) = match leg {
            "a" => (
                &rec.chain_a,
                params.htlc_a()?,
                rec.amount_a,
                rec.htlc_a_txid.as_deref(),
                rec.htlc_a_vout,
            ),
            _ => (
                &rec.chain_b,
                params.htlc_b()?,
                rec.amount_b,
                rec.htlc_b_txid.as_deref(),
                rec.htlc_b_vout,
            ),
        };
        let spk = htlc.script_pubkey();
        let expected_spk = hex::encode(spk.as_bytes());
        let backend = self.backend(chain)?;

        // 1) Message pointer (fast path).
        if let (Some(txid), Some(vout)) = (txid, vout) {
            let op = OutPoint {
                txid: bitcoin::Txid::from_str(txid)?,
                vout,
            };
            if let Some(info) = backend.get_txout(&op, &spk)? {
                if info.script_pubkey_hex == expected_spk && info.value_sat == amount {
                    return Ok(Some((op, info.confirmations)));
                }
            }
            // Pointer missing/spent/mismatched → fall through to a chain scan.
        }
        // 2) Chain-watched discovery by the derivable HTLC script. Re-read via
        //    get_txout (MultiBackend demands cross-backend agreement there)
        //    before trusting a discovered output.
        if let Some((op, _)) = backend.find_funding(&spk)? {
            if let Some(info) = backend.get_txout(&op, &spk)? {
                if info.script_pubkey_hex == expected_spk && info.value_sat == amount {
                    return Ok(Some((op, info.confirmations)));
                }
            }
        }
        Ok(None)
    }

    /// Timelock-relative safety deadline for the participant waiting on the
    /// chain-A funding (spec §7.3/§7.4): give up if it has not confirmed within
    /// ~25% of the window to our own chain-B refund (T2). Waiting longer would
    /// compress the rest of the swap (fund B, let the initiator redeem B before
    /// T2) into an unsafe window. Nothing is locked on our side at `accepted`,
    /// so the resulting abort costs nothing.
    fn funding_wait_expired(&self, rec: &SwapRecord) -> bool {
        let window = u64::from(rec.t2).saturating_sub(rec.created_at);
        let deadline = rec.created_at + window / 4;
        local_now() >= deadline
    }

    /// True once the §7.4 chain-B fund deadline (T2 − fund_margin) has passed:
    /// the taker can no longer safely fund leg B (mirrors the gate in `fund()`).
    /// Conservative — if the clock can't be read, return false so a transient
    /// node hiccup never aborts a still-fundable swap.
    fn fund_deadline_passed(&self, rec: &SwapRecord) -> bool {
        let net = rec.chain_b.network;
        let (fund_margin, _, _) = action_margins(net);
        let Ok(backend) = self.backend(&rec.chain_b) else {
            return false;
        };
        let Ok(mtp) = backend.tip_median_time() else {
            return false;
        };
        let now = deadline_clock(net, local_now(), mtp);
        !action_safe(now, fund_margin, rec.t2)
    }

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
        let adaptor_records = self.store.list_adaptor().unwrap_or_default();
        // Coins with a swap ACTIVE as of this tick (incl. ones going terminal
        // during it): swap progress moves the nodeless wallet — fundings
        // confirm, redeems land, refunds return — so poke those coins' sync
        // workers after the tick (issue #87: "swap events on the coin").
        // Long-terminal history swaps are excluded, so idle merchants poke
        // nothing.
        let mut active_coins: BTreeSet<String> = BTreeSet::new();
        for record in &records {
            if !matches!(
                record.state,
                State::Completed | State::Refunded | State::Aborted
            ) {
                active_coins.insert(record.chain_a.coin_id.clone());
                active_coins.insert(record.chain_b.coin_id.clone());
            }
        }
        for rec in &adaptor_records {
            if !matches!(
                rec.state,
                AdaptorState::Completed | AdaptorState::Refunded | AdaptorState::Aborted
            ) {
                active_coins.insert(rec.chain_a.coin_id.clone());
                active_coins.insert(rec.chain_b.coin_id.clone());
            }
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
        for rec in adaptor_records {
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
        for coin_id in &active_coins {
            self.wallet_manager.poke(coin_id);
        }
        // Observability: refresh the live progress snapshot from the records (and
        // fold in this round's latest event per swap). Never fails the tick.
        self.refresh_progress(&events);
        events
    }

    /// Rebuild the in-memory [`SwapProgress`] map from the current records. Called
    /// at the end of every [`Engine::tick`]; terminal swaps yield no progress so
    /// the map self-prunes. One light confirmations query per active swap per tick
    /// (the 30s scheduler cadence) — never on the UI's faster poll.
    fn refresh_progress(&self, events: &[TickEvent]) {
        // Previous snapshot — carries the awaiting-phase baseline forward so the
        // blocks-elapsed count survives across ticks.
        let prev = self.progress.lock().map(|g| g.clone()).unwrap_or_default();
        let mut latest: HashMap<&str, &TickEvent> = HashMap::new();
        for e in events {
            latest.insert(e.swap_id.as_str(), e); // events are in order → last wins
        }
        let mut snap: HashMap<String, SwapProgress> = HashMap::new();
        let fold = |mut p: SwapProgress| -> SwapProgress {
            if let Some(e) = latest.get(p.swap_id.as_str()) {
                if e.action != "error" {
                    p.last_action = Some(e.action.clone());
                    p.last_detail = Some(e.detail.clone());
                }
            }
            p
        };
        if let Ok(records) = self.store.list() {
            for rec in &records {
                if let Some(p) = self.swap_progress_v1(rec, &prev) {
                    snap.insert(rec.swap_id.clone(), fold(p));
                }
            }
        }
        for rec in self.store.list_adaptor().unwrap_or_default() {
            if let Some(p) = self.swap_progress_v2(&rec, &prev) {
                snap.insert(rec.swap_id.clone(), fold(p));
            }
        }
        if let Ok(mut g) = self.progress.lock() {
            *g = snap;
        }
    }

    /// A clone of the current progress snapshot (served by the `swapprogress` RPC).
    pub fn swap_progress_snapshot(&self) -> Vec<SwapProgress> {
        self.progress
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Progress for one v1 swap. Surfaces only waits that are OURS: the
    /// counterparty's lock burying toward `n` (our gate before we act) and our
    /// own claim burying (settlement). One nuance for the maker after it locks
    /// A: the taker won't lock B until our A lock reaches `n_a` (enforced in
    /// `fund`), so while it buries we surface that as a determinate `our_lock`
    /// target — the wait there is genuinely on our own lock, not theirs. Only
    /// once it's buried (taker still silent) do we fall through to the
    /// `awaiting_lock` liveness count on their chain. `None` when nothing applies.
    fn swap_progress_v1(
        &self,
        rec: &SwapRecord,
        prev: &HashMap<String, SwapProgress>,
    ) -> Option<SwapProgress> {
        use Role::*;
        use State::*;
        let htlc_spk = |leg_a: bool| {
            self.swap_params(rec)
                .ok()
                .and_then(|p| {
                    if leg_a {
                        p.htlc_a().ok()
                    } else {
                        p.htlc_b().ok()
                    }
                })
                .map(|h| h.script_pubkey())
        };
        match (rec.role, rec.state) {
            // Maker: once the taker's B lock is in, wait for it to bury, then
            // secure our B redeem. Before that, the taker won't lock B until our
            // A lock reaches `n_a` — so while OUR lock buries show it as a
            // determinate target; only once it's buried (taker still silent)
            // does this become a liveness wait on their chain.
            (Initiator, FundedA) | (Initiator, FundedB) => match &rec.htlc_b_txid {
                Some(txid) => self.progress_confirming(
                    &rec.swap_id,
                    &rec.chain_b,
                    txid.clone(),
                    rec.htlc_b_vout,
                    htlc_spk(false),
                    rec.n_b,
                    "their_lock",
                    None,
                ),
                // Our leg A is in; the taker won't lock B until it reaches `n_a`
                // (enforced in `fund`). While it buries → determinate `our_lock`;
                // once buried (taker still silent) → a liveness count anchored to
                // its confirmations, so it survives a restart.
                None => match rec.htlc_a_txid.as_deref() {
                    Some(txid_a) => {
                        let confs_a = self
                            .lock_confs(&rec.chain_a, txid_a, rec.htlc_a_vout, htlc_spk(true))
                            .unwrap_or(0);
                        // The TAKER acts here, at THEIR n_a (per-side depths,
                        // rc12 recut) — their advisory value makes the target
                        // exact; own n_a is the pre-exchange fallback.
                        let taker_n_a = rec.their_n_a.unwrap_or(rec.n_a);
                        if confs_a < taker_n_a {
                            self.progress_confirming(
                                &rec.swap_id,
                                &rec.chain_a,
                                txid_a.to_string(),
                                rec.htlc_a_vout,
                                htlc_spk(true),
                                taker_n_a,
                                "our_lock",
                                None,
                            )
                        } else {
                            self.progress_awaiting_anchored(
                                &rec.swap_id,
                                &rec.chain_b,
                                "awaiting_lock",
                                confs_a - taker_n_a,
                            )
                        }
                    }
                    None => {
                        self.progress_awaiting(&rec.swap_id, &rec.chain_b, "awaiting_lock", prev)
                    }
                },
            },
            (Initiator, RedeemedB) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_b,
                rec.final_txid.clone()?,
                None,
                spend_spk(rec),
                rec.n_b,
                "settlement",
                feerate_of(rec.final_tx_hex.as_deref(), rec.amount_b),
            ),
            // Taker: wait for the maker's A lock to bury; after we lock B, wait for
            // their reveal; then secure our A redeem.
            (Participant, Accepted) => match &rec.htlc_a_txid {
                Some(txid) => self.progress_confirming(
                    &rec.swap_id,
                    &rec.chain_a,
                    txid.clone(),
                    rec.htlc_a_vout,
                    htlc_spk(true),
                    rec.n_a,
                    "their_lock",
                    None,
                ),
                None => self.progress_awaiting(&rec.swap_id, &rec.chain_a, "awaiting_lock", prev),
            },
            // We've locked leg B; waiting for the maker's reveal. Anchor the
            // liveness count to our lock's depth (`htlc_b_height`, persisted at
            // funding) so it survives a restart instead of re-seeding to 0.
            // #3: leg A is verified; now WE fund leg B. While that funding is
            // pending/retrying (e.g. a locked wallet, #2) our OWN action is
            // outstanding — show "funding", NOT "awaiting_claim" (which would
            // imply it's the maker's turn). A growing count here flags a stuck
            // fund (unlock the wallet). On success we advance to FundedB.
            (Participant, FundedA) => {
                self.progress_awaiting(&rec.swap_id, &rec.chain_b, "funding", prev)
            }
            // Leg B is locked (`htlc_b_height` persisted at funding); wait for the
            // maker's reveal, anchoring the liveness count to our lock's depth so
            // it survives a restart instead of re-seeding to 0.
            // Symmetry with the maker's (Initiator, FundedA): while OUR leg B
            // buries toward n_b show a determinate "your lock confirming · confs/
            // n_b" — the maker cannot claim (reveal) until then. Once n_b-deep,
            // switch to the awaiting-their-claim liveness wait. confs are read
            // from chain each time (resumable, no persisted anchor).
            (Participant, FundedB) => {
                let confs_b = rec
                    .htlc_b_txid
                    .as_deref()
                    .and_then(|txid| {
                        self.lock_confs(&rec.chain_b, txid, rec.htlc_b_vout, htlc_spk(false))
                    })
                    .unwrap_or(0);
                // The MAKER reveals here, at THEIR n_b (per-side depths, rc12
                // recut) — their advisory value makes the target exact.
                let maker_n_b = rec.their_n_b.unwrap_or(rec.n_b);
                if confs_b < maker_n_b {
                    self.progress_confirming(
                        &rec.swap_id,
                        &rec.chain_b,
                        rec.htlc_b_txid.clone()?,
                        rec.htlc_b_vout,
                        htlc_spk(false),
                        maker_n_b,
                        "our_lock",
                        None,
                    )
                } else {
                    self.progress_awaiting_anchored(
                        &rec.swap_id,
                        &rec.chain_b,
                        "awaiting_claim",
                        confs_b.saturating_sub(maker_n_b),
                    )
                }
            }
            (Participant, Completed) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_a,
                rec.final_txid.clone()?,
                None,
                spend_spk(rec),
                rec.n_a,
                "settlement",
                feerate_of(rec.final_tx_hex.as_deref(), rec.amount_a),
            ),
            // Maker just accepted; funding leg A (about to broadcast / retrying).
            // A growing count here flags a stuck fund (e.g. a locked wallet, #2).
            (Initiator, Accepted) => {
                self.progress_awaiting(&rec.swap_id, &rec.chain_a, "funding", prev)
            }
            // A broadcast refund burying — surface it as securing (parity with the
            // redeem `settlement`) so a refund's wait is never a blank line. Each
            // side refunds its OWN leg: initiator leg A, participant leg B.
            (Initiator, Refunded) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_a,
                rec.final_txid.clone()?,
                None,
                spend_spk(rec),
                rec.n_a,
                "settlement",
                feerate_of(rec.final_tx_hex.as_deref(), rec.amount_a),
            ),
            (Participant, Refunded) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_b,
                rec.final_txid.clone()?,
                None,
                spend_spk(rec),
                rec.n_b,
                "settlement",
                feerate_of(rec.final_tx_hex.as_deref(), rec.amount_b),
            ),
            _ => None,
        }
    }

    /// Progress for one v2 (adaptor) swap — same OURS-only model as v1. v2 funds
    /// and waits inside the `Signed` state; the Taproot leg scriptPubKey is
    /// derived from the adaptor params (as the tick does).
    fn swap_progress_v2(
        &self,
        rec: &AdaptorSwapRecord,
        prev: &HashMap<String, SwapProgress>,
    ) -> Option<SwapProgress> {
        use crate::adaptor_swap::AdaptorState::*;
        let leg_spk = |leg_a: bool| {
            let secp = bitcoin::secp256k1::Secp256k1::new();
            let p = self.adaptor_params(rec).ok()?;
            if leg_a {
                p.leg_a(&secp).ok()?.script_pubkey(&secp).ok()
            } else {
                p.leg_b(&secp).ok()?.script_pubkey(&secp).ok()
            }
        };
        match (rec.role, rec.state) {
            // Maker: wait for the taker's B lock to bury, then secure our B redeem.
            // Leg B's txid is BUILT at accept (two-phase, spec §7), so it exists
            // the instant we reach `Signed` — but the taker only BROADCASTS it once
            // OUR leg A is `n_a`-deep. The phase is therefore driven by OBSERVING
            // the leg-B output (see `maker_signed_phase`), mirroring v1's maker:
            // while our leg A buries show a determinate `our_lock · confs/n_a`
            // ("Your lock confirming"), then a liveness `awaiting_lock` once it's
            // buried, then `their_lock · confs/n_b` from the FIRST SIGHTING of leg
            // B (mempool included — v1 flips on the taker's `funded` message at
            // broadcast, so gating on the first confirmation made v2 lag v1 by one
            // block) — never a misleading `their_lock` for a leg B that was never
            // broadcast (which drove narrate() to a false "both locked").
            (Role::Initiator, Signed) => {
                let leg_b_seen = rec.funding_b_txid.as_deref().and_then(|txid| {
                    self.lock_seen_confs(&rec.chain_b, txid, rec.funding_b_vout, leg_spk(false))
                });
                let leg_a_confs = rec.funding_a_txid.as_deref().map(|txid_a| {
                    self.lock_confs(&rec.chain_a, txid_a, rec.funding_a_vout, leg_spk(true))
                        .unwrap_or(0)
                });
                // The TAKER broadcasts leg B once leg A is n_a-deep BY THEIR
                // COUNT (per-side depths, rc12 recut) — their advisory value
                // makes both the phase flip and the displayed target exact.
                let taker_n_a = rec.their_n_a.unwrap_or(rec.n_a);
                match maker_signed_phase(leg_b_seen, leg_a_confs, taker_n_a) {
                    MakerSignedPhase::TheirLockB => self.progress_confirming(
                        &rec.swap_id,
                        &rec.chain_b,
                        rec.funding_b_txid.clone()?,
                        rec.funding_b_vout,
                        leg_spk(false),
                        rec.n_b,
                        "their_lock",
                        None,
                    ),
                    MakerSignedPhase::OurLockA => self.progress_confirming(
                        &rec.swap_id,
                        &rec.chain_a,
                        rec.funding_a_txid.clone()?,
                        rec.funding_a_vout,
                        leg_spk(true),
                        taker_n_a,
                        "our_lock",
                        None,
                    ),
                    // Anchor the liveness count to leg A's depth so it survives a
                    // restart (leg A is `n_a`-deep here, so this can't underflow).
                    MakerSignedPhase::AwaitingLock => self.progress_awaiting_anchored(
                        &rec.swap_id,
                        &rec.chain_b,
                        "awaiting_lock",
                        leg_a_confs.unwrap_or(taker_n_a).saturating_sub(taker_n_a),
                    ),
                    MakerSignedPhase::AwaitingLockUnanchored => {
                        self.progress_awaiting(&rec.swap_id, &rec.chain_b, "awaiting_lock", prev)
                    }
                }
            }
            (Role::Initiator, RedeemedB) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_b,
                rec.final_txid_b.clone()?,
                None,
                first_output_spk(rec.final_tx_b_hex.as_deref()),
                rec.n_b,
                "settlement",
                Some(rec.redeem_feerate_b),
            ),
            // Taker: wait for the maker's A lock; after we lock B, wait for their
            // reveal; then secure our A redeem.
            (Role::Participant, Signed) => {
                // Two-phase (spec §7): leg B is BUILT (funding_b_txid set) at accept
                // time but BROADCAST only after leg A is n_a-deep. So the honest
                // wait until we broadcast is on THEIR leg A burying — show that, not
                // a misleading "our lock 0/n_b" for a tx that isn't on-chain yet.
                if rec.funding_b_broadcast {
                    if let Some(txid_b) = &rec.funding_b_txid {
                        // OUR leg B is live and burying toward n_b (the maker can't
                        // reveal until then); once n_b-deep, the awaiting-their-claim
                        // wait. confs are a chain read (resumable).
                        let confs_b = self
                            .lock_confs(&rec.chain_b, txid_b, rec.funding_b_vout, leg_spk(false))
                            .unwrap_or(0);
                        // The MAKER reveals at THEIR n_b (per-side depths,
                        // rc12 recut) — their advisory value is the exact
                        // target we're waiting out.
                        let maker_n_b = rec.their_n_b.unwrap_or(rec.n_b);
                        if confs_b < maker_n_b {
                            self.progress_confirming(
                                &rec.swap_id,
                                &rec.chain_b,
                                txid_b.clone(),
                                rec.funding_b_vout,
                                leg_spk(false),
                                maker_n_b,
                                "our_lock",
                                None,
                            )
                        } else {
                            self.progress_awaiting_anchored(
                                &rec.swap_id,
                                &rec.chain_b,
                                "awaiting_claim",
                                confs_b.saturating_sub(maker_n_b),
                            )
                        }
                    } else {
                        self.progress_awaiting(&rec.swap_id, &rec.chain_a, "awaiting_lock", prev)
                    }
                } else if let Some(txid) = &rec.funding_a_txid {
                    // Leg B built, not yet broadcast: we are waiting for the maker's
                    // leg A to bury n_a-deep before we commit leg B — "their lock
                    // confirming · confs/n_a".
                    self.progress_confirming(
                        &rec.swap_id,
                        &rec.chain_a,
                        txid.clone(),
                        rec.funding_a_vout,
                        leg_spk(true),
                        rec.n_a,
                        "their_lock",
                        None,
                    )
                } else {
                    self.progress_awaiting(&rec.swap_id, &rec.chain_a, "awaiting_lock", prev)
                }
            }
            (Role::Participant, Completed) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_a,
                rec.final_txid_a.clone()?,
                None,
                first_output_spk(rec.final_tx_a_hex.as_deref()),
                rec.n_a,
                "settlement",
                Some(rec.redeem_feerate_a),
            ),
            // Pre-Signed (handshake): leg A is broadcast the INSTANT the taker's
            // `accept` lands (the autopilot funds on that message), but the swap
            // sits in these states for two more relay round-trips (nonces, then
            // partial sigs) before `Signed`. Mirror v1's maker once the lock is
            // out: `our_lock · confs/n_a` while it buries, then an anchored
            // liveness wait — a stale "funding" here reads as "not committed yet"
            // (and narrate's accepted line even offered free cancel) while the
            // coins are already locked. No pointer yet = the fund really is
            // pending (locked wallet, retry) → keep the "funding" hint.
            (Role::Initiator, Accepted | NoncesExchanged) => match rec.funding_a_txid.as_deref() {
                Some(txid_a) => {
                    let confs_a = self
                        .lock_confs(&rec.chain_a, txid_a, rec.funding_a_vout, leg_spk(true))
                        .unwrap_or(0);
                    // The taker will act at THEIR n_a (per-side, rc12 recut).
                    let taker_n_a = rec.their_n_a.unwrap_or(rec.n_a);
                    if confs_a < taker_n_a {
                        self.progress_confirming(
                            &rec.swap_id,
                            &rec.chain_a,
                            txid_a.to_string(),
                            rec.funding_a_vout,
                            leg_spk(true),
                            taker_n_a,
                            "our_lock",
                            None,
                        )
                    } else {
                        self.progress_awaiting_anchored(
                            &rec.swap_id,
                            &rec.chain_b,
                            "awaiting_lock",
                            confs_a - taker_n_a,
                        )
                    }
                }
                None => self.progress_awaiting(&rec.swap_id, &rec.chain_a, "funding", prev),
            },
            (Role::Participant, Accepted | NoncesExchanged) => match &rec.funding_a_txid {
                Some(txid) => self.progress_confirming(
                    &rec.swap_id,
                    &rec.chain_a,
                    txid.clone(),
                    rec.funding_a_vout,
                    leg_spk(true),
                    rec.n_a,
                    "their_lock",
                    None,
                ),
                None => self.progress_awaiting(&rec.swap_id, &rec.chain_a, "awaiting_lock", prev),
            },
            // A broadcast refund burying — surface as securing (parity with the
            // redeem `settlement`). Each side refunds its OWN leg.
            (Role::Initiator, Refunded) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_a,
                rec.final_txid_a.clone()?,
                None,
                first_output_spk(rec.final_tx_a_hex.as_deref()),
                rec.n_a,
                "settlement",
                None,
            ),
            (Role::Participant, Refunded) => self.progress_confirming(
                &rec.swap_id,
                &rec.chain_b,
                rec.final_txid_b.clone()?,
                None,
                first_output_spk(rec.final_tx_b_hex.as_deref()),
                rec.n_b,
                "settlement",
                None,
            ),
            _ => None,
        }
    }

    /// Confirmations of a leg's funding lock, read the robust way: scan the
    /// unspent output by outpoint+spk (works even when the tx isn't in our
    /// wallet or, on regtest, txindex is off), falling back to a wallet
    /// `gettransaction` for our own spends. `None` if the backend is
    /// unreachable; `0` while the output is unconfirmed or unseen. Used both for
    /// the determinate `confs/needed` lines and to anchor the liveness counts.
    fn lock_confs(
        &self,
        chain: &ChainRef,
        txid: &str,
        vout: Option<u32>,
        spk: Option<ScriptBuf>,
    ) -> Option<u32> {
        let backend = self.backend(chain).ok()?;
        let confs = match (vout, spk.as_ref()) {
            // A funding lock — usually the COUNTERPARTY's tx, so it isn't in our
            // wallet and (on regtest) txindex may be off, which makes
            // `tx_confirmations` blind to it. Scan the unspent output by
            // outpoint+spk instead, exactly as the tick does — while the lock is
            // unspent (the wait is live) this returns its depth.
            (Some(vout), Some(spk)) => bitcoin::Txid::from_str(txid)
                .ok()
                .and_then(|txid| {
                    backend
                        .get_txout(&OutPoint { txid, vout }, spk)
                        .ok()
                        .flatten()
                })
                .map(|o| o.confirmations)
                .unwrap_or(0),
            // Our own settlement tx is a wallet tx, so `gettransaction` finds it.
            _ => backend.tx_confirmations(txid, spk.as_ref()).unwrap_or(0),
        }
        .min(u64::from(u32::MAX)) as u32;
        Some(confs)
    }

    /// Like [`Engine::lock_confs`] for a funding outpoint, but preserving
    /// VISIBILITY: `None` when the outpoint isn't seen at all (not broadcast, or
    /// not yet propagated to our node), `Some(0)` while it sits in the mempool
    /// (`gettxout` includes the mempool), `Some(c)` once buried `c` deep. The v2
    /// maker's `Signed` phase needs the distinction — leg B's txid is known from
    /// accept, so only observation tells broadcast apart from not-yet.
    fn lock_seen_confs(
        &self,
        chain: &ChainRef,
        txid: &str,
        vout: Option<u32>,
        spk: Option<ScriptBuf>,
    ) -> Option<u32> {
        let backend = self.backend(chain).ok()?;
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid).ok()?,
            vout: vout?,
        };
        backend
            .get_txout(&outpoint, &spk?)
            .ok()
            .flatten()
            .map(|o| o.confirmations.min(u64::from(u32::MAX)) as u32)
    }

    /// A "confirming X/n" entry — a wait that is ours: the counterparty's lock
    /// burying (our gate before we act), or our own claim burying (settlement).
    /// `None` once `confs >= needed` (the leg is final, so the line disappears).
    fn progress_confirming(
        &self,
        swap_id: &str,
        chain: &ChainRef,
        txid: String,
        vout: Option<u32>,
        spk: Option<ScriptBuf>,
        needed: u32,
        watching: &str,
        feerate: Option<u64>,
    ) -> Option<SwapProgress> {
        let confs = self.lock_confs(chain, &txid, vout, spk)?;
        if confs >= needed {
            return None;
        }
        Some(SwapProgress {
            swap_id: swap_id.to_string(),
            watching: watching.into(),
            coin: coin_symbol(chain),
            confs,
            needed,
            blocks_elapsed: None,
            feerate_sat_vb: feerate,
            last_action: None,
            last_detail: None,
            updated_at: local_now(),
            awaiting_since_height: None,
        })
    }

    /// An "awaiting <counterparty action>" entry whose `blocks_elapsed` is
    /// anchored to an on-chain fact the caller computed (a lock's depth) rather
    /// than carried in the in-memory snapshot — so the count survives a restart.
    /// Used once the relevant lock is on-chain; the pre-lock waits (nothing
    /// on-chain to anchor to yet) still use [`Engine::progress_awaiting`].
    fn progress_awaiting_anchored(
        &self,
        swap_id: &str,
        chain: &ChainRef,
        watching: &str,
        blocks: u32,
    ) -> Option<SwapProgress> {
        Some(SwapProgress {
            swap_id: swap_id.to_string(),
            watching: watching.into(),
            coin: coin_symbol(chain),
            confs: 0,
            needed: 0,
            blocks_elapsed: Some(blocks),
            feerate_sat_vb: None,
            last_action: None,
            last_detail: None,
            updated_at: local_now(),
            awaiting_since_height: None,
        })
    }

    /// An "awaiting <counterparty action>" entry for the PRE-LOCK waits — no
    /// on-chain anchor exists yet (the counterparty hasn't locked), so the
    /// blocks-elapsed baseline is carried forward from the prior snapshot while
    /// the phase is unchanged. This count re-seeds to 0 on a restart; the
    /// lock/reveal waits avoid that via [`Engine::progress_awaiting_anchored`].
    fn progress_awaiting(
        &self,
        swap_id: &str,
        chain: &ChainRef,
        watching: &str,
        prev: &HashMap<String, SwapProgress>,
    ) -> Option<SwapProgress> {
        let tip = self
            .backend(chain)
            .ok()?
            .tip_height()
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32;
        let since = prev
            .get(swap_id)
            .filter(|p| p.watching == watching)
            .and_then(|p| p.awaiting_since_height)
            .unwrap_or(tip);
        Some(SwapProgress {
            swap_id: swap_id.to_string(),
            watching: watching.into(),
            coin: coin_symbol(chain),
            confs: 0,
            needed: 0,
            blocks_elapsed: Some(tip.saturating_sub(since)),
            feerate_sat_vb: None,
            last_action: None,
            last_detail: None,
            updated_at: local_now(),
            awaiting_since_height: Some(since),
        })
    }

    /// C8: abandon taker-side pending takes older than the handshake timeout.
    /// An abandoned take (maker committed elsewhere / vanished) would otherwise
    /// linger in our db forever — the only other clock on a take is the 24h
    /// offer TTL. Nothing is locked, so dropping it is safe; we just stop
    /// waiting on a dead handshake and emit a `take-timeout` event per drop.
    fn prune_stale_pending_takes(&self, events: &mut Vec<TickEvent>) -> Result<()> {
        let now = local_now();
        for (offer_id, _offer_json, created_at) in self.store.pending_takes_with_age()? {
            // Prune a take whose maker never followed through within the
            // pre-funding window — the abandoned handshake the timeout targets.
            if now.saturating_sub(created_at) >= PRE_FUNDING_TIMEOUT_SECS {
                self.store.remove_pending_take(&offer_id)?;
                // Report the truth: if inits DID arrive but failed processing,
                // say what failed instead of claiming "no init" (the misread
                // that cost the 2026-07-08 incident hours of transport
                // debugging). The last transient failure is recorded by the
                // init path under `take_last_error:<offer_id>`.
                let last_error_key = format!("take_last_error:{offer_id}");
                let detail = match self.store.meta_get(&last_error_key)? {
                    Some(err) => format!(
                        "no usable init within {}s; abandoning pending take (last init failed: {err})",
                        PRE_FUNDING_TIMEOUT_SECS
                    ),
                    None => format!(
                        "no init within {}s; abandoning pending take",
                        PRE_FUNDING_TIMEOUT_SECS
                    ),
                };
                let _ = self.store.meta_del(&last_error_key);
                events.push(TickEvent {
                    swap_id: offer_id.clone(),
                    action: "take-timeout".into(),
                    detail,
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
                // Only auto-redeem (reveal s) while we are still inside the §7.4
                // reveal deadline (T2 − 2h); past it, fall through to the refund.
                let net = rec.chain_b.network;
                let (_, reveal_margin, _) = action_margins(net);
                let now = deadline_clock(net, local_now(), backend_b.tip_median_time()?);
                if action_safe(now, reveal_margin, rec.t2) {
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
            // Alice funded chain A; while chain B can still be redeemed safely
            // (before T2) watch chain B for Bob's funding — the `funded` message
            // is only a hint now — and advance to FundedB once it is
            // n_b-confirmed, so the FundedB arm above completes it. Once that
            // window has closed (or chain B never appeared) fall back to the T1
            // refund of chain A rather than chase a redeem we can't finish.
            (Role::Initiator, State::FundedA) => {
                // Nurse our own (leg-A) funding while it is unconfirmed — RBF it up
                // to the current market if it went out under-priced.
                if let Some(ev) =
                    self.maybe_bump_funding_v1(rec, "a", &self.backend(&rec.chain_a)?)?
                {
                    return Ok(Some(ev));
                }
                let backend_b = self.backend(&rec.chain_b)?;
                // No point advancing to FundedB once we could no longer reveal s
                // safely (§7.4 reveal deadline T2 − 2h): fall back to the T1
                // refund of chain A rather than chase a redeem we can't finish.
                let net = rec.chain_b.network;
                let (_, reveal_margin, _) = action_margins(net);
                let now = deadline_clock(net, local_now(), backend_b.tip_median_time()?);
                if action_safe(now, reveal_margin, rec.t2) {
                    if let Some((outpoint, confs)) = self.locate_funding(rec, "b")? {
                        if confs >= u64::from(rec.n_b) {
                            let mut updated = rec.clone();
                            updated.htlc_b_txid = Some(outpoint.txid.to_string());
                            updated.htlc_b_vout = Some(outpoint.vout);
                            updated.htlc_b_height =
                                Some(backend_b.tip_height()?.saturating_sub(confs));
                            updated.state = State::FundedB;
                            self.store.put(&updated)?;
                            return event(
                                "funded-b",
                                "chain-B HTLC confirmed (chain-watched)".into(),
                            );
                        }
                        // #6: record the leg-B funding pointer on FIRST chain
                        // detection (before n_b), so the maker's progress shows
                        // `their_lock confs/n_b` — parity with the relay path, which
                        // sets it from the `funded` message. State stays FundedA (the
                        // redeem still gates on n_b above). Not redundant derived
                        // state: htlc_b_txid is the core leg-B pointer the message
                        // path persists too — we just discovered it from chain.
                        if rec.htlc_b_txid.is_none() {
                            let mut updated = rec.clone();
                            updated.htlc_b_txid = Some(outpoint.txid.to_string());
                            updated.htlc_b_vout = Some(outpoint.vout);
                            updated.htlc_b_height =
                                Some(backend_b.tip_height()?.saturating_sub(confs));
                            self.store.put(&updated)?;
                            return event(
                                "their-lock",
                                "chain-B HTLC seen; burying to n_b (chain-watched)".into(),
                            );
                        }
                    }
                }
                self.try_refund_due(rec, "a")
            }
            // Alice's redeem broadcast: mark completed once it confirms;
            // fee-bump while it does not (§7.4: the reveal must not linger
            // in a mempool as T2 approaches).
            (Role::Initiator, State::RedeemedB) => {
                let backend_b = self.backend(&rec.chain_b)?;
                let txid = rec.final_txid.as_deref().context("no redeem txid")?;
                let confs = backend_b.tx_confirmations(txid, spend_spk(rec).as_ref())?;
                // Completion needs the chain's full confirmation policy,
                // not 1 conf — a shallow redeem can still reorg away, and
                // the T1 refund stays armed until this point (spec §9.5).
                if confs >= u64::from(rec.n_b) {
                    let mut updated = rec.clone();
                    updated.state = State::Completed;
                    self.store.put(&updated)?;
                    let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
                    return event("completed", txid.to_string());
                }
                // Mined but shallow (1..n_b): the redeem is in a block, so it
                // can't be RBF'd — nursing it would only emit rejected
                // double-spend broadcasts every tick. Just wait for depth.
                if confs >= 1 {
                    return Ok(None);
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
                // Nurse our own (leg-B) funding while it is unconfirmed.
                if let Some(ev) = self.maybe_bump_funding_v1(rec, "b", &backend_b)? {
                    return Ok(Some(ev));
                }
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
                        // §7.4: claim chain A only while inside Bob's redeem
                        // deadline (T1 − 1h); past it a redeem races Alice's
                        // refund, so leave it (our chain-B leg is already gone).
                        let net = rec.chain_a.network;
                        let (_, _, redeem_a_margin) = action_margins(net);
                        let now = deadline_clock(net, local_now(), backend_a.tip_median_time()?);
                        if action_safe(now, redeem_a_margin, rec.t1) {
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
            // Bob waiting on Alice's chain-A funding. The `funded` message is a
            // hint; we detect (or rediscover) the chain-A HTLC by its derivable
            // script and advance once it is n_a-confirmed, then fund chain B
            // (fund() re-verifies chain A as a reorg guard before committing).
            // If it has not confirmed by the timelock-relative safety deadline,
            // abort cleanly — nothing is locked on our side at `accepted`.
            (Role::Participant, State::Accepted) => {
                if let Some((outpoint, confs)) = self.locate_funding(rec, "a")? {
                    if confs >= u64::from(rec.n_a) {
                        let mut updated = rec.clone();
                        updated.htlc_a_txid = Some(outpoint.txid.to_string());
                        updated.htlc_a_vout = Some(outpoint.vout);
                        updated.state = State::FundedA;
                        self.store.put(&updated)?;
                        if self.auto_fund {
                            let (funded, env) = self.fund(&updated.swap_id)?;
                            if let Some(cp) = funded.counterparty_identity.clone() {
                                // Best-effort: the initiator also chain-watches.
                                let _ = self.relay_send_all(&cp, &env);
                            }
                            return event("auto-fund", "chain-A confirmed; funded chain B".into());
                        }
                        return event(
                            "funded-a",
                            "chain-A HTLC confirmed; ready to fund chain B".into(),
                        );
                    }
                }
                if self.funding_wait_expired(rec) {
                    self.abort(
                        &rec.swap_id,
                        "chain-A funding not confirmed before the safety deadline",
                    )?;
                    return event(
                        "abort-timeout",
                        "chain-A funding not confirmed in time; aborted (nothing locked)".into(),
                    );
                }
                Ok(None)
            }
            // Retry the taker's leg-B funding (rc6 #2): the fund may have failed
            // to BROADCAST after the state advanced to FundedA — e.g. a locked
            // wallet (RPC -13) — stranding the swap with no retry. Re-attempt each
            // tick so it self-heals once the wallet is unlocked; fund() is
            // idempotent (adopts an on-chain funding rather than double-funding).
            (Role::Participant, State::FundedA) => {
                if !self.auto_fund {
                    return Ok(None);
                }
                // Past the §7.4 fund deadline leg B can never be safely funded, and
                // nothing is locked on our side — abort cleanly (maker refunds A at T1).
                if self.fund_deadline_passed(rec) {
                    self.abort(
                        &rec.swap_id,
                        "missed the chain-B fund deadline (wallet locked too long?)",
                    )?;
                    return event(
                        "abort-fund-deadline",
                        "too late to fund chain B; aborted (nothing locked on our side)".into(),
                    );
                }
                let (funded, env) = self.fund(&rec.swap_id)?;
                if let Some(cp) = funded.counterparty_identity.clone() {
                    let _ = self.relay_send_all(&cp, &env);
                }
                event("auto-fund", "retried chain-B funding".into())
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
                // Rescue safety (#54): a node restored from the accept snapshot is
                // `Accepted` with NO funding pointer even when our leg is already
                // funded on chain. `abort()` assumes nothing is committed (its
                // guard is pointer-based), so aborting here would strand a funded
                // leg. Rediscover our own leg by its derivable script FIRST: if
                // it is on chain, adopt the pointer and advance to the funded
                // state so the refund/redeem path — not a false abort — governs
                // the committed funds. Only a genuinely unfunded, stale handshake
                // aborts. (Participant `Accepted` is handled by an earlier arm.)
                let our_leg = match rec.role {
                    Role::Initiator => "a",
                    Role::Participant => "b",
                };
                if let Some((outpoint, _confs)) = self.locate_funding(rec, our_leg)? {
                    let mut updated = rec.clone();
                    match rec.role {
                        Role::Initiator => {
                            updated.htlc_a_txid = Some(outpoint.txid.to_string());
                            updated.htlc_a_vout = Some(outpoint.vout);
                            updated.state = State::FundedA;
                        }
                        Role::Participant => {
                            updated.htlc_b_txid = Some(outpoint.txid.to_string());
                            updated.htlc_b_vout = Some(outpoint.vout);
                            updated.htlc_b_height = Some(self.backend(&rec.chain_b)?.tip_height()?);
                            updated.state = State::FundedB;
                        }
                    }
                    self.store.put(&updated)?;
                    return event(
                        "rescue-adopt-funding",
                        format!(
                            "adopted our funded leg {our_leg}; state {:?}",
                            updated.state
                        ),
                    );
                }
                self.abort(&rec.swap_id, "pre-funding handshake timed out")?;
                event(
                    "abort-timeout",
                    format!("no funding within {PRE_FUNDING_TIMEOUT_SECS}s; aborted"),
                )
            }
            // Retry the maker's leg-A funding (rc6 #2), mirroring the taker's
            // FundedA retry. The recv-driven fund may have failed to broadcast
            // (locked wallet → -13) with the state left at Accepted. Re-attempt
            // each tick; the C8 timeout-abort above matches first once expired,
            // so this only runs inside the pre-funding window. Idempotent fund().
            (Role::Initiator, State::Accepted) => {
                if !self.auto_fund {
                    return Ok(None);
                }
                let (funded, env) = self.fund(&rec.swap_id)?;
                if let Some(cp) = funded.counterparty_identity.clone() {
                    let _ = self.relay_send_all(&cp, &env);
                }
                event("auto-fund", "retried chain-A funding".into())
            }
            _ => Ok(None),
        }
    }

    /// Refund leg `leg` if its timelock has matured and the HTLC is still
    /// unspent; otherwise do nothing (the next tick retries).
    fn try_refund_due(&self, rec: &SwapRecord, leg: &str) -> Result<Option<TickEvent>> {
        let (chain, locktime) = match leg {
            "a" => (&rec.chain_a, rec.t1),
            _ => (&rec.chain_b, rec.t2),
        };
        let backend = self.backend(chain)?;
        // Conservative (min) MTP for refund readiness (M6) — matches refund().
        if backend.tip_median_time_min()? < u64::from(locktime) {
            return Ok(None);
        }
        // Locate the funding by the stored pointer, FALLING BACK to a spk-based
        // chain scan if the pointer is dead (`locate_funding`'s fallback). This
        // self-heals a pointer left stale by a funding RBF whose local bookkeeping
        // didn't land (see `maybe_bump_funding_v1`): without it a stale pointer
        // reads as "already spent" and the auto-refund would silently never fire.
        // `None` = genuinely nothing to refund (a real spend / not funded yet).
        let Some((outpoint, _confs)) = self.locate_funding(rec, leg)? else {
            return Ok(None);
        };
        // If the live outpoint differs from what we recorded, the record is stale
        // (post-RBF). Re-sync the pointer AND drop the pre-signed refund (it was
        // built against the old outpoint), so `refund()` rebuilds against the live
        // one; persist before refunding so the fix is durable.
        let stored = match leg {
            "a" => rec.htlc_a_txid.as_deref().zip(rec.htlc_a_vout),
            _ => rec.htlc_b_txid.as_deref().zip(rec.htlc_b_vout),
        };
        let live_txid = outpoint.txid.to_string();
        if stored != Some((live_txid.as_str(), outpoint.vout)) {
            let mut fixed = rec.clone();
            fixed.refund_tx_hex = None; // stale: signed against the old outpoint
            match leg {
                "a" => {
                    fixed.htlc_a_txid = Some(live_txid.clone());
                    fixed.htlc_a_vout = Some(outpoint.vout);
                }
                _ => {
                    fixed.htlc_b_txid = Some(live_txid.clone());
                    fixed.htlc_b_vout = Some(outpoint.vout);
                }
            }
            self.store.put(&fixed)?;
        }
        let updated = self.refund(&rec.swap_id)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "auto-refund".into(),
            detail: updated.final_txid.unwrap_or_default(),
        }))
    }

    /// Nurse our unconfirmed HTLC spend (v1 redeem/refund) toward the live
    /// market, not by a market-blind geometric step. The unified strategy
    /// (post-mortem 2026-06-25, `fee-bump-design.md` §2.3):
    ///
    /// - **Block-driven cadence** — act at most once per block (`last_action_height`);
    ///   the 30s tick that produced the live-mainnet fee storm now backs out within
    ///   the same block.
    /// - **Market-tracking, value-capped target** — [`FeeBumpPolicy::target_feerate`]
    ///   = `min(market, value_at_risk·cap, ceiling)`; it can never bid 159 sat/vB
    ///   into a 1 sat/vB market, and never exceeds the amount being claimed.
    /// - **Bump only when it clears BIP125 Rule 4** — the target must beat the
    ///   current feerate by the node's incremental-relay fee (A4), else there is
    ///   nothing relayable to do.
    /// - **Evicted-only rebroadcast** — when no bump is warranted, re-anchor the
    ///   *same* tx only if it actually fell out of the mempool; steady state is
    ///   silent (no per-tick wallet churn).
    ///
    /// Only called with a 0-confirmation tx (callers gate on confirmations).
    fn maybe_bump(&self, rec: &SwapRecord, backend: &MultiBackend) -> Result<Option<TickEvent>> {
        let Some(tx_hex) = &rec.final_tx_hex else {
            return Ok(None); // record predates fee-bumping support
        };
        // Step 0: act at most once per block.
        let tip_height = backend.tip_height()?;
        if tip_height == rec.last_action_height {
            return Ok(None);
        }
        let old_tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
                .context("corrupt final_tx_hex")?;
        let old_txid = old_tx.compute_txid().to_string();
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
        let vsize = if is_redeem {
            REDEEM_TX_VSIZE
        } else {
            REFUND_TX_VSIZE
        };
        let old_fee = amount.saturating_sub(old_tx.output[0].value.to_sat());
        let old_feerate_kvb = old_fee.saturating_mul(1000) / vsize.max(1);

        // Step 3: market-tracking, value-capped target. This nurses redeem and
        // refund (both claim spends) → value-capped, NOT the funding fee ceiling.
        // For REDEEM (#47) the market estimate is deadline-aware: escalate the
        // conf_target as the redeem's confirm-by deadline nears (Initiator leg-B
        // → T2; Participant leg-A → T1 − redeem-a margin), because a redeem that
        // misses its timelock loses the leg. Refunds keep the cheap baseline (we
        // are the only spender — no counterparty race).
        let (conf_target, conservative) = if is_redeem {
            let deadline = match rec.role {
                Role::Initiator => u64::from(rec.t2),
                _ => u64::from(rec.t1).saturating_sub(action_margins(chain.network).2),
            };
            let now = deadline_clock(chain.network, local_now(), backend.tip_median_time()?);
            redeem_conf_target(deadline.saturating_sub(now))
        } else {
            (6, false)
        };
        // sat/kvB throughout — Core/Electrum quote BTC/kvB, so keep that native
        // resolution instead of rounding the market to a whole sat/vB.
        let market_kvb = backend.fee_rate_for_kvb(conf_target, conservative)?;
        let target_kvb = self.fee_bump.claim_feerate_kvb(market_kvb, amount, vsize);

        // Step 4 gate: bump when the market has risen above what the tx already
        // pays — nothing to do otherwise (the "already paying enough" no-op that
        // escalate() lacked). Re-anchor the existing tx only if it was evicted
        // (step 5). The BIP125 Rule-4 increment is enforced by the ABSOLUTE
        // `new_fee` floor below, NOT by the gate: with a 1-sat/vB increment the
        // old integer gate `target < old + incr` collapsed to exactly this
        // `target <= old` test, so keeping the increment in the gate too (now in
        // precise kvB) would wrongly refuse a sub-sat/vB rise that master acted on.
        let incr_kvb = backend.incremental_relay_feerate_kvb()?;
        // Rule 4 constrains the ABSOLUTE fee: the replacement must pay at least
        // `incr * vsize` more than the tx it evicts. Compute both fees in sat
        // (kvB × vsize, rounded up) and floor to old_fee + incr·vsize — otherwise
        // the node rejects the RBF (-26).
        let new_fee = target_kvb
            .saturating_mul(vsize)
            .div_ceil(1000)
            .max(old_fee.saturating_add(incr_kvb.saturating_mul(vsize).div_ceil(1000)));
        let dustless = amount > new_fee + DUST_LIMIT_SAT;
        if target_kvb <= old_feerate_kvb || !dustless {
            return self.reanchor_if_evicted(rec, backend, &old_tx, &old_txid);
        }

        // Step 4: build and broadcast the higher-fee replacement, then record the
        // block we acted in so we don't act again until the next block.
        let outpoint = old_tx.input[0].previous_output;
        let seed = self.store.seed()?;
        let key = v1_swap_key(&seed, rec, coin_of(chain)?)?;
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
        updated.last_action_height = tip_height;
        self.store.put(&updated)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "fee-bump".into(),
            detail: format!(
                "{txid} (fee {old_fee} -> {new_fee} sat, {:.3} -> {:.3} sat/vB)",
                old_feerate_kvb as f64 / 1000.0,
                target_kvb as f64 / 1000.0
            ),
        }))
    }

    /// Step 5 of the bump loop: when no fee bump is warranted, re-broadcast the
    /// *same* tx (same txid — invisible to the wallet) only if it actually fell
    /// out of the mempool. In steady state the tx is present, so this is a silent
    /// no-op — eliminating the per-tick rebroadcast that the old escalator emitted.
    fn reanchor_if_evicted(
        &self,
        rec: &SwapRecord,
        backend: &MultiBackend,
        old_tx: &bitcoin::Transaction,
        old_txid: &str,
    ) -> Result<Option<TickEvent>> {
        if backend.is_in_mempool(old_txid)? {
            return Ok(None); // present → nothing to do
        }
        let txid = backend.broadcast(old_tx)?;
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "rebroadcast".into(),
            detail: txid.to_string(),
        }))
    }

    /// v1 funding nurse: RBF-bump our own unconfirmed funding (`leg` = "a"/"b")
    /// when the market has risen above the rate it went out at, before the
    /// fund-margin deadline. Returns an event only when it bumps (or skips on a
    /// recoverable `bumpfee` failure); a silent `Ok(None)` is the common no-op.
    ///
    /// Liveness only, never a loss: a stalled funding refunds at the timelock. RBF
    /// is safe vs the counterparty because they detect the lock by **scriptPubKey,
    /// not txid** (`find_funding` → `scantxoutset raw(<spk>)`), so an RBF that keeps
    /// the HTLC output identical is invisible to them — and we run only while the
    /// funding is unconfirmed, before they have waited out the confirmations. The
    /// v1 refund is a single-key CLTV spend, so re-signing it against the new
    /// outpoint is purely local (no counterparty round).
    fn maybe_bump_funding_v1(
        &self,
        rec: &SwapRecord,
        leg: &str,
        backend: &MultiBackend,
    ) -> Result<Option<TickEvent>> {
        let params = self.swap_params(rec)?;
        let (chain, amount, htlc, txid, vout) = match leg {
            "a" => (
                &rec.chain_a,
                rec.amount_a,
                params.htlc_a()?,
                rec.htlc_a_txid.as_deref(),
                rec.htlc_a_vout,
            ),
            _ => (
                &rec.chain_b,
                rec.amount_b,
                params.htlc_b()?,
                rec.htlc_b_txid.as_deref(),
                rec.htlc_b_vout,
            ),
        };
        let (Some(txid), Some(vout)) = (txid, vout) else {
            return Ok(None); // our leg isn't funded yet
        };
        let htlc_spk = htlc.script_pubkey();
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(txid)?,
            vout,
        };
        // Only nurse while the funding is unconfirmed; once it has a confirmation
        // (or vanished) there is nothing to bump.
        match backend.get_txout(&outpoint, &htlc_spk)? {
            Some(txout) if txout.confirmations == 0 => {}
            _ => return Ok(None),
        }
        // Deadline gate against this leg's OWN refund timelock on its OWN chain
        // (leg A → T1, leg B → T2), with the fund margin. Past it, stop bumping and
        // let it stall → refund.
        let deadline = if leg == "a" { rec.t1 } else { rec.t2 };
        let net = chain.network;
        let (fund_margin, _, _) = action_margins(net);
        let now = deadline_clock(net, local_now(), backend.tip_median_time()?);
        if !action_safe(now, fund_margin, deadline) {
            return Ok(None);
        }
        // Recompute the broadcast feerate; chase market, bounded by the policy
        // ceiling AND the funds-gate reservation (× old_feerate, so the bump stays
        // within the headroom that gate set aside). All sat/kvB — the estimator's
        // native resolution — so a sub-integer market and the tx's true feerate
        // (fee/vsize, NOT truncated to a whole sat/vB) are compared exactly.
        let (old_fee, fvsize) = backend.wallet_tx_fee_vsize(txid)?;
        let old_feerate_kvb = old_fee.saturating_mul(1000) / fvsize.max(1);
        let market_kvb = backend.fee_rate_for_kvb(backend.funding_conf_target(), false)?;
        let incr_kvb = backend.incremental_relay_feerate_kvb()?;
        // Chase market (floored to the Rule-4 minimum so the RBF is ACCEPTED, not
        // rejected as it was in the field — see `funding_bump_rate_kvb`), or a
        // no-op when the market hasn't moved.
        let Some(rate_kvb) = funding_bump_rate_kvb(
            old_feerate_kvb,
            market_kvb,
            incr_kvb,
            self.fee_bump.max_feerate_sat_vb.saturating_mul(1000),
            self.fee_bump.funding.reservation_mult,
        ) else {
            return Ok(None);
        };
        // RBF via the wallet. A recoverable failure (insufficient funds — the funds
        // gate is a soft pre-flight, not a lock — or not-replaceable) is a graceful
        // no-op for this tick, never a crash: the funding stalls → refund.
        let new_txid = match backend.wallet_bumpfee(txid, rate_kvb) {
            Ok(t) => t,
            Err(e) => {
                return Ok(Some(TickEvent {
                    swap_id: rec.swap_id.clone(),
                    action: "funding-bump-skipped".into(),
                    detail: format!("leg {leg}: {e:#}"),
                }));
            }
        };
        // Bookkeeping AFTER the on-chain bump succeeded: re-locate the HTLC output
        // on the replacement (the bump funds itself from change; the HTLC value is
        // unchanged but its vout can move), rebuild/re-sign the refund against the
        // new outpoint (single-key, local), and persist the new pointer + refund in
        // one atomic put. If any of this fails the bump has ALREADY happened, so we
        // must NOT propagate a hard error: emit a warning carrying the new txid and
        // let chain-watch (find_funding, spk-based) re-sync the pointer on a later
        // tick — self-healing. The same self-heal covers a crash before the put.
        let bookkeep = || -> Result<()> {
            let new_vout = backend.find_vout(&new_txid, &hex::encode(htlc_spk.as_bytes()))?;
            let new_outpoint = OutPoint {
                txid: bitcoin::Txid::from_str(&new_txid)?,
                vout: new_vout,
            };
            let seed = self.store.seed()?;
            let key = v1_swap_key(&seed, rec, coin_of(chain)?)?;
            let destination = backend
                .params()
                .parse_address(&backend.wallet_new_address()?)?;
            let fee = spend_fee_sat(
                self.fee_bump.target_feerate(
                    backend.fee_rate_sat_per_vb()?,
                    amount,
                    REFUND_TX_VSIZE,
                ),
                REFUND_TX_VSIZE,
            );
            let refund_tx = build_refund_tx(&htlc, new_outpoint, amount, destination, fee, &key)?;
            let mut updated = rec.clone();
            updated.refund_tx_hex = Some(bitcoin::consensus::encode::serialize_hex(&refund_tx));
            match leg {
                "a" => {
                    updated.htlc_a_txid = Some(new_txid.clone());
                    updated.htlc_a_vout = Some(new_vout);
                }
                _ => {
                    updated.htlc_b_txid = Some(new_txid.clone());
                    updated.htlc_b_vout = Some(new_vout);
                }
            }
            self.store.put(&updated)
        };
        if let Err(e) = bookkeep() {
            return Ok(Some(TickEvent {
                swap_id: rec.swap_id.clone(),
                action: "funding-bump-resync-pending".into(),
                detail: format!(
                    "leg {leg}: bumped to {new_txid} but local refund update failed \
                     ({e:#}); chain-watch will re-sync"
                ),
            }));
        }
        Ok(Some(TickEvent {
            swap_id: rec.swap_id.clone(),
            action: "funding-fee-bump".into(),
            detail: format!(
                "leg {leg}: {new_txid} (funding {:.3} -> {:.3} sat/vB)",
                old_feerate_kvb as f64 / 1000.0,
                rate_kvb as f64 / 1000.0
            ),
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
    /// and the Nostr board alike, since both implement
    /// [`Noticeboard`](crate::board::Noticeboard). (This is
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
        let offers = chosen.1.offers()?;
        // Honor our own local revocations. A withdrawn offer can still linger on a
        // stateless HTTP corkboard, or be re-served by a relay before the NIP-09
        // deletion propagates — so filter anything we've locally blocked
        // (`offer_revoked:<swap_id>`, the same marker the take guards check). Without
        // this, navigating away from and back to the board re-lists an offer the
        // maker just withdrew.
        let mut kept = Vec::with_capacity(offers.len());
        for o in offers {
            if self
                .store
                .meta_get(&format!("offer_revoked:{}", o.swap_id))?
                .is_some()
            {
                continue;
            }
            kept.push(o);
        }
        Ok(kept)
    }

    // ---- Encrypted swap-state rescue (issue #54) ----

    /// Publish an encrypted-to-self snapshot of a v1 swap record. Taken once
    /// after `accept`: the negotiated params make any funding we commit
    /// refundable — and (via the seed preimage / chain-extracted secret)
    /// completable — from the seed alone. Best-effort across boards.
    fn snapshot_v1(&self, rec: &SwapRecord) -> Result<()> {
        let body = serde_json::json!({
            "v": 1,
            "record": serde_json::to_value(rec)?,
            "next_index": self.store.peek_next_swap_index()?,
        });
        // v1 snapshots once (accept) — no later state to outrank.
        self.publish_snapshot_body(&rec.swap_id, body, 0)
    }

    /// Publish an encrypted-to-self snapshot of a v2 record. Taken at `accept`
    /// (leg-A refund basis for the initiator) and again at `Signed`, where the
    /// record additionally carries the assembled adaptor signatures — the one
    /// datum that is neither seed- nor chain-derivable — so the swap can be
    /// COMPLETED, not just refunded, from the record alone.
    fn snapshot_v2(&self, rec: &AdaptorSwapRecord) -> Result<()> {
        let body = serde_json::json!({
            "v": 2,
            "record": serde_json::to_value(rec)?,
            "next_index": self.store.peek_next_swap_index()?,
        });
        // State rank: the Signed snapshot must strictly REPLACE the accept one
        // on the relay even when both publish within the same second — a
        // rescued maker restored to `accepted` cannot re-handshake (the
        // counterparty's nonces are consumed) and would strand until refund.
        let seq = match rec.state {
            AdaptorState::Created | AdaptorState::Accepted | AdaptorState::NoncesExchanged => 0,
            _ => 1,
        };
        self.publish_snapshot_body(&rec.swap_id, body, seq)
    }

    fn publish_snapshot_body(
        &self,
        swap_id: &str,
        body: serde_json::Value,
        seq: u64,
    ) -> Result<()> {
        let env = self.signed_envelope("swapstate", swap_id, body)?;
        let me = self.store.seed()?.identity_pubkey()?.to_string();
        let blob = crate::board::seal_envelope(&me, &env)?;
        for (_, board) in self.boards()? {
            let _ = board.publish_snapshot(swap_id, &blob, seq); // best-effort per board
        }
        Ok(())
    }

    /// Tombstone a swap's rescue snapshot once it reaches a terminal state, so a
    /// machine restored from seed never resurrects a finished swap.
    fn tombstone_swap(&self, swap_id: &str) -> Result<()> {
        for (_, board) in self.boards()? {
            let _ = board.tombstone_snapshot(swap_id);
        }
        Ok(())
    }

    /// Rebuild in-flight swaps from encrypted-to-self relay snapshots — the
    /// seed-only cross-machine recovery path (#54). `blobs` are the sealed
    /// `PACTSEALED1:` payloads pactd fetched from our own snapshot events. Each
    /// is decrypted with our identity key, its inner signature verified, and the
    /// record adopted IFF we have no local record for that swap_id (local always
    /// wins) and it is not terminal. Restores the next-swap-index high-water
    /// mark so a reissued index can never reuse a completed swap's keys. Returns
    /// `(restored, seen)`. The scheduler then drives each rescued swap to
    /// completion or refund via chain-watch — all later state is derivable.
    pub fn rescue_from_blobs(&self, blobs: &[String]) -> Result<(usize, usize)> {
        let (kp, me, have) = self.rescue_context()?;
        let mut restored = 0usize;
        let mut hi = self.store.peek_next_swap_index()?;
        for blob in blobs {
            match self.rescue_decode(&kp, &me, blob) {
                Ok((rec, next_index)) => {
                    hi = hi.max(next_index);
                    if !have.contains(rec.swap_id()) && !rec.terminal() {
                        match &rec {
                            RescuedRecord::V1(r) => self.store.put(r)?,
                            RescuedRecord::V2(r) => self.store.put_adaptor(r)?,
                        }
                        restored += 1;
                    }
                }
                Err(e) => eprintln!("rescue: skipping unreadable snapshot: {e:#}"),
            }
        }
        self.store.set_next_swap_index_at_least(hi)?;
        Ok((restored, blobs.len()))
    }

    /// Read-only twin of [`Engine::rescue_from_blobs`]: count the snapshots
    /// that WOULD be adopted, without adopting anything or moving the index
    /// high-water mark. This is the detection half of the gated rescue (#54):
    /// pactd surfaces the count + the two-machines warning and waits for an
    /// explicit `restorefromrelay` — silently re-driving a swap that another
    /// live machine on the same seed is still driving can double-fund it.
    pub fn rescue_preview(&self, blobs: &[String]) -> Result<(usize, usize)> {
        let (kp, me, have) = self.rescue_context()?;
        let mut pending = 0usize;
        for blob in blobs {
            match self.rescue_decode(&kp, &me, blob) {
                Ok((rec, _)) if !have.contains(rec.swap_id()) && !rec.terminal() => pending += 1,
                Ok(_) => {}
                Err(e) => eprintln!("rescue: skipping unreadable snapshot: {e:#}"),
            }
        }
        Ok((pending, blobs.len()))
    }

    /// Shared setup for a rescue pass: identity keypair + pubkey and the set
    /// of swap ids we already hold locally (local always wins over a snapshot).
    fn rescue_context(
        &self,
    ) -> Result<(
        bitcoin::secp256k1::Keypair,
        String,
        std::collections::HashSet<String>,
    )> {
        let seed = self.store.seed()?;
        let kp = seed.identity_keypair()?;
        let me = seed.identity_pubkey()?.to_string();
        let mut have: std::collections::HashSet<String> = std::collections::HashSet::new();
        for r in self.store.list()? {
            have.insert(r.swap_id);
        }
        for r in self.store.list_adaptor()? {
            have.insert(r.swap_id);
        }
        Ok((kp, me, have))
    }

    /// Decrypt + validate one snapshot blob. Returns the decoded record and the
    /// counter high-water mark stamped into the snapshot.
    fn rescue_decode(
        &self,
        kp: &bitcoin::secp256k1::Keypair,
        me: &str,
        blob: &str,
    ) -> Result<(RescuedRecord, u32)> {
        let env = crate::board::open_envelope(kp, blob)?;
        messages::verify(&env)?;
        ensure!(env.msg_type == "swapstate", "not a swapstate snapshot");
        ensure!(env.from == me, "snapshot is not ours");
        let next_index = env
            .body
            .get("next_index")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as u32;
        let rec_val = env.body.get("record").context("snapshot has no record")?;
        let rec = match env.body.get("v").and_then(|x| x.as_u64()) {
            Some(1) => RescuedRecord::V1(serde_json::from_value(rec_val.clone())?),
            Some(2) => RescuedRecord::V2(serde_json::from_value(rec_val.clone())?),
            _ => bail!("unknown snapshot version"),
        };
        Ok((rec, next_index))
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

    /// Fund our leg AND relay the resulting `funded` envelope to the
    /// counterparty — what the auto-fund path does. The plain [`Engine::fund`]
    /// only broadcasts and RETURNS the envelope; a manual fund (e.g. a recovery
    /// via the `fund` RPC) must also notify, else the maker is never told and
    /// falls back to chain-watch (#5). Best-effort relay: the counterparty also
    /// chain-watches, so a relay hiccup doesn't strand the swap.
    pub fn fund_and_notify(&self, swap: &str) -> Result<(SwapRecord, Envelope)> {
        let (record, envelope) = self.fund(swap)?;
        if let Some(cp) = record.counterparty_identity.clone() {
            let _ = self.relay_send_all(&cp, &envelope);
        }
        Ok((record, envelope))
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
        validate_offer_offsets(network, t1_secs, t2_secs)?;
        let proto = resolve_offer_protocol(&give.0, &get.0, network, protocol)?;
        // Don't advertise a swap we can't service: both legs' nodes must be live.
        let chain_a = ChainRef {
            coin_id: give.0.clone(),
            network,
        };
        let chain_b = ChainRef {
            coin_id: get.0.clone(),
            network,
        };
        self.ensure_chains_live(&[&chain_a, &chain_b])?;
        // We fund the `give` leg when this offer is taken — don't advertise a
        // swap the core wallet can't cover, and don't advertise more than the
        // wallet can cover across all our live offers in this coin combined.
        self.ensure_can_fund_new_offer(network, &give.0, give.1)?;
        let body = crate::board::OfferBody {
            wire: crate::wire_epoch(&proto),
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
        let offer_id = offer_id.context("no boards accepted the offer")?;
        // Register in our own ledger (offer-lifecycle): the scheduler re-publishes
        // it to roll the relay TTL forward, and graceful shutdown revokes it.
        // `valid_for` mirrors expired()'s effective lifetime, so
        // `created + valid_for` is the FINAL expiry (after which we stop refreshing).
        self.store.my_offer_put(
            &offer.swap_id,
            &serde_json::to_string(&offer)?,
            body.created,
            ttl_secs.unwrap_or(24 * 3600),
            local_now(),
        )?;
        Ok(offer_id)
    }

    /// Withdraw an offer: signed revocation to every board (the listing
    /// disappears immediately) AND a local block, so a taker replaying
    /// the saved signed offer afterwards is refused. Withdrawing commits
    /// nothing — offers never lock funds.
    pub fn revoke_board_offer(&self, offer_id: &str) -> Result<()> {
        let revocation = self.signed_envelope("revoke", offer_id, serde_json::json!({}))?;
        self.store
            .meta_set(&format!("offer_revoked:{offer_id}"), "1")?;
        // Reflect in our own ledger so refresh skips it and the My-offers view
        // shows it withdrawn — but only from `live`, so the auto-revoke that fires
        // when a take commits doesn't overwrite the `taken` state. No-op if the
        // offer predates the registry.
        self.store.my_offer_mark_revoked(offer_id)?;
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

    /// Terminally revoke every live offer whose pair involves `coin_id` (either
    /// leg). Called at reconfigure time when a coin is removed — those offers can
    /// no longer be honored, so withdraw them explicitly (the relaunch's skip-of-
    /// de-list leaves the SURVIVING offers listed). Returns the revoked offer ids.
    /// Best-effort per offer: a board rejecting one revocation doesn't abort the
    /// rest, so a removed coin never leaves a serveable-looking listing behind.
    pub fn revoke_offers_for_coin(&self, coin_id: &str) -> Result<Vec<String>> {
        let mut revoked = Vec::new();
        for o in self.store.my_offers_live()? {
            let Ok(env) = serde_json::from_str::<Envelope>(&o.envelope) else {
                continue; // malformed local row — skip, don't block the removal
            };
            let Ok(body) = serde_json::from_value::<crate::board::OfferBody>(env.body) else {
                continue;
            };
            if body.give_asset == coin_id || body.get_asset == coin_id {
                if let Err(err) = self.revoke_board_offer(&o.offer_id) {
                    eprintln!(
                        "warning: revoke-on-coin-remove failed for {}: {err:#}",
                        o.offer_id
                    );
                }
                revoked.push(o.offer_id);
            }
        }
        Ok(revoked)
    }

    /// How often a live offer is re-published to roll its relay TTL forward.
    /// Must stay well under `pact_nostr::RELAY_TTL_SECS` (30 min) so a listing
    /// never lapses between refreshes while the maker is online.
    const REFRESH_SECS: u64 = 10 * 60;

    /// Offer-lifecycle maintenance, called every scheduler tick (it self-gates
    /// per offer via `last_refresh`, so it is cheap):
    ///  - past the maker-set FINAL expiry (`created + valid_for`) → retire the
    ///    offer (mark `expired` + de-list everywhere);
    ///  - otherwise, every `REFRESH_SECS`, re-publish the stored signed offer so
    ///    the Nostr listing's rolling relay TTL advances (addressable replace by
    ///    d-tag = swap_id). HTTP corkboards are stateless and keep a listing until
    ///    revoked, so they need no refresh.
    pub fn refresh_offers(&self) -> Result<Vec<TickEvent>> {
        let now = local_now();
        let mut events = Vec::new();
        for o in self.store.my_offers_live()? {
            let final_expiry = o.created.saturating_add(o.valid_for);
            // valid_for == 0 means "no expiry" — skip those.
            if o.valid_for != 0 && now >= final_expiry {
                // Mark expired FIRST so the de-list's auto-mark-revoked is a no-op
                // (it only flips `live`), keeping the terminal state `expired`.
                self.store.my_offer_set_state(&o.offer_id, "expired")?;
                if let Err(err) = self.revoke_board_offer(&o.offer_id) {
                    eprintln!("warning: could not de-list expired offer: {err:#}");
                }
                events.push(TickEvent {
                    swap_id: o.offer_id.clone(),
                    action: "offer-expired".into(),
                    detail: "past valid-for".into(),
                });
                continue;
            }
            if now.saturating_sub(o.last_refresh) >= Self::REFRESH_SECS {
                let offer: Envelope = serde_json::from_str(&o.envelope)?;
                for (name, board) in self.boards()? {
                    if name == "nostr" {
                        let _ = board.post_offer(&offer); // queues a fresh-TTL replace
                    }
                }
                self.store.my_offer_touch_refresh(&o.offer_id, now)?;
            }
        }
        Ok(events)
    }

    /// Revoke every still-live offer — called on graceful shutdown (revoke-on-close)
    /// so a maker who quits cleanly stops advertising offers they can no longer
    /// honor. A crash skips this; the short relay TTL then drops the listings
    /// within `pact_nostr::RELAY_TTL_SECS`. Returns how many were withdrawn.
    pub fn revoke_live_offers(&self) -> Result<usize> {
        let live = self.store.my_offers_live()?;
        let n = live.len();
        for o in live {
            if let Err(err) = self.revoke_board_offer(&o.offer_id) {
                eprintln!(
                    "warning: revoke-on-close failed for {}: {err:#}",
                    o.offer_id
                );
            }
        }
        Ok(n)
    }

    /// SOFT de-list of every live offer on a clean shutdown: tell the boards to
    /// drop the listing (so we don't advertise while offline) but keep the offer
    /// LIVE and unblocked locally, so the next startup re-advertises it
    /// ([`Self::readvertise_offers`]). This is the auto-on-close path; it is
    /// deliberately NOT terminal — unlike the user's explicit withdraw
    /// ([`Self::revoke_board_offer`]), which records a local block and marks the
    /// offer revoked for good.
    pub fn delist_live_offers(&self) -> Result<usize> {
        let live = self.store.my_offers_live()?;
        let n = live.len();
        for o in &live {
            let revocation =
                match self.signed_envelope("revoke", &o.offer_id, serde_json::json!({})) {
                    Ok(env) => env,
                    Err(err) => {
                        eprintln!(
                            "warning: delist-on-close sign failed for {}: {err:#}",
                            o.offer_id
                        );
                        continue;
                    }
                };
            for (_, board) in self.boards()? {
                let _ = board.revoke(&revocation); // best-effort; TTL drops it anyway
            }
        }
        Ok(n)
    }

    /// On startup, re-advertise still-valid offers — those soft-de-listed on the
    /// last clean close, or whose relay TTL lapsed while offline — so a maker who
    /// returns within an offer's `valid_for` resumes advertising instead of
    /// silently losing it. Re-posts the stored signed envelope to every board and
    /// rolls the relay TTL. Offers past their final expiry are skipped (the next
    /// `refresh_offers` retires them). Returns how many were re-advertised.
    pub fn readvertise_offers(&self) -> Result<usize> {
        let now = local_now();
        let mut n = 0;
        for o in self.store.my_offers_live()? {
            let final_expiry = o.created.saturating_add(o.valid_for);
            if o.valid_for != 0 && now >= final_expiry {
                continue; // expired — leave it for refresh_offers to retire
            }
            let offer: Envelope = match serde_json::from_str(&o.envelope) {
                Ok(env) => env,
                Err(_) => continue,
            };
            for (_, board) in self.boards()? {
                let _ = board.post_offer(&offer); // queues a fresh-TTL (re)listing
            }
            self.store.my_offer_touch_refresh(&o.offer_id, now)?;
            n += 1;
        }
        Ok(n)
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
        ensure!(
            body.wire == crate::wire_epoch(&body.protocol),
            "offer speaks {} wire v{}, this build speaks v{} — maker and taker must run compatible releases",
            body.protocol,
            body.wire,
            crate::wire_epoch(&body.protocol)
        );
        ensure!(!body.expired(local_now()), "offer has expired");
        ensure!(offer.from != self.identity()?, "that is our own offer");
        // Don't signal a take we can't honor: parse the offer's network, then
        // require both legs supported AND their nodes live before committing.
        let network = match body.network.as_str() {
            "regtest" => Network::Regtest,
            "testnet" => Network::Testnet,
            "mainnet" => Network::Mainnet,
            other => bail!("unsupported network in offer: {other}"),
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
        self.ensure_chains_live(&[&chain_a, &chain_b])?;
        // Taking means WE fund the maker's `get` leg — refuse if we can't.
        self.ensure_can_fund(network, &body.get_asset, body.get_amount)?;
        self.store
            .put_pending_take(offer_id, &serde_json::to_string(&offer)?, local_now())?;
        // `taken_at` (signed, being part of the body) lets the maker drop a
        // take that reaches it stale — after our pending take will have
        // pruned itself — instead of committing to a dead handshake.
        let take = self.signed_envelope(
            "take",
            offer_id,
            serde_json::json!({
                "offer": serde_json::to_value(&offer)?,
                "taken_at": local_now(),
                // The TAKER's wire epoch for this protocol — the maker gates
                // on it (an offer's own epoch only proves the maker's side).
                "wire": crate::wire_epoch(&body.protocol),
            }),
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
            let last_error = self
                .store
                .meta_get(&format!("take_last_error:{offer_id}"))?;
            out.push(PendingTakeInfo {
                offer_id,
                from: offer.from,
                body: offer.body,
                created_at,
                last_error,
            });
        }
        Ok(out)
    }

    // -----------------------------------------------------------------
    // Private (off-market) offers — the Pact handbook (private offers). A private offer is
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
        validate_offer_offsets(network, t1_secs, t2_secs)?;
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
        // No fund check here: `make_private_offer` is a pure builder (a slip can
        // be drafted offline). Fundability is hard-gated when the slip is taken
        // (after the chain-up gate) and again at `fund`.
        let proto = resolve_offer_protocol(&give.0, &get.0, network, protocol)?;

        let body = crate::board::OfferBody {
            wire: crate::wire_epoch(&proto),
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
        ensure!(
            body.wire == crate::wire_epoch(&body.protocol),
            "offer speaks {} wire v{}, this build speaks v{} — maker and taker must run compatible releases",
            body.protocol,
            body.wire,
            crate::wire_epoch(&body.protocol)
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
        self.ensure_chains_live(&[&chain_a, &chain_b])?;
        // Taking means WE fund the maker's `get` leg — refuse if we can't.
        self.ensure_can_fund(network, &body.get_asset, body.get_amount)?;

        self.store.put_pending_take(
            &offer.swap_id,
            &serde_json::to_string(&offer)?,
            local_now(),
        )?;
        // Same signed `taken_at` staleness stamp + taker wire as a board take.
        let take = self.signed_envelope(
            "take",
            &offer.swap_id,
            serde_json::json!({
                "offer": serde_json::to_value(&offer)?,
                "taken_at": local_now(),
                "wire": crate::wire_epoch(&body.protocol),
            }),
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
            let expiry = body.created + body.ttl_secs.unwrap_or(24 * 3600);
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
                        // Deterministic failure (validation/parse): retrying
                        // the same envelope can never succeed — one clear
                        // event, cursor advances, done. (The taker's init path
                        // already turned its own permanent failures into
                        // `take-failed` + a reasoned abort before this.)
                        Err(err) if is_permanent(&err) => {
                            events.push(TickEvent {
                                swap_id: envelope.swap_id.clone(),
                                action: "relay-error".into(),
                                detail: format!("{err:#}"),
                            });
                        }
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
    /// one outgoing message per call from the record + nonce state. v2 ALWAYS
    /// auto-funds (the Satchel auto-fund toggle gates v1 only): v2 funding is
    /// one step of an automated handshake, so pausing it for manual funding just
    /// wedges the swap. nonce/sign/assemble are safe to automate (no new funds);
    /// redeem is the scheduler's job (`tick`).
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

        // Handshake already complete (Signed or beyond): nothing left to
        // drive here — completion is the scheduler's chain-watch. Load-bearing
        // for rescue (#54): a restored record carries the assembled sigs but a
        // WIPED nonce store, so without this gate the replayed relay history
        // re-arms the nonce/partial steps below — re-opening a signing session
        // that nonce-safety has already dead-ended — and their early returns
        // starve the scheduler-driven redeem forever.
        if !matches!(
            rec.state,
            AdaptorState::Created | AdaptorState::Accepted | AdaptorState::NoncesExchanged
        ) {
            return ev("adaptor-recv", msg_type.into());
        }

        // 1. Fund my leg: initiator on `accept`; participant once leg A is in.
        // No `auto_fund` gate — v2 always auto-funds (see the fn doc): the
        // manual e2e drives via `adaptorrecv`, which never reaches this
        // autopilot, so production board swaps are the only caller here.
        if !self.adaptor_my_leg_funded(rec) {
            let ready = match rec.role {
                Role::Initiator => msg_type == "accept",
                // The participant BUILDS + pre-signs leg B on the funding_ready(A)
                // pointer (commits no funds — just a local unbroadcast tx and the
                // adaptor sigs). The CRITICAL safety gate is on the BROADCAST of
                // leg B (adaptor_tick_one): only once the swap is Signed (σ_A held)
                // AND leg A is verified on-chain n_a-deep. Alice cannot use σ_B
                // until B is on-chain, which the participant controls.
                Role::Participant => rec.funding_a_txid.is_some(),
            };
            if ready {
                // adaptor_fund routes by role: initiator broadcasts leg A;
                // participant BUILDS leg B unbroadcast (scheduler broadcasts it
                // post-Signed once leg A is verified n_a-deep).
                let fr = self.adaptor_fund(swap)?;
                self.relay_send_all(counterparty, &fr)?;
                let detail = if rec.role == Role::Initiator {
                    "broadcast leg A + funding_ready"
                } else {
                    "built leg B (unbroadcast) + funding_ready"
                };
                return ev("adaptor-fund", detail.into());
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

    /// Every incoming `abort`, routed by what `envelope.swap_id` resolves to:
    ///  1. a v1 record → the legacy [`Self::recv`] path (advisory: flips the
    ///     state only while neither HTLC is funded);
    ///  2. a v2 adaptor record → flip to `Aborted` iff the sender is the
    ///     pinned counterparty and OUR leg is unfunded (after funding the
    ///     timelocks are the safety, same rule as v1);
    ///  3. an offer WE served → resolve `offer_served:<offer_id>` to the swap
    ///     record the take created and abort that (the taker cancels by OFFER
    ///     id when the handshake died before it ever learned the swap id);
    ///  4. a pending take WE sent → the maker rejected the take; drop it;
    ///  5. none of ours → ignore (junk).
    fn recv_abort(&self, envelope: &Envelope) -> Result<Option<TickEvent>> {
        let event = |swap_id: &str, action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: swap_id.into(),
                action: action.into(),
                detail,
            }))
        };
        messages::verify(envelope)?;
        let reason = envelope.body["reason"]
            .as_str()
            .unwrap_or("unspecified")
            .to_string();
        // 1. v1 record (recv re-checks the pinned counterparty).
        if self.store.get(&envelope.swap_id).is_ok() {
            let rec = self.recv(envelope)?;
            return event(
                &rec.swap_id,
                "recv",
                format!("counterparty abort: {reason}"),
            );
        }
        // 2. v2 record, directly by swap id.
        if self.store.get_adaptor(&envelope.swap_id).is_ok() {
            return self.abort_record_by_peer(&envelope.swap_id, &envelope.from, &reason);
        }
        // 3. An offer we served: the take-committed record carries a fresh
        //    swap id the taker may never have learned (init lost) — the
        //    served-marker maps offer id -> swap id.
        if let Some(swap_id) = self
            .store
            .meta_get(&format!("offer_served:{}", envelope.swap_id))?
        {
            return self.abort_record_by_peer(&swap_id, &envelope.from, &reason);
        }
        // 4. A pending take of ours the maker is rejecting.
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
        event(&envelope.swap_id, "take-failed", reason)
    }

    /// Abort the (v1 or v2) record `swap_id` on a counterparty's signed
    /// abort: only the pinned counterparty is honored, and only while the
    /// pre-funding gate holds (v1: neither HTLC funded; v2: our leg
    /// unfunded) — funded swaps ignore aborts, timelocks are the safety.
    fn abort_record_by_peer(
        &self,
        swap_id: &str,
        sender: &str,
        reason: &str,
    ) -> Result<Option<TickEvent>> {
        let event = |action: &str, detail: String| {
            Ok(Some(TickEvent {
                swap_id: swap_id.into(),
                action: action.into(),
                detail,
            }))
        };
        if let Ok(mut rec) = self.store.get(swap_id) {
            ensure!(
                rec.counterparty_identity.as_deref() == Some(sender),
                "abort signed by {sender} but counterparty pinned otherwise (spec §8.2)"
            );
            if rec.htlc_a_txid.is_none() && rec.htlc_b_txid.is_none() {
                rec.state = State::Aborted;
                self.store.put(&rec)?;
                let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
                return event("counterparty-abort", reason.into());
            }
            return event(
                "counterparty-abort",
                "ignored (funded; timelocks protect)".into(),
            );
        }
        let mut rec = self.store.get_adaptor(swap_id)?;
        ensure!(
            rec.counterparty_identity.as_deref() == Some(sender),
            "abort signed by {sender} but counterparty pinned otherwise (spec §8.2)"
        );
        // Same commitment semantics as adaptor_abort: the participant's leg B
        // being merely BUILT (never broadcast, spec v2 §7) does not lock funds,
        // so the peer's abort can be honored — releasing the built tx's inputs.
        let our_leg_committed = match rec.role {
            Role::Initiator => rec.funding_a_txid.is_some(),
            Role::Participant => {
                rec.funding_b_txid.is_some() && !self.adaptor_leg_b_uncommitted(&rec)
            }
        };
        if our_leg_committed {
            return event(
                "counterparty-abort",
                "ignored (funded; timelocks protect)".into(),
            );
        }
        let _ = self.adaptor_cancel_built_leg_b(&rec); // release reserved inputs
        rec.state = AdaptorState::Aborted;
        self.store.put_adaptor(&rec)?;
        let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
        event("counterparty-abort", reason.into())
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
                // Staleness gate FIRST, before ANY side effect (serving,
                // revoking, record creation): a take older than the taker's
                // own pending-take prune window is a handshake the taker has
                // certainly given up on — e.g. it sat queued while our node
                // was unreachable. Acting on it would burn the offer and
                // strand a record on a counterparty that stopped listening.
                // Dropped SILENTLY (no reject envelope — the taker's card
                // pruned itself long ago, nobody is listening); the local
                // event is the only trace. `taken_at` is REQUIRED (this
                // product has never shipped a take without it); a missing or
                // unparsable stamp is treated as stale. A FUTURE stamp
                // (taker clock ahead) saturates to age 0 = fresh, so honest
                // clock skew within the 15-min window is tolerated.
                let taken_at = envelope.body["taken_at"].as_u64().unwrap_or(0);
                if local_now().saturating_sub(taken_at) >= PRE_FUNDING_TIMEOUT_SECS {
                    return event(
                        &offer.swap_id,
                        "take-stale",
                        format!(
                            "dropped a take older than {PRE_FUNDING_TIMEOUT_SECS}s (taker gave up); offer stays live"
                        ),
                    );
                }
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
                // Wire gate: a taker on a different wire epoch cannot complete
                // the handshake — refuse before serving so the offer stays
                // live and the taker gets a clear reason (a pre-rc10 taker
                // sends no `wire`, which parses as epoch 1).
                let take_wire = envelope.body["wire"].as_u64().unwrap_or(1) as u32;
                let our_wire = crate::wire_epoch(&body.protocol);
                if take_wire != our_wire {
                    self.reject_take(
                        &envelope.from,
                        &offer.swap_id,
                        "incompatible release (protocol wire version) — please update Satchel",
                    )?;
                    return event(
                        &offer.swap_id,
                        "take-rejected",
                        format!("taker wire v{take_wire}, ours v{our_wire}; offer stays live"),
                    );
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
                    "mainnet" => Network::Mainnet,
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
                // Mark our own offer taken before the C5 auto-revoke below (which
                // only flips `live` offers, so this `taken` survives).
                self.store.my_offer_set_state(&offer.swap_id, "taken")?;
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
                let (offer_id, offer) = self
                    .match_pending_take(&envelope.from, echoed_offer_id)?
                    .context("init from a maker we have no pending take with")
                    .map_err(permanent_err)?;
                // Build the accept; classify any failure. Deterministic
                // (permanent-tagged) failures — wire mismatch, §7.3 violations,
                // malformed bodies — can never succeed on retry: drop the
                // pending take NOW, tell the maker WHY (reasoned abort, so
                // their swap dies in seconds instead of at the 900s reaper),
                // and surface one clear `take-failed` event. Transient
                // failures (backend/seed access) record themselves for the
                // take-timeout message and bubble into the retry loop.
                let attempt = (|| -> Result<(String, Envelope)> {
                    let body: crate::board::OfferBody = serde_json::from_value(offer.body.clone())
                        .map_err(|e| {
                            permanent_err(anyhow::Error::new(e).context("malformed offer body"))
                        })?;
                    // The maker must honor their own advert. Compare against
                    // the same chain-aware "now" the maker used.
                    let chain_a: ChainRef =
                        serde_json::from_value(envelope.body["chain_a"].clone())
                            .context("init without chain_a")
                            .map_err(permanent_err)?;
                    let chain_b: ChainRef =
                        serde_json::from_value(envelope.body["chain_b"].clone())
                            .context("init without chain_b")
                            .map_err(permanent_err)?;
                    let now = self.coordination_now(&chain_a, &chain_b)?;
                    crate::board::init_matches_offer(&envelope.body, &body, now)
                        .map_err(permanent_err)?;
                    // Branch on the init protocol: v2 builds an adaptor accept.
                    let is_v2 = envelope.body["protocol"].as_str()
                        == Some(crate::adaptor_swap::PROTOCOL_V2);
                    if is_v2 {
                        let (rec, accept) = self.adaptor_accept(envelope)?;
                        Ok((rec.swap_id, accept))
                    } else {
                        let (rec, accept) = self.accept(envelope)?;
                        Ok((rec.swap_id, accept))
                    }
                })();
                let last_error_key = format!("take_last_error:{offer_id}");
                match attempt {
                    Ok((swap_id, accept)) => {
                        self.store.remove_pending_take(&offer_id)?;
                        let _ = self.store.meta_del(&last_error_key);
                        self.relay_send_all(&envelope.from, &accept)?;
                        event(&swap_id, "init->accept", format!("offer {offer_id}"))
                    }
                    Err(err) if is_permanent(&err) => {
                        self.store.remove_pending_take(&offer_id)?;
                        let _ = self.store.meta_del(&last_error_key);
                        let abort = self.signed_envelope(
                            "abort",
                            &envelope.swap_id,
                            serde_json::json!({
                                "reason": format!("take failed on the taker's side: {err:#}")
                            }),
                        )?;
                        // Best-effort: even if the abort doesn't land, the
                        // maker's C8 pre-funding reaper cleans up at 900s.
                        let _ = self.relay_send_all(&envelope.from, &abort);
                        event(
                            &envelope.swap_id,
                            "take-failed",
                            format!("offer {offer_id}: {err:#}"),
                        )
                    }
                    Err(err) => {
                        // Transient: remember the cause so a later take-timeout
                        // reports the truth, then let the relay loop retry.
                        let _ = self.store.meta_set(&last_error_key, &format!("{err:#}"));
                        Err(err)
                    }
                }
            }
            // Every abort — take rejections, counterparty cancels (by swap
            // id OR by offer id), v1 and v2 — routes through one resolver.
            "abort" => self.recv_abort(envelope),
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
            "accept" | "funded" | "redeemed" => {
                if self.store.get_adaptor(&envelope.swap_id).is_ok() {
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
        fee: SendFee,
    ) -> Result<String> {
        let backend = self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?;
        // The address must belong to this chain — catches pasting a BTC
        // address into the POCX send form before money moves.
        backend.params().parse_address(address)?;
        backend.wallet_send(address, amount_sat, fee)
    }

    /// Sweep the coin's whole wallet to `address` (the send form's "send
    /// everything"): fee comes out of the swept amount, wallet ends empty.
    pub fn wallet_send_all(
        &self,
        network: Network,
        coin_id: &str,
        address: &str,
        fee: SendFee,
    ) -> Result<String> {
        let backend = self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?;
        backend.params().parse_address(address)?;
        backend.wallet_send_all(address, fee)
    }

    /// Fee estimates for the send form's Slow/Normal/Fast presets (144/6/1
    /// blocks, phoenix parity), plus the coin's feerate floor. A preset is
    /// `None` when the estimator has no data for that target — the form
    /// disables it and, when ALL are `None`, falls back to a custom rate at
    /// the floor.
    pub fn wallet_fee_estimates(
        &self,
        network: Network,
        coin_id: &str,
    ) -> Result<crate::chain::SendFeeEstimates> {
        let backend = self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?;
        // Decimal sat/vB at the estimator's full sat/kvB resolution — the
        // fraction (1080 sat/kvB → 1.08) is real queue priority and the UI
        // shows it verbatim.
        let vb = |kvb: Option<u64>| kvb.map(|k| k as f64 / 1000.0);
        Ok(crate::chain::SendFeeEstimates {
            min_sat_per_vb: backend.params().min_feerate_sat_vb.max(1) as f64,
            fast: vb(backend.fee_estimate_kvb(1)?),
            normal: vb(backend.fee_estimate_kvb(6)?),
            slow: vb(backend.fee_estimate_kvb(144)?),
        })
    }

    /// RBF-bump a wallet-owned unconfirmed send to `sat_per_vb`, returning the
    /// replacement txid. Every wallet send is broadcast BIP125-replaceable, so
    /// this is the "stuck tx" lever behind the Activity dialog's Bump fee
    /// action (nodeless coins; a node-backed coin bumps in the node's own
    /// wallet, which owns the tx).
    ///
    /// A LIVE SWAP'S FUNDING is refused: the funding nurse owns its fee — v1
    /// re-RBFs it under the swap's FeeBumpPolicy, and v2 deliberately CPFPs
    /// because replacing the funding would change its txid and invalidate the
    /// pre-signed MuSig2 redeems (the funding outpoint is committed into the
    /// adaptor signatures). A hand-RBF of a v2 funding could strand the swap.
    pub fn wallet_bumpfee(
        &self,
        network: Network,
        coin_id: &str,
        txid: &str,
        sat_per_vb: u64,
    ) -> Result<String> {
        let live_v1 = |s: &State| !matches!(s, State::Completed | State::Refunded | State::Aborted);
        let live_v2 = |s: &AdaptorState| {
            !matches!(
                s,
                AdaptorState::Completed | AdaptorState::Refunded | AdaptorState::Aborted
            )
        };
        for r in self.store.list()? {
            if live_v1(&r.state)
                && (r.htlc_a_txid.as_deref() == Some(txid)
                    || r.htlc_b_txid.as_deref() == Some(txid))
            {
                bail!(
                    "{txid} funds live swap {} — the swap engine manages its fee \
                     (see get/setfeepolicy), bumpfee must not replace it",
                    r.swap_id
                );
            }
        }
        for r in self.store.list_adaptor()? {
            if live_v2(&r.state)
                && (r.funding_a_txid.as_deref() == Some(txid)
                    || r.funding_b_txid.as_deref() == Some(txid))
            {
                bail!(
                    "{txid} funds live swap {} — the swap engine manages its fee \
                     (see get/setfeepolicy), bumpfee must not replace it",
                    r.swap_id
                );
            }
        }
        self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?
        // The RPC surface takes whole sat/vB from the user; the backend bumps at
        // sat/kvB resolution.
        .wallet_bumpfee(txid, sat_per_vb.saturating_mul(1000))
    }

    /// The wallet activity feed for a nodeless coin (`listtransactions`, design
    /// doc §4) — newest first, straight off the bdk tx graph. Refuses for
    /// Core-backed coins: Satchel keeps those read-only by design (the node's
    /// own wallet is the operator's tool).
    pub fn wallet_transactions(
        &self,
        network: Network,
        coin_id: &str,
    ) -> Result<Vec<crate::chain::WalletTxInfo>> {
        self.backend(&ChainRef {
            coin_id: coin_id.to_string(),
            network,
        })?
        .wallet_transactions()
    }

    /// Live fee rate (sat/vB) for a configured coin, or the same conservative
    /// fallback the backends use when a coin is unconfigured/unreachable. The
    /// `bool` is `true` when the rate is the fallback (the UI flags it as a
    /// guess) rather than a live estimate. Never errors — a fee *preview* must
    /// not fail just because one node is down.
    fn fee_rate_or_fallback(&self, network: Network, coin_id: &str) -> (u64, bool) {
        // Mirrors the per-backend fallback (chain.rs FALLBACK_SAT_PER_VB).
        const FALLBACK_SAT_PER_VB: u64 = 1;
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
        let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
                                                   // Best-effort notify — an explicit user cancel is the ONE case that
                                                   // sends an abort envelope (automatic timeouts stay silent; the
                                                   // counterparty's own pre-funding clock clears their side). Not gated
                                                   // on board_url: relay_send_all fans out over whatever boards exist.
        if let Some(counterparty) = &rec.counterparty_identity {
            let abort = self.signed_envelope(
                "abort",
                &rec.swap_id,
                serde_json::json!({ "reason": reason }),
            )?;
            let _ = self.relay_send_all(counterparty, &abort);
        }
        Ok(rec)
    }

    /// v2 twin of [`Self::abort`]: back out of an adaptor swap while OUR leg
    /// is still uncommitted. Once our funding is on-chain the timelocked
    /// refund is the only safe exit, so this refuses. The participant's leg B
    /// being merely BUILT (txid known from accept, spec v2 §7) does NOT
    /// commit it — an abort then also releases the built tx's input
    /// reservation. Persisted nonce sessions are deliberately KEPT — the
    /// store's overwrite refusal is what guarantees an aborted swap can
    /// never sign again.
    pub fn adaptor_abort(&self, swap: &str, reason: &str) -> Result<AdaptorSwapRecord> {
        let mut rec = self.store.get_adaptor(swap)?;
        let our_leg_committed = match rec.role {
            Role::Initiator => rec.funding_a_txid.is_some(),
            Role::Participant => {
                rec.funding_b_txid.is_some() && !self.adaptor_leg_b_uncommitted(&rec)
            }
        };
        ensure!(
            !our_leg_committed,
            "cannot abort: our leg is funded — use the timelocked refund instead"
        );
        let _ = self.adaptor_cancel_built_leg_b(&rec); // release reserved inputs
        rec.state = AdaptorState::Aborted;
        self.store.put_adaptor(&rec)?;
        let _ = self.tombstone_swap(&rec.swap_id); // terminal (#54)
        if let Some(counterparty) = &rec.counterparty_identity {
            let abort = self.signed_envelope(
                "abort",
                &rec.swap_id,
                serde_json::json!({ "reason": reason }),
            )?;
            let _ = self.relay_send_all(counterparty, &abort);
        }
        Ok(rec)
    }

    /// Back out of a take we sent that the maker never answered — the UI's
    /// Cancel on an "initiating" pre-swap. Removes the pending take and
    /// best-effort relays an `abort` keyed by the OFFER id; the maker
    /// resolves that to its swap record via the `offer_served` marker
    /// ([`Self::recv_abort`]). The maker's own pre-funding timeout is the
    /// fallback when the envelope never arrives.
    pub fn cancel_pending_take(&self, offer_id: &str) -> Result<()> {
        let (_, offer_json) = self
            .store
            .pending_takes()?
            .into_iter()
            .find(|(id, _)| id == offer_id)
            .with_context(|| format!("no pending take for offer {offer_id}"))?;
        let offer: Envelope = serde_json::from_str(&offer_json)?;
        self.store.remove_pending_take(offer_id)?;
        let abort = self.signed_envelope(
            "abort",
            offer_id,
            serde_json::json!({ "reason": "taker cancelled" }),
        )?;
        let _ = self.relay_send_all(&offer.from, &abort);
        Ok(())
    }
}

/// The scriptPubKey our final spend pays (output 0 of the stored tx) —
/// the script hint Electrum backends need to locate the transaction.
fn spend_spk(rec: &SwapRecord) -> Option<bitcoin::ScriptBuf> {
    first_output_spk(rec.final_tx_hex.as_deref())
}

/// The first-output scriptPubKey of a serialized tx (our HTLC spends are 1-out),
/// used as a backend lookup hint. `None` if the hex is absent/corrupt.
fn first_output_spk(tx_hex: Option<&str>) -> Option<bitcoin::ScriptBuf> {
    let bytes = hex::decode(tx_hex?).ok()?;
    let tx: bitcoin::Transaction = bitcoin::consensus::encode::deserialize(&bytes).ok()?;
    Some(tx.output.first()?.script_pubkey.clone())
}

/// Effective feerate (sat/vB) of a 1-in/1-out settlement tx: `(claimed − output)
/// / vsize`. `None` if the hex is absent/corrupt or the arithmetic underflows.
fn feerate_of(tx_hex: Option<&str>, claimed_amount: u64) -> Option<u64> {
    let bytes = hex::decode(tx_hex?).ok()?;
    let tx: bitcoin::Transaction = bitcoin::consensus::encode::deserialize(&bytes).ok()?;
    let out = tx.output.first()?.value.to_sat();
    let fee = claimed_amount.checked_sub(out)?;
    let vsize = tx.vsize() as u64;
    (vsize > 0).then(|| fee / vsize)
}

/// Display symbol for a chain (e.g. `btcx` → `BTCX`), for "Securing your {coin}".
fn coin_symbol(chain: &ChainRef) -> String {
    chain.coin_id.to_uppercase()
}

/// Live per-swap progress for the UI (observability only). Rebuilt each
/// [`Engine::tick`] and served by `swapprogress`; see the field doc on
/// [`Engine::progress`]. Secret-free by construction (counts, txid-derived data,
/// feerate, the latest scheduler action text — no preimage or keys).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SwapProgress {
    pub swap_id: String,
    /// The current wait, and how to display it:
    /// - `awaiting_lock` / `awaiting_claim` — waiting on the COUNTERPARTY to act
    ///   (their lock to appear, or their claim/reveal). No target; show
    ///   `blocks_elapsed` as a liveness count.
    /// - `their_lock` / `our_lock` — a lock burying toward `needed` (their lock
    ///   as our gate, or our own lock confirming). Show `confs/needed`.
    /// - `funding` — OUR funding of this leg is pending/retrying (#3); show
    ///   `blocks_elapsed` (a growing count flags a stuck fund, e.g. a locked wallet).
    /// - `settlement` — our own claim burying ("Securing your {coin}").
    ///   Show `confs/needed` (+ `feerate_sat_vb`).
    pub watching: String,
    /// Display symbol of the watched leg (e.g. `BTC`).
    pub coin: String,
    /// Confirmations so far — for the `their_lock`/`settlement` phases (`0` for
    /// the awaiting phases).
    pub confs: u32,
    /// Required depth for this leg (`n_a`/`n_b`); `0` for the awaiting phases.
    pub needed: u32,
    /// Blocks elapsed in the current awaiting phase (liveness cue, no deadline).
    /// Present only for `awaiting_lock`/`awaiting_claim`/`funding`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks_elapsed: Option<u32>,
    /// Current feerate of our settlement tx (sat/vB); `settlement` phase only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feerate_sat_vb: Option<u64>,
    /// The most recent scheduler action for this swap, if any this tick.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_detail: Option<String>,
    /// When this snapshot was taken (unix seconds) — lets the UI grey out stale data.
    pub updated_at: u64,
    /// Internal carry-forward: the chain tip when the awaiting phase began, so
    /// `blocks_elapsed` grows across ticks. The UI ignores it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub awaiting_since_height: Option<u32>,
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
    /// Why the maker's init last failed to process (transient failures only —
    /// permanent ones drop the take immediately). `None` = no init seen yet.
    pub last_error: Option<String>,
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

        // …but opting into v2 is allowed for a Taproot pair on every network now
        // that v2+ is mainnet-enabled (resolve_offer_protocol relies on this).
        // A pair without Taproot on both legs still can't run v2.
        assert!(adaptor_offer_allowed("btcx", "btc", Network::Regtest));
        assert!(adaptor_offer_allowed("btcx", "btc", Network::Testnet));
        assert!(adaptor_offer_allowed("btcx", "btc", Network::Mainnet));
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

        // Mainnet v2+ is enabled now (ADAPTOR_MAINNET_ENABLED) — init succeeds.
        assert!(alice
            .adaptor_init(
                Network::Mainnet,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2
            )
            .is_ok());

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
        // The view reports (effective, default, min) for the setup UI.
        assert_eq!(
            engine
                .coin_confirmations_view(Network::Regtest, "btc")
                .unwrap(),
            (4, 1, 1)
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
        let (brec, accept) = bob.adaptor_accept(&init).unwrap();
        // Bob resolves from his config: pocx override 7, btc default 1.
        assert_eq!((brec.n_a, brec.n_b), (7, 1));
        // rc12 recut: the depths are also EXCHANGED as advisory display
        // values — Bob's record knows Alice's, and once Alice processes the
        // accept her record knows Bob's. Gates keep using own values.
        assert_eq!((brec.their_n_a, brec.their_n_b), (Some(1), Some(5)));
        let arec2 = alice.recv_adaptor(&accept).unwrap();
        assert_eq!((arec2.n_a, arec2.n_b), (1, 5), "own gates unchanged");
        assert_eq!((arec2.their_n_a, arec2.their_n_b), (Some(7), Some(1)));

        // No backward compat: record fields are required — a blob missing one
        // (e.g. a pre-depth record without n_a) no longer silently defaults, it
        // fails to load. A full record round-trips.
        let full = serde_json::to_value(&arec).unwrap();
        assert!(serde_json::from_value::<crate::store::AdaptorSwapRecord>(full.clone()).is_ok());
        let mut v = full;
        v.as_object_mut().unwrap().remove("n_a");
        assert!(
            serde_json::from_value::<crate::store::AdaptorSwapRecord>(v).is_err(),
            "a record missing a required field must not deserialize"
        );

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    #[test]
    fn confirmations_clamp_into_spec_band_on_mainnet() {
        // rc12 recut: floor 2 / cap = chain default on mainnet — a config
        // outside the band CLAMPS instead of poisoning every future
        // handshake (the 2026-07-08 incident: btc=2 under the old ≥6 floor).
        let (mut engine, dir) = engine_with("confs-band", None);
        let btc = ChainRef {
            coin_id: "btc".into(),
            network: Network::Mainnet,
        };
        let btcx = ChainRef {
            coin_id: "btcx".into(),
            network: Network::Mainnet,
        };
        // In-band values stand as configured (2 is legal now).
        engine.coin_confirmations.insert("btc".into(), 2);
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 2);
        // Below the floor → raised to 2; above the default → capped at it.
        engine.coin_confirmations.insert("btc".into(), 1);
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 2);
        engine.coin_confirmations.insert("btc".into(), 50);
        assert_eq!(engine.confirmations_for(&btc).unwrap(), 6);
        engine.coin_confirmations.insert("btcx".into(), 50);
        assert_eq!(engine.confirmations_for(&btcx).unwrap(), 10);
        // The listcoins view exposes (effective, default==max, min).
        assert_eq!(
            engine
                .coin_confirmations_view(Network::Mainnet, "btc")
                .unwrap(),
            (6, 6, 2)
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn out_of_band_advisory_depth_is_rejected_permanently() {
        // A peer advertising a depth outside [2, default] is a foreseeable
        // liveness stall — the participant refuses the init up-front, tagged
        // permanent so the relay loop fails fast instead of retrying 10×.
        let (alice, ad) = engine_with("v2-band-alice", None);
        let (bob, bd) = engine_with("v2-band-bob", None);
        let now = local_now() as u32;
        // A valid mainnet profile: T2 = now+12h, T1 = now+24h.
        let (t1, t2) = (now + 24 * 3600, now + 12 * 3600);
        let (_arec, init) = alice
            .adaptor_init(
                Network::Mainnet,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2,
            )
            .unwrap();
        // Sanity: the honest init passes.
        assert!(bob.adaptor_accept(&init).is_ok());

        // Tamper the advisory N_A to 50 (> btcx cap 10) and re-sign as Alice
        // (the signature is over the body, so tampering must re-sign).
        let mut evil = init.clone();
        evil.body["n_a"] = serde_json::json!(50u32);
        messages::sign(
            &mut evil,
            &alice.store.seed().unwrap().identity_keypair().unwrap(),
        )
        .unwrap();
        let err = bob.adaptor_accept(&evil).unwrap_err();
        assert!(is_permanent(&err), "band violation must be permanent");
        assert!(
            format!("{err:#}").contains("advisory N_A"),
            "error names the offending value: {err:#}"
        );

        // A wire-epoch mismatch fails the same way — loud and permanent.
        let mut old_peer = init.clone();
        old_peer.body["wire"] = serde_json::json!(2u32);
        messages::sign(
            &mut old_peer,
            &alice.store.seed().unwrap().identity_keypair().unwrap(),
        )
        .unwrap();
        let err = bob.adaptor_accept(&old_peer).unwrap_err();
        assert!(is_permanent(&err), "wire mismatch must be permanent");
        assert!(
            format!("{err:#}").contains("compatible releases"),
            "error tells the user to update: {err:#}"
        );

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    #[test]
    fn v1_taker_derives_own_depths_and_stores_makers_as_advisory() {
        // rc12 recut: the v1 taker no longer adopts the maker's n_a/n_b from
        // the init body — it derives its own from local config (per-side
        // ownership, matching v2) and keeps the maker's as display advisory.
        let (alice, ad) = engine_with("v1-perside-alice", None);
        let (mut bob, bd) = engine_with("v1-perside-bob", None);
        bob.coin_confirmations.insert("btcx".into(), 4);
        let now = local_now() as u32;
        let (t1, t2) = (now + 40_000, now + 20_000);
        let (arec, init) = alice
            .offer(
                Network::Regtest,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                t1,
                t2,
                None,
                None,
            )
            .unwrap();
        // Alice (regtest defaults): 1/1.
        assert_eq!((arec.n_a, arec.n_b), (1, 1));
        let (brec, accept) = bob.accept(&init).unwrap();
        // Bob derives his OWN: btcx override 4, btc default 1 — NOT the
        // maker's body values — and stores Alice's as advisory.
        assert_eq!((brec.n_a, brec.n_b), (4, 1));
        assert_eq!((brec.their_n_a, brec.their_n_b), (Some(1), Some(1)));
        // Alice learns Bob's depths from the accept.
        let arec2 = alice.recv(&accept).unwrap();
        assert_eq!((arec2.their_n_a, arec2.their_n_b), (Some(4), Some(1)));
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
    fn maker_signed_phase_tracks_observation_not_txid_presence() {
        use MakerSignedPhase::*;
        // Regression for the "Both locked / Their lock confirming · 0/10" bug:
        // leg B's funding txid is BUILT at accept, so it is always present by
        // `Signed`. The phase must be chosen from OBSERVATION of the output, so
        // the maker's display walks: our leg A confirming → awaiting the taker's
        // lock → their lock confirming — never jumping straight to `their_lock`
        // (which narrate() renders as a false "Both locked") before leg B is
        // actually broadcast.

        // Leg B not observed (never broadcast) while our leg A buries toward
        // n_a: this is the exact live-swap state the user hit — must be
        // `our_lock`, NOT `their_lock`. (n_a = 6 on mainnet BTC → "Your lock
        // confirming · 0/6".)
        assert_eq!(maker_signed_phase(None, Some(0), 6), OurLockA);
        assert_eq!(maker_signed_phase(None, Some(5), 6), OurLockA);

        // Our leg A is n_a-deep but leg B still isn't observed: we're genuinely
        // waiting on the taker to broadcast → an honest liveness wait.
        assert_eq!(maker_signed_phase(None, Some(6), 6), AwaitingLock);
        assert_eq!(maker_signed_phase(None, Some(9), 6), AwaitingLock);

        // Leg B observed from FIRST SIGHTING — the mempool (0 confs) counts:
        // v1's maker flips on the taker's `funded` message at broadcast time, so
        // gating v2 on the first confirmation made it lag v1 by exactly one
        // block ("Their lock confirming · 0/10" was skipped).
        assert_eq!(maker_signed_phase(Some(0), Some(6), 6), TheirLockB);
        assert_eq!(maker_signed_phase(Some(1), Some(6), 6), TheirLockB);
        assert_eq!(maker_signed_phase(Some(3), Some(6), 6), TheirLockB);
        // Even if our leg A were somehow shallow, an observed leg B wins.
        assert_eq!(maker_signed_phase(Some(0), Some(0), 6), TheirLockB);

        // No leg-A funding recorded yet → pre-lock liveness wait (no anchor).
        assert_eq!(maker_signed_phase(None, None, 6), AwaitingLockUnanchored);
    }

    #[test]
    fn mainnet_allowed_after_audit_gate_lifted() {
        // Mainnet was refused pending external review; with the protocol + impl
        // under audit the gate is lifted, so a valid-profile v1 offer now
        // succeeds entirely offline (like testnet/regtest).
        let (engine, dir) = engine_with("mainnet", Some("pw"));
        let now = local_now() as u32;
        let (record, _) = engine
            .offer(
                Network::Mainnet,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                now + 10 * 3600,
                now + 5 * 3600,
                None,
                None,
            )
            .expect("mainnet v1 offer is now permitted (audit gate lifted)");
        assert_eq!(record.chain_a.network, Network::Mainnet);
        assert_eq!((record.n_a, record.n_b), (10, 6));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn action_margins_zero_on_regtest_spec_values_otherwise() {
        // Regtest is exempt (§7.5) so the e2e suite drives swaps in seconds.
        assert_eq!(action_margins(Network::Regtest), (0, 0, 0));
        // Mainnet/testnet carry the normative §7.4 lead-times: 3h fund,
        // 2h reveal, 1h redeem-A.
        assert_eq!(action_margins(Network::Mainnet), (3 * 3600, 2 * 3600, 3600));
        assert_eq!(action_margins(Network::Testnet), (3 * 3600, 2 * 3600, 3600));
    }

    #[test]
    fn deadline_clock_is_conservative_off_regtest() {
        // Regtest keeps the historical pure-MTP behaviour (the chain MTP starts
        // at the 2011 genesis time and the suite relies on that lag).
        assert_eq!(deadline_clock(Network::Regtest, 5_000, 1_000), 1_000);
        // Elsewhere we take the LATER of wall clock and chain MTP, so neither a
        // lagging chain nor a slow local clock pushes us past a deadline.
        assert_eq!(deadline_clock(Network::Mainnet, 5_000, 1_000), 5_000);
        assert_eq!(deadline_clock(Network::Mainnet, 1_000, 5_000), 5_000);
    }

    #[test]
    fn action_safe_enforces_margin_before_deadline() {
        let t2: u32 = 1_780_000_000;
        let reveal = 2 * 3600;
        // Comfortably before T2 − 2h: safe to reveal.
        assert!(action_safe(u64::from(t2) - 3 * 3600, reveal, t2));
        // Exactly at the deadline (now + margin == T2): NOT safe (strict <).
        assert!(!action_safe(u64::from(t2) - reveal, reveal, t2));
        // Inside the 2h window: refused.
        assert!(!action_safe(u64::from(t2) - 3600, reveal, t2));
        // Regtest margin 0 collapses to the old "now < deadline" rule.
        assert!(action_safe(u64::from(t2) - 1, 0, t2));
        assert!(!action_safe(u64::from(t2), 0, t2));
    }

    #[test]
    fn redeem_conf_target_escalates_as_deadline_nears() {
        // #47/#48: plenty of time → today's cheap economical "normal" (6, false).
        assert_eq!(redeem_conf_target(12 * 3600), (6, false));
        assert_eq!(redeem_conf_target(6 * 3600 + 1), (6, false));
        // Under 6h → robust conservative middle bands.
        assert_eq!(redeem_conf_target(6 * 3600), (3, true));
        assert_eq!(redeem_conf_target(2 * 3600 + 1), (3, true));
        assert_eq!(redeem_conf_target(2 * 3600), (2, true));
        assert_eq!(redeem_conf_target(3600 + 1), (2, true));
        // Final stretch → fastest target (worth almost any fee, value-capped).
        assert_eq!(redeem_conf_target(3600), (1, true));
        assert_eq!(redeem_conf_target(0), (1, true));
    }

    #[test]
    fn redeem_a_guard_uses_t1_margin_like_v1() {
        // M3: the v2 participant's leg-A claim is gated on the §7.4 redeem-A
        // margin (the 3rd slot, 1h on mainnet/testnet, 0 on regtest) against T1
        // — the same predicate the v1 participant `redeem` uses, so v1 and v2
        // stop racing Alice's refund at the same point.
        let t1: u32 = 1_780_000_000;
        let (_, _, redeem_a_margin) = action_margins(Network::Mainnet);
        assert_eq!(redeem_a_margin, 3600);
        // More than 1h before T1: safe to claim leg A.
        assert!(action_safe(u64::from(t1) - 2 * 3600, redeem_a_margin, t1));
        // Inside the final hour: refused (would race the T1 refund).
        assert!(!action_safe(u64::from(t1) - 1800, redeem_a_margin, t1));
        // Regtest margin 0: claim allowed up to T1 (e2e completes well before).
        let (_, _, rt_margin) = action_margins(Network::Regtest);
        assert!(action_safe(u64::from(t1) - 1, rt_margin, t1));
        assert!(!action_safe(u64::from(t1), rt_margin, t1));
    }

    #[test]
    fn init_v2_redeem_feerate_is_required_and_roundtrips() {
        use crate::messages::InitV2Body;
        // No backward compat: the negotiated redeem feerates are REQUIRED — an
        // init that omits them is rejected outright (no silent default).
        let without = serde_json::json!({
            "protocol": "pact-htlc-v2",
            "chain_a": { "asset": "btcx", "network": "regtest" },
            "chain_b": { "asset": "btc", "network": "regtest" },
            "amount_a": 1u64, "amount_b": 2u64, "t1": 1_780_050_000u32, "t2": 1_780_020_000u32,
            "alice_swap_a": "x", "alice_swap_b": "y", "alice_refund_a": "z",
            "adaptor_point": "p"
        });
        assert!(
            serde_json::from_value::<InitV2Body>(without).is_err(),
            "an init without redeem feerates must not deserialize"
        );
        // With every required field present, the body round-trips the
        // negotiated rates verbatim.
        let with = serde_json::from_value::<InitV2Body>(serde_json::json!({
            "protocol": "pact-htlc-v2",
            "chain_a": { "asset": "btcx", "network": "regtest" },
            "chain_b": { "asset": "btc", "network": "regtest" },
            "amount_a": 1u64, "amount_b": 2u64, "t1": 1_780_050_000u32, "t2": 1_780_020_000u32,
            "alice_swap_a": "x", "alice_swap_b": "y", "alice_refund_a": "z",
            "adaptor_point": "p", "alice_sweep_b": "",
            "redeem_feerate_a": 30u64, "redeem_feerate_b": 45u64
        }))
        .unwrap();
        assert_eq!(with.redeem_feerate_a, 30);
        let round: InitV2Body =
            serde_json::from_str(&serde_json::to_string(&with).unwrap()).unwrap();
        assert_eq!(round.redeem_feerate_a, 30);
        assert_eq!(round.redeem_feerate_b, 45);
    }

    #[test]
    fn cpfp_child_fee_lifts_package_to_target() {
        // v2+: the cooperative redeem can't be RBF'd, so a child bumps the
        // package. Parent: 111 vB at 10 sat/vB (= 10_000 sat/kvB) committed.
        let parent_vsize = crate::taproot::KEYPATH_REDEEM_VSIZE;
        let parent_fee = 10 * parent_vsize; // committed at 10 sat/vB

        // Target below what the parent already pays: no child needed.
        assert_eq!(cpfp_child_fee_kvb(parent_fee, parent_vsize, 5_000), None);
        assert_eq!(cpfp_child_fee_kvb(parent_fee, parent_vsize, 10_000), None);

        // Target 50 sat/vB (50_000 sat/kvB): the package (parent+child vsizes)
        // must pay 50 * (111 + 150) = 13050 sat; child covers the shortfall.
        let pkg_vsize = parent_vsize + CPFP_CHILD_VSIZE;
        let child = cpfp_child_fee_kvb(parent_fee, parent_vsize, 50_000).unwrap();
        assert_eq!(child, 50 * pkg_vsize - parent_fee);
        // The realised package feerate meets the target.
        assert!((parent_fee + child) / pkg_vsize >= 50);
        // No arbitrary floor: the child pays EXACTLY the top-up to reach market.
        // A zero-fee parent at target 1 sat/vB → the natural package fee
        // (1 × package vsize = 261 sat), not lifted to any minimum.
        assert_eq!(
            cpfp_child_fee_kvb(0, parent_vsize, 1_000).unwrap(),
            pkg_vsize
        );
        // Sub-integer precision now survives: a 1.5 sat/vB (1_500 sat/kvB) target
        // on a zero-fee parent lifts the package to ceil(1.5 × 261) = 392 sat —
        // not rounded down to 1 sat/vB the way the old integer path would have.
        assert_eq!(
            cpfp_child_fee_kvb(0, parent_vsize, 1_500).unwrap(),
            (1_500 * pkg_vsize).div_ceil(1000)
        );
    }

    #[test]
    fn funding_bump_clears_bip125_rule4_at_sub_sat_vb() {
        // Regression for the field deadlock (swap 084f20d3): a funding broadcast
        // at ~1.004 sat/vB (fee 1004 sat over ~1000 vB) stranded because the old
        // integer nurse truncated old→"1", offered "2", and the node required
        // 2.004 → rejected every tick. In sat/kvB the offer must clear old+incr.
        let old_kvb = 1_004; // 1.004 sat/vB, the true broadcast feerate
        let incr_kvb = 1_000; // 1 sat/vB incremental relay fee (mainnet default)
        let ceiling_kvb = 500_000; // default 500 sat/vB policy ceiling
        let mult = 3; // default funding.reservation_mult

        // Market risen to ~2.0 sat/vB: the offered rate MUST reach at least the
        // node's Rule-4 minimum (old + incr = 2.004 sat/vB), not the old "2".
        let rate = funding_bump_rate_kvb(old_kvb, 2_000, incr_kvb, ceiling_kvb, mult).unwrap();
        assert!(
            rate >= old_kvb + incr_kvb,
            "offer {rate} sat/kvB must clear the {}=old+incr Rule-4 floor",
            old_kvb + incr_kvb
        );
        assert_eq!(rate, 2_004); // exactly the required minimum, in kvB

        // Market well above the floor: chase it precisely (no rounding to a whole
        // sat/vB), still bounded by the reservation (3 × old = 3.012 sat/vB).
        assert_eq!(
            funding_bump_rate_kvb(old_kvb, 2_800, incr_kvb, ceiling_kvb, mult),
            Some(2_800)
        );
        assert_eq!(
            funding_bump_rate_kvb(old_kvb, 9_000, incr_kvb, ceiling_kvb, mult),
            Some(mult * old_kvb) // reservation cap: 3 × 1004 = 3012
        );

        // Market NOT risen above what we already pay → no bump (no churn).
        assert_eq!(
            funding_bump_rate_kvb(old_kvb, old_kvb, incr_kvb, ceiling_kvb, mult),
            None
        );
        assert_eq!(
            funding_bump_rate_kvb(old_kvb, 900, incr_kvb, ceiling_kvb, mult),
            None
        );

        // The offer never exceeds the reservation for any funding ≥ 1 sat/vB, so
        // the funds-gate headroom is respected even when the incr floor kicks in.
        let just_above = funding_bump_rate_kvb(old_kvb, old_kvb + 1, incr_kvb, ceiling_kvb, mult);
        assert!(just_above.unwrap() <= mult * old_kvb);
    }

    #[test]
    fn revoke_offers_for_coin_hits_only_that_coins_pairs() {
        // #97: removing a coin must terminally revoke every live offer whose pair
        // involves it (either leg) — and nothing else.
        let (engine, dir) = engine_with("revoke-coin", None);
        let put = |id: &str, give: &str, get: &str| {
            let body = serde_json::to_value(crate::board::OfferBody {
                protocol: "pact-htlc-v1".into(),
                wire: 1,
                network: "regtest".into(),
                give_asset: give.into(),
                give_amount: 1,
                get_asset: get.into(),
                get_amount: 1,
                t1_secs: 3600,
                t2_secs: 1800,
                ttl_secs: Some(3600),
                created: 1_700_000_000,
            })
            .unwrap();
            let env = engine.signed_envelope("offer", id, body).unwrap();
            engine
                .store
                .my_offer_put(
                    id,
                    &serde_json::to_string(&env).unwrap(),
                    1_700_000_000,
                    0,
                    1_700_000_000,
                )
                .unwrap();
        };
        put("o_btcx_btc", "btcx", "btc"); // btc on the GET leg
        put("o_btc_ltc", "btc", "ltc"); // btc on the GIVE leg
        put("o_pocx_ltc", "pocx", "ltc"); // no btc — must survive
        assert_eq!(engine.store.my_offers_live().unwrap().len(), 3);

        let revoked = engine.revoke_offers_for_coin("btc").unwrap();
        assert_eq!(revoked.len(), 2, "both btc-pair offers revoked");
        let live: Vec<String> = engine
            .store
            .my_offers_live()
            .unwrap()
            .into_iter()
            .map(|o| o.offer_id)
            .collect();
        assert_eq!(
            live,
            vec!["o_pocx_ltc".to_string()],
            "only the non-btc offer survives"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fee_policy_defaults_then_persists_across_reopen() {
        let (mut engine, dir) = engine_with("feepolicy", None);
        // A fresh merchant starts on the defaults.
        assert_eq!(engine.fee_bump, crate::fee_policy::FeeBumpPolicy::default());

        // Change a couple of fields and persist.
        let mut pol = engine.fee_bump;
        pol.max_feerate_sat_vb = 250;
        pol.redeem.committed_mult = 4;
        engine.set_fee_bump(pol).unwrap();
        assert_eq!(engine.fee_bump.max_feerate_sat_vb, 250);
        drop(engine);

        // Reopening the same store reloads the persisted policy (survives restart
        // with no Satchel involved).
        let engine2 = Engine::open(&dir, None, BTreeMap::new()).unwrap();
        assert_eq!(engine2.fee_bump.max_feerate_sat_vb, 250);
        assert_eq!(engine2.fee_bump.redeem.committed_mult, 4);
        // Untouched fields keep their defaults.
        assert_eq!(engine2.fee_bump.funding.reservation_mult, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn coin_wallet_parses_the_scoped_wallet_from_the_url() {
        let (mut e, dir) = engine_with("walletparse", None);
        // Wallet-qualified URL → the name; works with userpass or cookie auth and
        // with extra (comma-separated) backends after the primary.
        e.coins.insert(
            "btc".into(),
            "http://u:p@127.0.0.1:8332/wallet/alice,tcp://127.0.0.1:50001".into(),
        );
        assert_eq!(e.coin_wallet("btc"), Some("alice".into()));
        e.coins.insert(
            "btcx".into(),
            "http://__cookie__:deadbeef@127.0.0.1:19443/wallet/pocx".into(),
        );
        assert_eq!(e.coin_wallet("btcx"), Some("pocx".into()));
        // No wallet path → None (node default, not explicitly scoped).
        e.coins
            .insert("ltc".into(), "http://u:p@127.0.0.1:9332".into());
        assert_eq!(e.coin_wallet("ltc"), None);
        // Unconfigured coin → None.
        assert_eq!(e.coin_wallet("nope"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_fee_bump_rejects_invalid_and_keeps_old() {
        let (mut engine, dir) = engine_with("feepolicy-bad", None);
        let mut pol = engine.fee_bump;
        pol.max_feerate_sat_vb = crate::fee_policy::MAX_FEERATE_CEILING + 1; // over the ceiling
        assert!(engine.set_fee_bump(pol).is_err());
        // The live policy is unchanged after a rejected update.
        assert_eq!(engine.fee_bump, crate::fee_policy::FeeBumpPolicy::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offer_offsets_reject_tight_lift_clears_presets() {
        // The old 3h/6h "short": gap 3h < 4h AND T2 == fund margin — rejected.
        assert!(validate_offer_offsets(Network::Mainnet, 6 * 3600, 3 * 3600).is_err());
        // A 4h-gap but too-near T2 (2h) is refused on the fund margin.
        assert!(validate_offer_offsets(Network::Mainnet, 6 * 3600, 2 * 3600).is_err());
        // Regtest is exempt — the e2e suite uses tiny offsets.
        assert!(validate_offer_offsets(Network::Regtest, 6 * 3600, 3 * 3600).is_ok());
        // Every shipped UI preset (post-lift) clears the gate on mainnet — keep
        // in sync with satchel/ui/src/components/OfferForm.tsx `TERMS`
        // (short 12/6, medium 24/12, long 36/18 hours).
        for (t1, t2) in [
            (12 * 3600, 6 * 3600),
            (24 * 3600, 12 * 3600),
            (36 * 3600, 18 * 3600),
        ] {
            validate_offer_offsets(Network::Mainnet, t1, t2)
                .unwrap_or_else(|e| panic!("preset {t1}/{t2} must be valid: {e}"));
        }
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
        assert_eq!((record.n_a, record.n_b), (10, 6));
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
    fn post_and_take_blocked_when_chain_down() {
        // The chain-up gate: advertising or taking a swap is refused up front
        // when a leg's node is unreachable (here, a dead loopback port). Pure
        // envelope builders (offer/accept/make_private_offer) are NOT gated, so
        // they still succeed with no live node — that's the altitude split.
        let dir = std::env::temp_dir().join(format!("libswap-chaindown-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::init(&dir, None).unwrap();
        let mut coins = BTreeMap::new();
        // Port 1 refuses immediately — a stand-in for "node is down".
        coins.insert("btcx".to_string(), "http://127.0.0.1:1".to_string());
        coins.insert("btc".to_string(), "http://127.0.0.1:1".to_string());
        let engine = Engine::open(&dir, None, coins).unwrap();

        // Building an offer envelope needs no node — still works.
        engine
            .offer(
                Network::Regtest,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                1_700_000_002,
                1_700_000_001,
                None,
                None,
            )
            .expect("offer envelope build needs no live node");

        // Posting to the board hits the gate first.
        let err = engine
            .post_board_offer(
                Network::Regtest,
                ("btcx".into(), 100),
                ("btc".into(), 100),
                10 * 3600,
                5 * 3600,
                None,
                None,
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("unreachable"), "{err}");
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
        assert_eq!(give["fee_rate_sat_per_vb"], 1);
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
            let vbytes = leg["vbytes"].as_u64().unwrap();
            assert!(vbytes > 0);
            // No arbitrary floor: fee = market rate × vsize with only a 1 sat/vB
            // (min-relay) guard, so each leg pays at least its vsize in sats.
            assert!(leg["fee_sat"].as_u64().unwrap() >= vbytes);
        }
        // Fallback market is 1 sat/vB → each leg pays exactly its vsize (fund 160 vB
        // = 160 sat), no longer lifted to the old flat 1000-sat floor.
        assert_eq!(give_legs[0]["fee_sat"], give_legs[0]["vbytes"]);

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

    /// Insert a live offer straight into the store, mirroring what `post_board_offer`
    /// registers, so `committed_give_for_coin` has rows to sum without a live node.
    fn put_live_offer(
        engine: &Engine,
        id: &str,
        network: Network,
        give_asset: &str,
        give_amount: u64,
    ) {
        let body = crate::board::OfferBody {
            protocol: "pact-htlc-v1".into(),
            wire: crate::WIRE_V1,
            network: format!("{network:?}").to_lowercase(),
            give_asset: give_asset.into(),
            give_amount,
            get_asset: "btc".into(),
            get_amount: 100,
            t1_secs: 1,
            t2_secs: 1,
            ttl_secs: None,
            created: 0,
        };
        let env = Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: id.into(),
            from: "maker".into(),
            body: serde_json::to_value(&body).unwrap(),
            sig: String::new(),
        };
        engine
            .store
            .my_offer_put(id, &serde_json::to_string(&env).unwrap(), 0, 0, 0)
            .unwrap();
    }

    #[test]
    fn committed_give_sums_only_matching_coin_and_network() {
        let (engine, dir) = engine_with("committed-give", None);
        // Two live btcx offers on regtest, one btc offer, one foreign-network
        // btcx offer, and one malformed row — only the two regtest btcx offers
        // should be charged against a new regtest btcx offer.
        put_live_offer(&engine, "a", Network::Regtest, "btcx", 100);
        put_live_offer(&engine, "b", Network::Regtest, "btcx", 250);
        put_live_offer(&engine, "c", Network::Regtest, "btc", 999);
        put_live_offer(&engine, "d", Network::Mainnet, "btcx", 500);
        engine.store.my_offer_put("e", "not json", 0, 0, 0).unwrap();

        let (sum, count) = engine
            .committed_give_for_coin(Network::Regtest, "btcx")
            .unwrap();
        assert_eq!((sum, count), (350, 2));

        // The btc leg is summed independently.
        let (btc_sum, btc_count) = engine
            .committed_give_for_coin(Network::Regtest, "btc")
            .unwrap();
        assert_eq!((btc_sum, btc_count), (999, 1));

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
    fn stale_pending_take_is_pruned() {
        // A take whose maker never followed through within the pre-funding
        // window is cleaned up on the next tick.
        let (engine, dir) = engine_with("stale-take", None);
        let stale = local_now() - PRE_FUNDING_TIMEOUT_SECS - 60;
        engine
            .store
            .put_pending_take("old", &pending_offer_from("m"), stale)
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

    // ---- handshake cancel + staleness (the 2026-07-03 stuck-`created`
    // incident: a stale take burned the offer and stranded an uncancelable
    // v2 record; see HANDSHAKE_CANCEL_FIX_PLAN.md) ----

    /// A fresh v2 record on `engine` (initiator side), pre-funding.
    fn v2_record(engine: &Engine) -> crate::store::AdaptorSwapRecord {
        let now = local_now() as u32;
        let (rec, _init) = engine
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 50_000_000),
                ("btc".into(), 100_000),
                now + 40_000,
                now + 20_000,
            )
            .unwrap();
        rec
    }

    #[test]
    fn adaptor_abort_flips_prefunding_and_refuses_after_our_funding() {
        let (alice, ad) = engine_with("v2-abort-alice", None);
        let (bob, bd) = engine_with("v2-abort-bob", None);

        // Initiator, nothing funded → abort succeeds and persists.
        let rec = v2_record(&alice);
        let aborted = alice.adaptor_abort(&rec.swap_id, "test").unwrap();
        assert_eq!(aborted.state, AdaptorState::Aborted);
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        // Initiator with leg A funded → refused (refund is the only exit).
        let mut rec = v2_record(&alice);
        rec.funding_a_txid = Some("aa".repeat(32));
        alice.store.put_adaptor(&rec).unwrap();
        let err = alice
            .adaptor_abort(&rec.swap_id, "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("funded"), "{err}");

        // Participant: gate is OUR leg (B) — the counterparty's leg-A funding
        // does not block our abort (we lose nothing by walking away).
        let (_arec, init) = alice
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 1_000),
                ("btc".into(), 500),
                local_now() as u32 + 40_000,
                local_now() as u32 + 20_000,
            )
            .unwrap();
        let (mut brec, _accept) = bob.adaptor_accept(&init).unwrap();
        brec.funding_a_txid = Some("bb".repeat(32));
        bob.store.put_adaptor(&brec).unwrap();
        assert_eq!(
            bob.adaptor_abort(&brec.swap_id, "x").unwrap().state,
            AdaptorState::Aborted
        );
        // …but not once WE funded leg B.
        let (_arec2, init2) = alice
            .adaptor_init(
                Network::Regtest,
                ("btcx".into(), 2_000),
                ("btc".into(), 900),
                local_now() as u32 + 40_000,
                local_now() as u32 + 20_000,
            )
            .unwrap();
        let (mut brec2, _accept2) = bob.adaptor_accept(&init2).unwrap();
        brec2.funding_b_txid = Some("cc".repeat(32));
        bob.store.put_adaptor(&brec2).unwrap();
        assert!(bob.adaptor_abort(&brec2.swap_id, "x").is_err());

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
    }

    #[test]
    fn recv_abort_flips_v2_only_from_pinned_counterparty_and_only_prefunding() {
        let (alice, ad) = engine_with("v2-recv-abort-alice", None);
        let (bob, bd) = engine_with("v2-recv-abort-bob", None);
        let (carol, cd) = engine_with("v2-recv-abort-carol", None);
        let bob_id = bob.identity().unwrap();

        // Pinned counterparty aborts a pre-funding record → flips.
        let mut rec = v2_record(&alice);
        rec.counterparty_identity = Some(bob_id.clone());
        alice.store.put_adaptor(&rec).unwrap();
        let abort = bob
            .signed_envelope(
                "abort",
                &rec.swap_id,
                serde_json::json!({ "reason": "bye" }),
            )
            .unwrap();
        let ev = alice.recv_abort(&abort).unwrap().unwrap();
        assert_eq!(ev.action, "counterparty-abort");
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        // A third party's signed abort is refused and flips nothing.
        let mut rec = v2_record(&alice);
        rec.counterparty_identity = Some(bob_id.clone());
        alice.store.put_adaptor(&rec).unwrap();
        let forged = carol
            .signed_envelope(
                "abort",
                &rec.swap_id,
                serde_json::json!({ "reason": "hah" }),
            )
            .unwrap();
        assert!(alice.recv_abort(&forged).is_err());
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Created
        );

        // Once our leg is funded the abort is advisory only.
        let mut rec = v2_record(&alice);
        rec.counterparty_identity = Some(bob_id);
        rec.funding_a_txid = Some("dd".repeat(32));
        alice.store.put_adaptor(&rec).unwrap();
        let abort = bob
            .signed_envelope(
                "abort",
                &rec.swap_id,
                serde_json::json!({ "reason": "bye" }),
            )
            .unwrap();
        let ev = alice.recv_abort(&abort).unwrap().unwrap();
        assert!(ev.detail.contains("ignored"), "{}", ev.detail);
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Created
        );

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
        std::fs::remove_dir_all(&cd).ok();
    }

    #[test]
    fn recv_abort_resolves_offer_id_via_served_marker() {
        // The taker may never have learned the swap id (its pending take
        // pruned before our init arrived) — it cancels by OFFER id and the
        // served marker maps that to the record the take created.
        let (alice, ad) = engine_with("v2-offer-abort-alice", None);
        let (bob, bd) = engine_with("v2-offer-abort-bob", None);
        let (carol, cd) = engine_with("v2-offer-abort-carol", None);
        let bob_id = bob.identity().unwrap();

        let mut rec = v2_record(&alice);
        rec.counterparty_identity = Some(bob_id);
        alice.store.put_adaptor(&rec).unwrap();
        alice
            .store
            .meta_set("offer_served:offer-Z", &rec.swap_id)
            .unwrap();

        // Wrong sender first: refused, nothing flips.
        let forged = carol
            .signed_envelope("abort", "offer-Z", serde_json::json!({ "reason": "hah" }))
            .unwrap();
        assert!(alice.recv_abort(&forged).is_err());
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Created
        );

        // The pinned taker cancelling by offer id aborts the record.
        let abort = bob
            .signed_envelope(
                "abort",
                "offer-Z",
                serde_json::json!({ "reason": "gave up" }),
            )
            .unwrap();
        let ev = alice.recv_abort(&abort).unwrap().unwrap();
        assert_eq!(ev.action, "counterparty-abort");
        assert_eq!(ev.swap_id, rec.swap_id);
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        std::fs::remove_dir_all(&ad).ok();
        std::fs::remove_dir_all(&bd).ok();
        std::fs::remove_dir_all(&cd).ok();
    }

    #[test]
    fn cancel_pending_take_removes_the_take() {
        let (engine, dir) = engine_with("cancel-take", None);
        engine
            .store
            .put_pending_take("offer-A", &pending_offer_from("some-maker"), 1)
            .unwrap();
        // The relay notify is best-effort (no boards here) — the local
        // removal must succeed regardless.
        engine.cancel_pending_take("offer-A").unwrap();
        assert!(engine.store.pending_takes().unwrap().is_empty());
        // Cancelling a take we don't hold errors.
        assert!(engine.cancel_pending_take("offer-A").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn v2_prefunding_timeout_aborts_silently() {
        let (alice, dir) = engine_with("v2-timeout", None);
        let stale = local_now() - PRE_FUNDING_TIMEOUT_SECS - 5;

        // Created + past the window + nothing funded → aborted by the tick.
        let mut rec = v2_record(&alice);
        rec.created_at = stale;
        alice.store.put_adaptor(&rec).unwrap();
        let ev = alice.adaptor_tick_one(&rec).unwrap().unwrap();
        assert_eq!(ev.action, "abort-timeout");
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        // NoncesExchanged is still strictly pre-funding → also covered.
        let mut rec = v2_record(&alice);
        rec.state = AdaptorState::NoncesExchanged;
        rec.created_at = stale;
        alice.store.put_adaptor(&rec).unwrap();
        let ev = alice.adaptor_tick_one(&rec).unwrap().unwrap();
        assert_eq!(ev.action, "abort-timeout");

        // created_at == 0 (pre-timestamp record) must NOT read as infinitely
        // old: the tick leaves it alone.
        let mut rec = v2_record(&alice);
        rec.created_at = 0;
        alice.store.put_adaptor(&rec).unwrap();
        assert!(alice.adaptor_tick_one(&rec).unwrap().is_none());
        assert_eq!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Created
        );

        // A funding pointer disarms the timeout (belt-and-braces guard).
        let mut rec = v2_record(&alice);
        rec.created_at = stale;
        rec.funding_a_txid = Some("ee".repeat(32));
        alice.store.put_adaptor(&rec).unwrap();
        let _ = alice.adaptor_tick_one(&rec); // may err (no backends here)
        assert_ne!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        // Signed is excluded — funding may already be in flight there.
        let mut rec = v2_record(&alice);
        rec.state = AdaptorState::Signed;
        rec.created_at = stale;
        alice.store.put_adaptor(&rec).unwrap();
        let _ = alice.adaptor_tick_one(&rec); // may err (no backends here)
        assert_ne!(
            alice.store.get_adaptor(&rec.swap_id).unwrap().state,
            AdaptorState::Aborted
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_take_is_dropped_before_any_side_effect() {
        let (maker, md) = engine_with("stale-take-maker", None);
        let (taker, td) = engine_with("stale-take-taker", None);

        // A real signed offer of ours (the slip codec hands back the envelope).
        let slip = maker
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
        let offer = pact_proto::slip::decode_slip(&slip).unwrap();
        let offer_id = offer.swap_id.clone();
        let take_with = |taken_at: serde_json::Value| {
            taker
                .signed_envelope(
                    "take",
                    &offer_id,
                    serde_json::json!({
                        "offer": serde_json::to_value(&offer).unwrap(),
                        "taken_at": taken_at,
                    }),
                )
                .unwrap()
        };
        let no_side_effects = |maker: &Engine| {
            assert!(maker
                .store
                .meta_get(&format!("offer_served:{offer_id}"))
                .unwrap()
                .is_none());
            assert!(maker
                .store
                .meta_get(&format!("offer_revoked:{offer_id}"))
                .unwrap()
                .is_none());
            assert!(maker.store.list().unwrap().is_empty());
            assert!(maker.store.list_adaptor().unwrap().is_empty());
        };

        // Older than the taker's own prune window → dropped silently, the
        // offer is NOT burned, no record is created.
        let stale = take_with(serde_json::json!(
            local_now() - PRE_FUNDING_TIMEOUT_SECS - 5
        ));
        let ev = maker.handle_relay_envelope(&stale).unwrap().unwrap();
        assert_eq!(ev.action, "take-stale");
        no_side_effects(&maker);

        // A take without the (required) stamp is treated as stale.
        let unstamped = take_with(serde_json::Value::Null);
        let ev = maker.handle_relay_envelope(&unstamped).unwrap().unwrap();
        assert_eq!(ev.action, "take-stale");
        no_side_effects(&maker);

        // A fresh take gets PAST the gate: with no chain backends configured
        // here it then fails on chain access — but is NOT "take-stale", and
        // still nothing was burned before that failure.
        let fresh = take_with(serde_json::json!(local_now()));
        assert!(maker.handle_relay_envelope(&fresh).is_err());
        no_side_effects(&maker);

        std::fs::remove_dir_all(&md).ok();
        std::fs::remove_dir_all(&td).ok();
    }
}
