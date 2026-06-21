# Fee-bump unification + funding nurse — implementation plan

**Companion to** [`FUNDING_FEE_BUMP.md`](FUNDING_FEE_BUMP.md) (the design). That doc
says *what* and *why*; this one says *exactly how* — every file, function, const,
parameter, and ordering decision needed to land it.

**Status:** PENDING. Liveness, not safety. **Not** release-blocking.

**Two deliverables, sequenced:**

1. **Refactor** — collapse the scattered fee-bump consts into one
   `FeeBumpPolicy`, threaded engine → pactd → Satchel. *Pure behaviour-preserving*
   (defaults == today's consts). Covered by the existing test suite.
2. **Funding nurse** — the one tx with no bump today (the funding/lock) gains a
   market-chasing bump: **RBF** on v1, **CPFP-via-change** on v2. Needs a regtest
   fee-spike harness to verify.

Land (1) first and merge it green; (2) builds on the policy struct from (1).

---

## 0. Ground truth — where everything lives today

All paths are at the line numbers seen when this plan was written; treat them as
anchors, re-confirm before editing.

### Existing consts (the sprawl)

| Const | File:line | Value | Fate under this plan |
| --- | --- | --- | --- |
| `MIN_SPEND_FEE_SAT` | `swap.rs:38` | 1000 | → `policy.min_fee_sat` (keep const as the *default*) |
| `REGTEST_REDEEM_FEERATE` | `engine.rs:335` | 2 | **stays a const** — network special case, not a knob |
| `ADAPTOR_REDEEM_FEERATE_MULT` | `engine.rs:351` | 2 | → `policy.redeem.committed_mult` |
| `MAX_REDEEM_FEERATE` | `engine.rs:357` | 500 | **const STAYS** — governs the protocol-level init bound (§2.1, §2.6) only; the policy ceiling is a *separate* `fee_policy::MAX_FEERATE_CEILING` (= 500), decoupled even though equal |
| `ADAPTOR_REDEEM_FEERATE_FALLBACK` | `engine.rs:361` | 20 | **stays a const** — no-estimator fallback |
| `FUNDING_BUMP_FEERATE_MULT` | `engine.rs:373` | 3 | → `policy.funding.reservation_mult` |
| `FUNDING_VSIZE_EST` | `engine.rs:377` | 250 | **stays a const** — sizing constant |
| `FUNDING_FEERATE_FALLBACK` | `engine.rs:380` | 20 | **stays a const** — no-estimator fallback |
| `CPFP_CHILD_VSIZE` | `engine.rs:385` | 150 | **stays a const** — sizing constant |
| inline `+50%` (`old_fee/2`) | `engine.rs:2061`, `engine.rs:2993` | 50% | → `policy.{refund,redeem}.step_pct` |

**Rule of thumb:** *multipliers and steps that a user might reasonably trade off
(cost vs. safety) become policy fields; physical sizing constants and
network/fallback special-cases stay consts.*

### Existing bump / fee sites

| Site | File:line | What it does today |
| --- | --- | --- |
| `Engine::adaptor_redeem_feerate` | `engine.rs:522` | v2 committed redeem feerate = `live × MULT(2)` clamp `[2,500]` |
| `Engine::cpfp_child_fee` (pure) | `engine.rs:393` | child fee to lift v2 redeem package to `target` |
| `Engine::adaptor_cpfp_bump` | `engine.rs:1992` | builds + `wallet_sign_send`s the v2 redeem CPFP child |
| `Engine::adaptor_bump_refund` | `engine.rs:2042` | v2 refund RBF, `+50%/tick` (line 2061) |
| `Engine::maybe_bump` | `engine.rs:2965` | v1 redeem **and** refund RBF, `+50%/tick` (line 2993) |
| `Engine::ensure_can_fund` | `engine.rs:673` | funds gate; reserves `min(live×3, 500) × 250` headroom |
| `Engine::fund` (v1 broadcast) | `engine.rs:2219`, send at 2277 | `wallet_send` → `sendtoaddress` (no `replaceable` arg) |
| `Engine::adaptor_fund` (v2 broadcast) | `engine.rs:1521`, send at 1531 | same |
| `tick_one` (v1 scheduler) | `engine.rs:2704` | per-state driver; bump sites hang off it |
| `adaptor_tick_one` (v2 scheduler) | `engine.rs:1727` | v2 per-state driver |

### Backend trait (`chain.rs:43`)

Implemented by `CoreRpcBackend` (148), Electrum (565-ish), `MultiBackend` (834-ish).
Relevant methods already present: `broadcast`, `get_txout`, `find_funding`
(spk-based, 208), `find_vout` (245), `fee_rate_sat_per_vb` (clamps `[1,500]`),
`wallet_new_address`, `wallet_balance`, `wallet_send` (→ `sendtoaddress`, 345),
`wallet_sign_send` (CPFP signer, 367). **Missing** (added in §3): a fee-bump RPC
(`bumpfee`) and a wallet-tx introspection call (`gettransaction` decode for
fee / vsize / change output).

### Plumbing chain (how a config field flows out to the user)

> ⚠️ **The fee policy deliberately does NOT follow this chain — see §7.** Decision
> #4 makes it store-owned and RPC-driven (no `satchel.json` field, no launch flag).
> The `auto_fund` path below is shown only as the *contrast*; do not use it as the
> template for the fee policy.

`auto_fund` is the reference field for the *old* launch-arg pattern:

1. `Engine.auto_fund` — `engine.rs:63` (struct field), default at `engine.rs:446`.
2. `EngineConfig.auto_fund` — `pactd/merchants.rs:92`; copied onto the engine in
   `build_engine` at `merchants.rs:105`.
3. `Args.auto_fund` — `pactd/main.rs:124` (clap flag); fed into `EngineConfig` at
   `main.rs:1068`.
4. Satchel `Config` — `satchel/src/main.rs:185`; serialized to `satchel.json`
   (`main.rs:580`), passed to pactd as a launch arg (`main.rs:438`-ish).
5. Satchel UI — a Settings control + a `#[tauri::command]` saver
   (`save_nostr_relays` at `main.rs:604` is the template).

---

## 1. The policy struct

New module `pact/libswap/src/fee_policy.rs` (keep it out of the already-large
`engine.rs`). `serde` so pactd/Satchel can pass it as JSON; `Copy` (it is small
and all-scalar) so bump sites take it by value freely.

```rust
//! One parameterized fee-bump policy, shared by every bump site. Distinct
//! *strategy* per tx type is intrinsic (you cannot RBF a v2 redeem) and not a
//! user choice; the numeric *parameters* are. Defaults reproduce the historical
//! hardcoded consts exactly, so behaviour is unchanged until overridden.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FeeBumpPolicy {
    /// Absolute ceiling (sat/vB) applied to EVERY path, including the RBF
    /// escalators that are uncapped today. Belt-and-suspenders vs a runaway
    /// estimate. Default 500 (was `MAX_REDEEM_FEERATE`).
    pub max_feerate_sat_vb: u64,
    /// Floor for any single-tx bump (sat). Default 1000 (was `MIN_SPEND_FEE_SAT`).
    pub min_fee_sat: u64,
    pub funding: FundingPolicy,
    pub redeem: RedeemPolicy,
    pub refund: RefundPolicy,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FundingPolicy {
    /// Funds gate (`ensure_can_fund`) reserves `reservation_mult × live estimate`
    /// of balance as bump headroom. The nurse then CHASES CURRENT MARKET, bounded
    /// by this reservation AND `max_feerate_sat_vb` — it does NOT chase a multiple
    /// of the initial rate. Default 3 (was `FUNDING_BUMP_FEERATE_MULT`).
    pub reservation_mult: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RedeemPolicy {
    /// v2: over-provision multiplier baked into the adaptor sig at funding time
    /// (the unattended floor; CPFP chases market beyond it). Per-swap → new swaps
    /// only. Default 2 (was `ADAPTOR_REDEEM_FEERATE_MULT`).
    pub committed_mult: u64,
    /// v1: percent the fee escalates per scheduler tick. Default 50.
    pub step_pct: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RefundPolicy {
    /// Percent the fee escalates per scheduler tick, both protocols. Default 50.
    pub step_pct: u64,
}

impl Default for FeeBumpPolicy { /* 500 / 1000 / {3} / {2,50} / {50} */ }
// (impl Default for each sub-struct likewise — values from the table in §0)
```

**Decisions baked in (from the design doc):**

- `serde(default)` on every struct → an old/partial `satchel.json` or a flag set
  that omits a field gets the historical default. This is the *one* legacy-tolerance
  we allow and it is forward-only (new fields, never removed) — consistent with the
  no-backward-compat principle (new product, but config files are user data).
- `deny_unknown_fields` so a typo'd knob is a loud error, not a silent default.
- Define the ceiling const **in `fee_policy.rs`** so the module stays
  self-contained: `pub const MAX_FEERATE_CEILING: u64 = 500;`. The `Default` impl
  uses it (`max_feerate_sat_vb: MAX_FEERATE_CEILING`), and `validated()` bounds
  against it. Do **not** reference `engine::MAX_REDEEM_FEERATE` from here — that's a
  separate *protocol* const (spec §5) that happens to equal 500; keep the two
  concepts decoupled even though the numbers match.
- Validation method `fn validated(self) -> Result<Self>`: `min_fee_sat >= 1`,
  `max_feerate_sat_vb` in `1..=MAX_FEERATE_CEILING` (= 500), `reservation_mult >= 1`,
  `committed_mult >= 1`, `step_pct` in `1..=1000`. Called once where the policy
  enters the engine (pactd `setfeepolicy` / `build_engine`), not on every tick.
  - **`max_feerate_sat_vb` is a configurable knob bounded by a hard 500 ceiling
    (decision: revert of #6).** `fee_rate_sat_per_vb` hard-clamps the estimate to
    `[1, 500]` (`MAX_SAT_PER_VB`, `chain.rs:325/333` and `:677/688`) — a sanity guard
    against estimator glitches that bounds **every** fee computation in the system
    (~8 sites). 500 is therefore an effective hard ceiling everywhere, for free.
    `max_feerate_sat_vb` is a real, Satchel-exposed knob settable anywhere in
    `[1, 500]`; it only *tightens* the bump ceilings below 500. We deliberately do
    **not** raise the estimator clamp to let it exceed 500: that would strip glitch
    protection from all ~8 consumers and force a per-site re-cap (more surface, more
    tests, weaker safety) to buy only an extreme >500 sat/vB regime these chains
    don't need. **Default stays 500** (unchanged behaviour). No `ESTIMATOR_SANITY_MAX`
    change — the existing 500 clamp stays.

Add to `engine.rs`:

```rust
pub struct Engine {
    // … existing fields …
    pub auto_fund: bool,
    /// Unified fee-bump policy (see crate::fee_policy). Default = historical
    /// consts, so behaviour is unchanged until Satchel/pactd overrides it.
    pub fee_bump: crate::fee_policy::FeeBumpPolicy,
}
```

Default it in `Engine::open` (`engine.rs:440`) with
`fee_bump: FeeBumpPolicy::default()`.

### Shared helper (one place computes the bumped fee, capped)

Put the escalation math in `fee_policy.rs` so v1 redeem, v1 refund, and v2 refund
all call one function — replacing the two copies of `old_fee + (old_fee/2).max(..)`
at `engine.rs:2061` and `engine.rs:2993`:

```rust
impl FeeBumpPolicy {
    /// RBF escalation step, capped at the absolute ceiling for this tx vsize.
    /// `vsize` is the spend's known vsize (REDEEM_TX_VSIZE = 155 / REFUND_TX_VSIZE
    /// = 146). Returns the new absolute fee (sat), or `None` when even the capped
    /// step can't clear the BIP125 Rule-4 floor (old_fee + incremental relay fee)
    /// — i.e. we're already at/above the ceiling and any "bump" would be
    /// unrelayable. Caller treats `None` as a no-op (skip this tick). The cap is
    /// the new behaviour: today's escalators have no feerate ceiling.
    pub fn escalate(&self, old_fee: u64, step_pct: u64, vsize: u64) -> Option<u64> {
        let stepped = old_fee + (old_fee * step_pct / 100).max(self.min_fee_sat);
        let ceiling_fee = self.max_feerate_sat_vb.saturating_mul(vsize);
        let new_fee = stepped.min(ceiling_fee);
        // BIP125 Rule 4: a replacement must beat old_fee by at least the
        // incremental relay fee (= 1 sat/vB × vsize ≈ 146-155 sat here). +1 is NOT
        // enough — it produces an unrelayable replacement. If the cap leaves no
        // room above that floor, there is nothing to do.
        let incremental_relay = vsize; // 1 sat/vB default min-relay increment
        (new_fee >= old_fee + incremental_relay).then_some(new_fee)
    }
}
```

> When `escalate` returns `None` (capped step can't clear the BIP125 floor), the
> caller skips the bump entirely — it does **not** broadcast a +1 nudge (that was
> the bug). The existing dust fallback (`amount <= new_fee + DUST_LIMIT_SAT →
> rebroadcast`) stays in front of this at each call site, and a normal step
> (`min_fee_sat = 1000`) always clears the floor — so `None` only ever bites once
> the fee has already climbed to `max_feerate_sat_vb`.

---

## 2. Refactor — wire the policy into existing sites (behaviour-preserving)

No new behaviour here except the **new feerate cap on the RBF escalators**. Each
edit reads `self.fee_bump.…` instead of a const.

1. **`adaptor_redeem_feerate`** (`engine.rs:522`):
   `rate * ADAPTOR_REDEEM_FEERATE_MULT` → `rate * self.fee_bump.redeem.committed_mult`.
   **Keep the clamp on the protocol const: `.clamp(REGTEST_REDEEM_FEERATE,
   MAX_REDEEM_FEERATE)` — do NOT switch it to `max_feerate_sat_vb`.** The committed
   redeem feerate is a **protocol-negotiated** value: it is baked into the init
   message and validated by the counterparty against their `MAX_REDEEM_FEERATE`
   (§2.6). It is conceptually distinct from the operator's local *bump* ceiling —
   coupling them would let a low cost-capping `max_feerate_sat_vb` silently cripple
   the unattended redeem's confirmation safety. `committed_mult` is the knob that
   tunes this path; the ceiling on the negotiated rate stays the protocol's.
   (`MAX_REDEEM_FEERATE` is thus used in two places — here and the §2.6 init bound —
   and stays a const; §0.)
2. **`adaptor_bump_refund`** (`engine.rs:2061`): replace the inline `new_fee` with
   `self.fee_bump.escalate(old_fee, self.fee_bump.refund.step_pct, REFUND_TX_VSIZE)`.
3. **`maybe_bump`** (`engine.rs:2993`): same, choosing `redeem.step_pct` /
   `REDEEM_TX_VSIZE` when `is_redeem`, else `refund.step_pct` / `REFUND_TX_VSIZE`.

   > **Call-site restructure (both 2 and 3):** `escalate` now returns
   > `Option<u64>` (§1, BIP125-floor fix), not `u64`. Each site becomes
   > `let Some(new_fee) = self.fee_bump.escalate(…) else { return Ok(None); /* skip
   > the bump this tick */ };` — placed *after* the existing dust-fallback check
   > (`amount <= new_fee + DUST_LIMIT_SAT → rebroadcast`), which still runs on the
   > `Some` value. `None` means "already at the cap, nothing relayable to do."
4. **`ensure_can_fund`** (`engine.rs:688-691`):
   `saturating_mul(FUNDING_BUMP_FEERATE_MULT)` →
   `saturating_mul(self.fee_bump.funding.reservation_mult)`. The upper clamp bound
   becomes `self.fee_bump.max_feerate_sat_vb` — but **the clamp must be made
   panic-safe**: `u64::clamp(min, max)` panics when `min > max`, and the existing
   `min` is `FUNDING_FEERATE_FALLBACK = 20`, so any `max_feerate_sat_vb < 20` would
   crash the engine on the next fundability check. Write it as
   `.clamp(FUNDING_FEERATE_FALLBACK.min(max_feerate), max_feerate)` (where
   `max_feerate = self.fee_bump.max_feerate_sat_vb`) so a low ceiling lowers the
   floor with it instead of panicking. **Belt-and-suspenders:** `validated()` also
   enforces a sane `max_feerate_sat_vb` lower bound (§1) — but the panic-safe clamp
   is the real fix, since it also covers any *other* const floor a future caller
   might pair with the ceiling.
5. **`cpfp_child_fee`** (`engine.rs:393`): stays pure, but its hardcoded
   `.max(MIN_SPEND_FEE_SAT)` floor (line 407) **must** become a passed-in `min_fee:
   u64` arg — otherwise the CPFP floor silently ignores the now-configurable
   `min_fee_sat`. Callers pass `self.fee_bump.min_fee_sat`. At the call site in
   `adaptor_cpfp_bump` (`engine.rs:2001`), also clamp `target` to
   `max_feerate_sat_vb` (today it's the raw estimate, already ≤500 by the backend
   clamp, but make the policy ceiling explicit).
6. **`init` validation** (`engine.rs:1041-1044`): **leave unchanged.** This range
   validates the *counterparty's* `redeem_feerate_a/b` and is a **protocol-level**
   bound — its error message cites "spec v2 §5" and the in-code comment notes "both
   parties must use the exact same value or the MuSig2 sighashes won't match."
   Routing it through a local preference would break interop: a peer who lowered
   `max_feerate` to 200 would reject a perfectly valid offer carrying
   `redeem_feerate = 300`. The local fee-bump policy governs *our bump behaviour*,
   not *acceptance of a counterparty's parameters*. So this check keeps the fixed
   `MAX_REDEEM_FEERATE` const (= 500) — which is why that const stays (§0 table).

7. **Estimator clamp — UNCHANGED (revert of #6).** The backend `MAX_SAT_PER_VB`
   clamp in `fee_rate_sat_per_vb` (`chain.rs:325/333`, `:677/688`) **stays at 500**.
   Because `max_feerate_sat_vb` is validated to `[1, 500]` (§1), that single existing
   clamp enforces the ceiling across all ~8 fee-computation sites for free — no
   per-site re-cap, no weakened glitch protection. The bump sites still apply
   `max_feerate_sat_vb` explicitly where they compute a target (the escalators via
   `escalate` §1; the funding nurse §4.4; the v2 CPFP `target` §2.5/§5.6), which only
   *tightens* the ceiling below 500 when the operator lowers the knob. No new const,
   no `ESTIMATOR_SANITY_MAX`.

**Const cleanup:** delete `ADAPTOR_REDEEM_FEERATE_MULT` and
`FUNDING_BUMP_FEERATE_MULT` once the last reader is gone; **keep**
`MAX_REDEEM_FEERATE` (still used by the §2.6 protocol bound), `REGTEST_REDEEM_FEERATE`,
both `*_FALLBACK`, `FUNDING_VSIZE_EST`, `CPFP_CHILD_VSIZE`, `MIN_SPEND_FEE_SAT` (now
also the policy default source). Update the doc-comments that reference the removed
consts (and `docs/handbook-pact/chapters/ch18-fees-refunds.md`, which still documents
the old `3×` / const names — see §9).

**Acceptance for deliverable 1:** `cargo test` + all e2e green with **no** test
edits (defaults reproduce old numbers); `cargo clippy` clean. The only intended
behavioural delta is RBF escalators now topping out at `max_feerate_sat_vb` — which
no existing test exercises (they never escalate past 500 sat/vB).

---

## 3. Backend additions for the funding nurse

Two new `ChainBackend` methods (default impls `bail!` like `wallet_sign_send` does
at `chain.rs:110`, so only `CoreRpcBackend` + `MultiBackend` delegate need them):

```rust
/// Wallet-tx fee + vsize, for recomputing a funding's broadcast feerate at bump
/// time (fee/vsize) without persisting it. Core: `gettransaction` (has `fee`) +
/// the tx's vsize from `decoderawtransaction`/`getmempoolentry`.
fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64 /*fee_sat*/, u64 /*vsize*/)>;

/// The wallet-owned change output of `funding_txid` (vout, value, spk) for a
/// CPFP child — identified POSITIVELY (the wallet knows its own outputs:
/// `gettransaction.details[].category == "send"` with our address, or the output
/// whose address `getaddressinfo.ismine`), NOT by negating the HTLC script.
/// `None` when the funding has no change output (exact-UTXO funding).
fn wallet_change_output(&self, funding_txid: &str, htlc_spk: &ScriptBuf)
    -> Result<Option<(u32, u64, ScriptBuf)>>;

/// RBF-bump a wallet-owned tx via the node's `bumpfee`, targeting `feerate`
/// (sat/vB). Returns the replacement txid. v1-funding only (the funding is
/// wallet-owned and BIP125-replaceable). Errors if the tx is not replaceable.
fn wallet_bumpfee(&self, txid: &str, feerate_sat_vb: u64) -> Result<String>;
```

`CoreRpcBackend` impls (`chain.rs`, alongside the existing wallet methods ~336-399):
- `wallet_tx_fee_vsize`: `gettransaction <txid>` → `fee` (negative BTC, abs) ×1e8;
  vsize from `decoderawtransaction <hex>` (`.vsize`) or `getmempoolentry`.
- `wallet_change_output`: `gettransaction <txid> true true` includes `decoded`;
  iterate `decoded.vout` and return the output the wallet **owns** —
  `getaddressinfo(<addr>).ismine == true`. The discriminator is clean precisely
  because the HTLC output is a **P2WSH/P2TR script the wallet does not own** (it's
  the swap script, not a wallet key), so `ismine` selects the change output and
  rejects the HTLC output unambiguously — no need to match-by-negating the HTLC spk.
  (A normal 2-out wallet funding has exactly one such output; an exact-UTXO funding
  has none → `None`.)
- `wallet_bumpfee`: `bumpfee <txid> {"fee_rate": <feerate>}`; read `.txid`.

**Prerequisite — make funding RBF-able.** `wallet_send` (`chain.rs:345`) calls
`sendtoaddress` with no `replaceable` arg, so it rides `-walletrbf`. Add explicit
replaceability for the funding broadcast. Two options:

- **(a)** add a `replaceable: bool` param to `wallet_send` and pass
  `{"replaceable": true}` to `sendtoaddress`; callers `fund` (`engine.rs:2277`) and
  `adaptor_fund` (`engine.rs:1531`) pass `true`. *(v2 funding does not need RBF, but
  making it replaceable is harmless and keeps one code path.)*
- **(b)** a dedicated `wallet_send_replaceable`.

Prefer (a) — single path, explicit at the call site.

---

## 4. Funding nurse — v1 (RBF)

**Trigger.** Our own v1 funding leg is broadcast but **not yet `n`-confirmed**, and
we are still before the §7.4 **fund-action margin** for that leg. Concretely:

- Initiator: `(Role::Initiator, State::FundedA)` — the arm at `engine.rs:2757`,
  before it falls through to `try_refund_due(rec, "a")`.
- Participant: `(Role::Participant, State::FundedB)` — the participant's resting
  state after `fund` (`engine.rs:2295`). *Confirm the exact participant tick arm
  that owns this state before wiring; add the nurse call there.*

In both, only act while `get_txout(funding_outpoint)` shows `confirmations == 0`
(unconfirmed) — once it has any confirmation, leave it alone.

**Algorithm** (`fn maybe_bump_funding_v1(&self, rec, leg, backend) -> Result<Option<TickEvent>>`):

1. Resolve `(funding_txid, funding_vout, htlc_spk, chain, amount)` for `leg`.
2. `confs = backend.get_txout(outpoint, &htlc_spk)?` — if `Some` with
   `confirmations >= 1` or `None`/spent, return `Ok(None)` (nothing to nurse).
3. Deadline gate: `action_safe(now, fund_margin, deadline)` using the same
   `action_margins(net)` / `deadline_clock` machinery as `fund` (`engine.rs:2247`).
   Past the margin → return `Ok(None)` (let it stall → refund; design §"On
   deadline-miss").
4. Compute current vs. broadcast feerate, and the chase target:
   - `(old_fee, vsize) = backend.wallet_tx_fee_vsize(&funding_txid)?`;
     `old_feerate = old_fee / vsize`.
   - `market = backend.fee_rate_sat_per_vb()?` (≤ 500 by the backend clamp).
   - `target = market`
       `.min(self.fee_bump.max_feerate_sat_vb)`
       `.min(self.fee_bump.funding.reservation_mult.saturating_mul(old_feerate))`.

     **The reservation bound must be `reservation_mult × old_feerate`, NOT
     `× market`** (a `× market` bound is vacuous: `reservation_mult ≥ 1` makes
     `min(market, mult × market) == market`, clamping nothing). The funds gate
     (`ensure_can_fund`, `engine.rs:688-691`) reserved only
     `clamp(live_at_post × reservation_mult, …, max_feerate_sat_vb) ×
     FUNDING_VSIZE_EST` sat of
     headroom. The funding is broadcast (`sendtoaddress`) microseconds after the
     gate read `live`, so `old_feerate = old_fee / vsize ≈ live_at_post` — exactly
     the quantity the reservation was sized against. Bounding the target by
     `reservation_mult × old_feerate` therefore keeps the bump **within the reserved
     balance**, so `bumpfee` is **very unlikely** to fail insufficient-funds in the
     large-spike scenario the nurse exists for. (Not *impossible*: `ensure_can_fund`
     is a soft pre-flight read of `wallet_balance()`, not a lock — a concurrent swap
     can have consumed the headroom since, and the real funding vsize can exceed
     `FUNDING_VSIZE_EST = 250` for a multi-input funding, so `target × real_vsize`
     can overshoot `ceiling × 250`. Both are reachable; both are handled as a
     graceful no-op, §9.) Still needs no persisted field (§6 intact). In a
     real spike `market ≫ old_feerate`, so this bound — not `max_feerate` — is what
     actually limits the chase, and it tops out at what the wallet can afford →
     stall → refund (the accepted liveness outcome).
   - If `target <= old_feerate` → `Ok(None)` (already paying enough).
5. `new_txid = backend.wallet_bumpfee(&funding_txid, target)?`.
6. **Re-locate the HTLC output** on the replacement:
   `new_vout = backend.find_vout(&new_txid, &hex::encode(htlc_spk.as_bytes()))?`
   (the bump funds itself from change; the HTLC output value is unchanged but its
   vout can move).
7. **Rebuild + re-sign + persist the refund** against the new outpoint — this is
   purely local (v1 refund is a single-key CLTV spend, `build_refund_tx`,
   `swap.rs:210`), no counterparty round:
   - `outpoint' = {new_txid, new_vout}`; rebuild with `build_refund_tx(&htlc,
     outpoint', amount, dest, fee, &swap_key)` exactly as `fund` does at
     `engine.rs:2304-2315`.
   - `rec.refund_tx_hex = Some(serialize_hex(refund'))`.
8. **Update the stored funding pointer** (`htlc_a_txid`/`htlc_a_vout` or `_b_`).
9. **Persist (crash-safe):** update `refund_tx_hex` **and** the funding pointer
   (`htlc_*_txid`/`htlc_*_vout`) on `rec` and write them in a **single**
   `store.put(&rec)` — so intra-struct field order is moot; the pointer and refund
   move atomically. The only real window is `bumpfee` → that one `put`. If we crash
   there:
   - the stored funding pointer + refund are **stale** (they reference the replaced
     outpoint). The stale refund simply *fails to broadcast* on the next tick (it
     spends an outpoint that no longer exists) — harmless, **not a loss**; and
   - `find_funding` is **spk-based** (`chain.rs:208`, scans the UTXO set by script),
     so on restart the locator re-discovers whichever txid now pays the HTLC script,
     the record re-syncs, and the refund is rebuilt against the live outpoint on the
     next nurse pass. Self-healing.
10. Emit `TickEvent { action: "funding-fee-bump", detail: "{new_txid} (funding
    feerate {old_feerate} -> {target} sat/vB)" }`.

**Counterparty safety (load-bearing, from design):** the taker detects the lock by
**scriptPubKey, not txid** (`find_funding` → `scantxoutset raw(<spk>)`,
`chain.rs:208`). An RBF that keeps the HTLC output identical is invisible to their
detection. And the nurse runs only while the funding is unconfirmed — before the
taker has waited out `n_a` confs — so no downstream tx of theirs exists yet.

---

## 5. Funding nurse — v2 (CPFP-via-change)

RBF is **forbidden** on v2 funding: the funding outpoint feeds the 2-of-2 MuSig2
adaptor sigs already exchanged; changing the txid invalidates them and redoing the
MuSig2 round needs the counterparty. So bump the **change output** with a child,
leaving the funding outpoint fixed. Mirrors the redeem-side `adaptor_cpfp_bump`
(`engine.rs:1992`).

**Trigger.** Our own v2 funding leg broadcast, unconfirmed, before the fund margin.
In `adaptor_tick_one` (`engine.rs:1727`), state `Signed` (post-fund, pre-redeem),
gated on our leg's funding being unconfirmed. (Initiator nurses leg A; participant
nurses leg B — symmetric to v1.)

**Algorithm** (`fn maybe_bump_funding_v2(&self, rec, leg, backend) -> Result<Option<TickEvent>>`):

1. Resolve `(funding_txid, funding_vout, leg_spk, amount)`.
2. Unconfirmed check via `get_txout` (as v1 step 2).
3. Deadline gate (as v1 step 3).
4. `Some((change_vout, change_value, change_spk)) =
   backend.wallet_change_output(&funding_txid, &leg_spk)?` **else** `Ok(None)` —
   exact-UTXO funding with no change can't be CPFP'd → stall → refund (acceptable
   edge case, design §"v2 → CPFP").
5. `(parent_fee, parent_vsize) = backend.wallet_tx_fee_vsize(&funding_txid)?`;
   `old_feerate = parent_fee / parent_vsize`.
6. `market = backend.fee_rate_sat_per_vb()?` (≤ 500 by the backend clamp);
   `target = market`
       `.min(self.fee_bump.max_feerate_sat_vb)`
       `.min(self.fee_bump.funding.reservation_mult.saturating_mul(old_feerate))`.
   Same reservation bound as v1 step 4 — `× old_feerate`, **not** `× market` — kept
   for consistency with v1. (Note: the v2 child's actual affordability guard is step
   8's `child_value > DUST_LIMIT_SAT` check against the change-output value — that is
   the real budget, not the `ensure_can_fund` headroom; the `× old_feerate` bound is
   a consistency cap, not the affordability gate.) If `target <= old_feerate` →
   `Ok(None)`.
7. `child_fee = cpfp_child_fee(parent_fee, parent_vsize, target,
   self.fee_bump.min_fee_sat)?` else `Ok(None)` (parent already clears target).
   *Reuse the existing pure fn with the new `min_fee` arg from §2.5; it uses
   `CPFP_CHILD_VSIZE` (150) for the child — that constant assumed a 1-in/1-out sweep
   child, which matches here.*
8. `child_value = change_value.checked_sub(child_fee)?`; `> DUST_LIMIT_SAT` else
   `Ok(None)`.
9. Build the child spending `{funding_txid, change_vout}` → a fresh
   `wallet_new_address`, value `child_value`, and
   `backend.wallet_sign_send(&child, change_value, &change_spk)?` (the existing CPFP
   signer at `chain.rs:367` — the change output is wallet-owned, so it signs).
10. **No record-pointer rewrite needed** — the funding outpoint is unchanged, the
    adaptor sigs stay valid, the refund stays valid. Persist nothing except,
    optionally, the child txid for observability. This is *much* simpler than v1.
11. Emit `TickEvent { action: "funding-cpfp-bump", detail: "{child_txid} (package
    -> {target} sat/vB)" }`.

> The asymmetry, one line: v1's outpoint-dependent downstream is **single-key**
> (re-sign locally) → RBF; v2's is **2-of-2** (needs the counterparty) → CPFP.

---

## 6. Record / store fields

**No new *swap-record* fields required** (the policy gets its own separate
local-config store record — decision #4, §7 — which is unrelated to per-swap state).
Confirmed by reading the flow:

- v1 funding pointer already persists (`htlc_a_txid`/`htlc_a_vout`,
  `htlc_b_txid`/`htlc_b_vout`) and the refund (`refund_tx_hex`) at
  `engine.rs:2285-2316`. The v1 nurse rewrites these in place.
- The broadcast feerate is **recomputed** at bump time (`fee / vsize` via
  `wallet_tx_fee_vsize`) — no new persisted field, per design §"Backend".
- v2 needs nothing new (outpoint fixed; child is fire-and-forget).

If, during implementation, the participant's resting v1 state turns out not to
distinguish "funded mine, waiting" from a state we shouldn't nurse, prefer gating
on the **unconfirmed `get_txout`** result over adding a new state.

---

## 7. Plumbing — RPC-driven, typed params, no JSON (decision #4)

**Decision #4:** the fee policy is **not** passed as a JSON blob. It is driven
through pactd's typed RPC (the same `match method` dispatch every other command
uses, `main.rs:360`), so a CLI/power user sets it with named typed params — and
pactd becomes the **single owner** of this setting (a deliberate, scoped exception
to "Satchel owns config / pactd holds none", justified by the requirement that
CLI/RPC be the source of truth). Satchel's Fees page is just another RPC client.

### libswap
- `Engine.fee_bump` field + default (§1). Add `Engine::set_fee_bump(&mut self,
  policy: FeeBumpPolicy) -> Result<()>`: `validated()` → set `self.fee_bump` →
  **persist to the merchant store**. The store gains a small typed local-config
  record for the policy (it already persists per-merchant state; this is one more
  blob). On `Engine::open`, if the store holds a policy it overrides the default —
  so a CLI-set policy survives restart with no Satchel involved.
- Re-export `pub use fee_policy::FeeBumpPolicy;` in `libswap/src/lib.rs` (both pactd
  and Satchel import it; both already depend on libswap).

### pactd — two new RPC methods (typed, in `dispatch`, `main.rs:363`)
- `getfeepolicy` → returns the effective policy as typed JSON
  (`{ max_feerate_sat_vb, min_fee_sat, reservation_mult, committed_mult, step_pct }`).
- `setfeepolicy` → named **optional** typed params (only the fields supplied
  change; the rest keep their current value), read via the existing `Params`
  helper, e.g. `p.get::<u64>("max_feerate_sat_vb")?`. Calls
  `engine.set_fee_bump(merged)` → validates server-side, updates the **live**
  engine, persists. Returns the new effective policy. No JSON-string arg anywhere —
  each field is a normal typed RPC param, callable from the CLI like every other
  method.
  - **Multi-merchant scoping.** pactd builds one engine *per merchant*
    (`build_engine`, `merchants.rs`). `get/setfeepolicy` resolve the engine the same
    way the other per-merchant RPCs do — the **active merchant** (`active_id`,
    `merchants.rs:402`), or an explicit `merchant` param if the RPC convention uses
    one. The policy record is therefore **per-merchant** (each merchant's store holds
    its own), matching how seeds/coins are already per-merchant. Confirm against the
    existing per-merchant RPC pattern when wiring.
- `EngineConfig` (`merchants.rs:82`): **no `fee_bump` field needed** — the policy
  lives in the store now, loaded at `Engine::open`. (If we want a first-run seed,
  optional per-field clap flags `--fee-max-feerate`, `--fee-min`,
  `--fee-reservation-mult`, `--fee-committed-mult`, `--fee-step-pct` can set the
  *initial* policy when the store has none; the stored value wins thereafter.
  Recommend including them — they're cheap, typed, and let a headless launch pin a
  policy. Still no JSON.)

### Satchel — RPC client, no satchel.json field
- The Fees page reads via `getfeepolicy` and writes via `setfeepolicy` (a thin
  `#[tauri::command]` that forwards typed params to the daemon, modeled on the
  existing RPC-forwarding commands, registered in `invoke_handler` `main.rs:1134`).
- **No `fee_bump` in `Config`/`satchel.json` and no launch flag** — pactd's store
  is the single source of truth, avoiding a dual-owner conflict. A `setfeepolicy`
  call takes effect **live** (no pactd relaunch), which is also better UX than the
  board-URL pattern.

---

## 8. Satchel Settings → Fees page

New **Fees** section under Settings (React/TS/MUI, `satchel/ui/`). Advanced /
collapsible — most users never open it. All copy goes through `en.ts` (i18n lint
guard is enforced — see the project's i18n rule).

> **Per-merchant scope.** The policy is per-merchant (decision #4: pactd store,
> active-merchant scoped). The page edits the **active merchant's** policy, not an
> app-wide one — so values change when the active merchant switches. State this in
> the section heading/subtitle (e.g. "Fees — for the active merchant") so a
> multi-merchant user isn't surprised.

Four controls, each a number field with the default pre-filled and inline help:

| Control | Field | Help copy (intent) |
| --- | --- | --- |
| Absolute max feerate (sat/vB) | `max_feerate_sat_vb` | "Ceiling for every fee bump. Default 500 (also the hard system maximum). Lower it to cap costs." (UI min 1, **max 500**.) |
| Funding bump reservation (×) | `funding.reservation_mult` | "Balance the funds check sets aside as bump headroom. Higher = rescues bigger fee spikes but ties up more balance and rejects more swaps. Default 3." |
| Redeem over-provision (×) | `redeem.committed_mult` | "How much extra the v2 redeem fee is pre-paid so it confirms even if Satchel is closed. **Applies to new swaps only.** Default 2." |
| RBF escalation step (%) | `redeem.step_pct` / `refund.step_pct` | "How aggressively a stuck spend's fee climbs each scheduler pass. Default 50%." |

> Decide: expose `redeem.step_pct` and `refund.step_pct` as **one** "RBF escalation
> step" control (they default the same and users won't distinguish) or two. Recommend
> **one** control bound to both for simplicity; split only if a user asks.

- **"Reset to defaults"** affordance.
- A persistent note: *"These are safety/cost trade-offs, not required setup. New
  values apply live to future bumps; swaps already funded keep the `committed_mult`
  and gate reservation they were funded under (both fixed at funding time)."*
- On save → a `#[tauri::command]` that forwards the changed fields to the daemon's
  **`setfeepolicy` RPC** (typed params, decision #4) — applied live, no relaunch.
  The page loads its initial values from **`getfeepolicy`**.

---

## 9. Regtest, docs, and edge cases

- **Regtest pin.** `REGTEST_REDEEM_FEERATE = 2` stays and `adaptor_redeem_feerate`
  short-circuits to it on regtest (`engine.rs:523`). **Caveat for the nurse
  (design §Verification):** confirm the *funding* feerate/target path is **not**
  similarly pinned on regtest — if it were, a simulated mempool spike wouldn't move
  the target and the bump would never fire. The funding nurse reads
  `fee_rate_sat_per_vb()` directly (clamps `[1,500]`, no regtest pin), so a raised
  regtest mempool floor *does* move it — good, but verify end-to-end.
- **Docs to update:** `docs/handbook-pact/chapters/ch18-fees-refunds.md` still
  documents the old const names + `3×` redeem mult (lines 57-58, 166-169) and the
  inline `+50%` formula (lines 21-24) — rewrite against the policy. Update the
  design doc table in `FUNDING_FEE_BUMP.md` if any default changes during impl.
- **Funding with no change (v2 exact-UTXO):** can't CPFP → `Ok(None)` → stall →
  refund. Acceptable (design). Worth a one-line `log`/`TickEvent` so it's visible.
- **`bumpfee` rejects (not replaceable):** only possible if the §3 replaceability
  prerequisite is missed; surface the error, don't silently swallow.
- **`bumpfee` rejects insufficient-funds:** reachable despite the funds gate (it's a
  soft pre-flight, not a lock — see §4.4). Treat it as a graceful `Ok(None)` +
  `log`/`TickEvent`, **not** a hard error or crash: the nurse simply can't bump this
  tick → the funding stalls → the refund timelock returns the funds (liveness, the
  accepted worst case). Same handling for the rarer over-vsize overshoot.
- **Cap below current fee:** `escalate` returns `None` (caller skips the bump — no
  +1 nudge, §1) and the nurse's `target <= old_feerate → None` both handle it — no
  panic, no unrelayable replacement, just a clean no-op that tick.

---

## 10. Verification

**Deliverable 1 (refactor):**
- `cargo test -p libswap` + workspace tests, **no test edits** — defaults reproduce
  old numbers.
- `cargo clippy --all-targets` clean.
- Add focused unit tests in `fee_policy.rs`: `escalate` returns the stepped fee
  normally, the cap value when the step exceeds it, and **`None`** once at the cap
  (capped step can't clear the BIP125 `old_fee + vsize` floor); `validated()`
  rejects (zero mult, `max_feerate_sat_vb > 500`, zero `step_pct`); `serde`
  round-trip with missing fields → defaults.
- Add a unit test that `cpfp_child_fee` honours the passed-in `min_fee` (the §2.5
  signature change) rather than a hardcoded floor.
- RPC tests: `setfeepolicy` with a partial field set changes only those, leaves the
  rest; `getfeepolicy` reflects it; `setfeepolicy` → drop+reopen the engine →
  `getfeepolicy` shows the persisted value (survives restart, no Satchel);
  server-side `validated()` rejects an out-of-range `setfeepolicy`.
- e2e suite green unchanged.

**Deliverable 2 (nurse):**
- The nurse can't be meaningfully unit-tested — it needs a **regtest fee-spike
  simulation**: broadcast a funding at a low rate, raise the mempool floor
  (`prioritisetransaction` / fill blocks / `settxfee`), run a scheduler tick, assert
  the bump (v1: replacement txid pays the same HTLC spk at higher feerate, refund
  rebuilt; v2: a CPFP child on the change output lifts the package feerate) lands
  before the fund-margin deadline. Build alongside the local release / regtest
  iteration loop (the playground harness).
- Crash-recovery check (v1): kill between `bumpfee` and the store `put`, restart,
  assert `find_funding` (spk-based) re-discovers the live outpoint and the swap
  proceeds.
- Counterparty-invisibility check (v1): a second engine watching by spk
  (`find_funding`) still detects the lock across an RBF.

---

## 11. Suggested PR breakdown

1. **PR-1 `fee-policy-refactor`** — §1 (struct + module + re-export + `validated`,
   `[1,500]` ceiling), §2 (rewire all existing sites, delete dead consts,
   panic-safe `ensure_can_fund` clamp; estimator clamp unchanged §2.7), §7 (engine
   store-persistence + `set_fee_bump`,
   pactd `getfeepolicy`/`setfeepolicy` RPC + optional seed flags, *no UI yet*).
   Behaviour-preserving (defaults unchanged); full suite green.
2. **PR-2 `fees-settings-page`** — §8 Satchel Settings → Fees UI + a thin
   `setfeepolicy`/`getfeepolicy`-forwarding `#[tauri::command]` + en.ts copy. Pure
   frontend/IPC on top of PR-1's RPC.
3. **PR-3 `funding-nurse-backend`** — §3 backend methods (`wallet_tx_fee_vsize`,
   `wallet_change_output`, `wallet_bumpfee`) + replaceable funding (`wallet_send`
   param). Unit-test the pure bits; no scheduler wiring yet.
4. **PR-4 `funding-nurse-v1`** — §4, wired into `tick_one`, + regtest spike harness.
5. **PR-5 `funding-nurse-v2`** — §5, wired into `adaptor_tick_one`.
6. **PR-6 `fees-docs`** — §9 handbook + design-doc reconciliation.

PR-1/PR-2 are landable immediately and carry all the user-facing value of the
unification. PR-3→PR-5 are the liveness improvement and depend on the regtest
harness; PR-6 closes out the docs.

---

## 12. Decisions — LOCKED

All design decisions are settled; the remaining items are implementation details
to confirm while coding, not choices.

1. **Default parameter values — KEEP.** `max_feerate_sat_vb = 500`,
   `min_fee_sat = 1000`, `reservation_mult = 3`, `committed_mult = 2`,
   `step_pct = 50` (= today's audited consts). Users override; defaults reproduce
   current behaviour. (§1.)
2. **Expose all four knobs in the Fees page — YES.** Max feerate, funding
   reservation ×, redeem over-provision ×, RBF step %. Collapsible/advanced. (§8.)
3. **RBF step: ONE UI control** bound to both `redeem.step_pct` + `refund.step_pct`.
   (§8.)
4. **pactd surface: typed RPC, NO JSON (decision #4).** `getfeepolicy` /
   `setfeepolicy` with named typed params; pactd's store owns the policy (persists
   across restart); Satchel is an RPC client; optional per-field seed flags. (§7.)
5. **`max_feerate_sat_vb` is a knob bounded by a hard 500 ceiling (decision #6
   REVERTED, per review).** It stays a real, Satchel-exposed param settable in
   `[1, 500]`; the existing 500 estimator clamp enforces it everywhere for free, so
   the knob only *tightens* the ceiling below 500. We do **not** raise the estimator
   clamp to allow >500: that would strip glitch protection from ~8 fee-computation
   sites and force a per-site re-cap, buying only an extreme regime these chains
   don't need. Default 500; behaviour unchanged. (§1, §2.7.)
6. **Reservation bound = `reservation_mult × old_feerate`**, not `× market` (the
   latter is vacuous). Keeps every bump within reserved balance, no persisted field.
   (§4 step 4 / §5 step 6.)
7. **Funding broadcast made BIP125-replaceable** via a `replaceable` arg on
   `wallet_send` (one path, explicit at the call site). (§3.)

### Implementation details to confirm while coding (not decisions)

- **Participant v1 resting state.** Confirm the exact `(role, state)` arm that
   hosts the participant's funding-B nurse; prefer gating on the unconfirmed
   `get_txout` over introducing a new state. (§4 / §6.)
- **Optional seed flags.** Whether to ship the per-field `--fee-*` first-run seed
   flags or rely solely on `setfeepolicy`; recommended to include, trivially
   droppable. (§7.)
