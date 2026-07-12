# Swap state reconstruction from chain ground truth — design (follow / rescue / takeover)

Status: IMPLEMENTED — #167 (§1–§5, §7 items 1–9), #168 (follower progress
line, was §7.10) and #171 (wallet-assisted tier L+, see §4.1a below);
deferred: subscription pokes and the tier-L scan throttle (§7.11/P5).
Companion to `MULTI_MACHINE_122.md` (#122/#134/#163/#164) and the #54 rescue
path. Triggered by the 2026-07-12 mainnet field bug: a follower that first
sees a swap *after* completion is a permanent `state=accepted` ghost.
Verified by `libswap::reconstruct` unit tests and
`harness/test_follow_e2e.py` (observe-after-completion, dormant-observer
takeover, and a Core-only observer resolving via the shared wallet).

**Amendment (#171, 2026-07-12).** The tier framing below originally treated
Core-RPC coins as history-blind. Field review corrected the deployment
contract: a multi-machine **backup session MUST point at the same node
wallet** as the primary (a takeover funds from it; v2 sweeps pay into it) —
so on Core-RPC coins the wallet's own history is shared evidence covering
every transaction the merchant side made. See §4.1a; escalation E6 is
retired by the same contract (sweeps land in the shared wallet by
definition — the requirement is now documented instead of escalated).

---

## 0. Verdict up front

**Root cause.** The follow evaluator asks a *live-state* question about a *historical
fact*: `follow_leg` discovers funding with `find_funding(spk)`
(`engine.rs:5212`), which is a live-UTXO read on every backend — Electrum
`blockchain.scripthash.listunspent` (btcx `backend.rs:840`), Core `scantxoutset`
(`chain.rs:684`). Once the HTLC/P2TR output is spent (i.e. **every completed swap**),
those return nothing, `res = Some(false)` = "not funded yet" forever
(`engine.rs:5217`), and `purge_followed_if_deep` never fires because `both_resolved`
requires having *first* seen the funding live (`engine.rs:5200-5209`).

**Fix shape.** One pure, idempotent per-leg classifier over the script's full
*history* (funding **and** spend, present or past), a whole-swap state derived as a
pure function of both leg classes, and a clock-based age-out as the universal
fallback for history-less backends. Every evaluation reconstructs from chain ground
truth; liveness (ticks, subscriptions) only *triggers* re-evaluation.

**Driver hardening: yes, but surgically.** The driver's action state machine
(`tick_one` / `adaptor_tick_one`) is *not* rewritten — it is already chain-grounded
at its decision points and carries the §7.4 safety gates. But it shares three
provable blind spots with the follower, all of the same "live question, historical
fact" species, and all three are closed by reusing the *same* classifier as a
pre-action gate (§5). The reconstruction becomes the driver's gatekeeper, not its
brain.

---

## 1. The state model — per-leg classification (v1 AND v2)

### 1.1 The classifier (pure function)

```
classify_leg(backend, spk, expected_amount, hints{ptr, height}) -> LegClass

LegClass =
  | Unknown                       // no capable view answered / chain error → keep record, retry
  | Unfunded                      // no output ever paying (spk, amount) visible
  | Funded   { op, confs, height }
  | SpentRedeem { op, spend_txid, spend_confs, witness }   // secret is ON CHAIN
  | SpentRefund { op, spend_txid, spend_confs }
  | SpentUnknown { op, spend_txid, spend_confs }           // witness matches neither — anomaly, NEVER purge
```

Inputs are all derivable from the accept snapshot alone: the v1 P2WSH spk from
`SwapParams::htlc_a/htlc_b` (`swap.rs:113-130`, script `htlc.rs:62-87`), the v2 P2TR
spk from `AdaptorSwapParams::leg_a/leg_b` (`adaptor_swap.rs:87-97`,
`taproot.rs:94-98`). `hints` (a previously persisted outpoint/height) are index
accelerators only — **never** truth; a hint that contradicts history is discarded
and the classification re-derived.

**History-capable backend (Tier H, §4):** fetch `history(spk)` — Electrum
`blockchain.scripthash.get_history`, funding + spends, confirmed + mempool (btcx
`backend.rs:710`; batched variant `backend.rs:620`). Funding = the tx in history
with an output paying exactly `(spk, expected_amount)` (byte- and value-matched, the
existing verification discipline of `locate_funding`, `engine.rs:4280`). Spend = the
tx in history with an input spending that outpoint (exactly what Electrum
`find_spend_witness` already walks, btcx `backend.rs:885-906`). Both are
self-verifying against locally reconstructed bytes — a lying server can withhold,
never substitute (the `chain.rs:1-6` doctrine).

**Live-only backend (Tier L, §4):** degrade to today's primitives — known pointer →
`get_txout` (live/spent), no pointer → `find_funding` — plus `find_spend_witness`
bounded by the *persisted funding height* (§5.3). A leg whose funding was spent
before this machine ever saw it live is `Unknown` on Tier L; the age-out (§4.2)
is the terminal decision there.

### 1.2 v1 spend classification (P2WSH witness shapes — deterministic)

The witness script is `IF <hash branch> ELSE <CLTV branch> ENDIF` (`htlc.rs:62-87`);
consensus requires the last witness item to be the script itself, so any spend of
the outpoint is structurally ours. The two spend shapes are fixed by our builders:

| shape | witness | classify |
|---|---|---|
| redeem | `[sig, pubkey, s, 0x01, witness_script]` (`swap.rs:182-211`) | a 32-byte item hashing to `hash_h` → **SpentRedeem** — reuse `extract_preimage` (`htlc.rs:104-114`), already hash-verified, positionless |
| refund | `[sig, pubkey, <>, witness_script]` (`swap.rs:213-241`) | no valid preimage + empty selector + (sanity) spend-tx `nLockTime ≥ T` → **SpentRefund** |
| other | anything else | **SpentUnknown** |

### 1.3 v2 spend classification (Taproot — witness structure IS the classification)

The leg has exactly **one** tapleaf (`taproot.rs:85-91`), so only two spend paths
exist by consensus:

| shape | witness | classify |
|---|---|---|
| cooperative redeem | exactly 1 item, 64 bytes (SIGHASH_DEFAULT; 65 with an explicit sighash byte) — `attach_keypath_signature` (`taproot.rs:190-196`) | **SpentRedeem**. Any key-path spend *is* the MuSig2 aggregate signing — no other key can produce it. Nothing script/preimage-shaped is revealed; the witness *sig itself* is the evidence. |
| CLTV refund | 3 items `[sig, leaf_script, control_block]` (`taproot.rs:250-254`), item[1] byte-equal to the locally rebuilt `refund_script()` (`taproot.rs:73-81`) | **SpentRefund** |
| other | anything else | **SpentUnknown** (a fabricated/garbage witness — see §6 anti-DoS) |

**Does a follower need `t`? No — and this is a hard separation.** A read-only
follower classifies by witness *structure* only; it never derives, imports, or
stores a secret. Only a *taken-over* record that must CLAIM needs one, and both
claim paths already exist and are seed+chain-derivable:

- v1 participant: `s` from the redeem witness via `extract_preimage`
  (driver: `engine.rs:4062-4076`, `engine.rs:5550-5568`).
- v2 participant: `t = reveal_from_onchain(adaptor_sig_b, final_sig)` — needs
  `adaptor_sig_b` (in the **Signed** snapshot, `engine.rs:6201-6216`) + the 64-byte
  key-path sig from the chain witness (driver: `engine.rs:2696-2707`).
- v2 initiator: `t` from seed — `adaptor_secret(DeriveScope(rec.derive_scope),
  swap_index)` (`engine.rs:2635-2641`), scope travels in the record so any
  same-seed machine re-derives it.
- Both protocols, both roles: refund keys are pure seed derivations.

Corollary (escalation E1): a v2 takeover from an **accept-only** snapshot (owner
died before `Signed` published) holds no adaptor sigs → it is **refund-only**. The
takeover UI must say so.

### 1.4 Whole-swap derived state (pure function of both leg classes)

```
derive(v, LegA, LegB, clock) -> DerivedSwap
```

| LegA | LegB | derived | action (follower) |
|---|---|---|---|
| Unfunded | Unfunded | PreFunding | keep; age-out eligible (§4.2) |
| Funded | Unfunded | InFlight(A locked) | keep, show progress |
| Funded | Funded | InFlight(both locked) | keep |
| Funded | SpentRedeem | RevealPublished (secret on chain, A claimable) | keep — this is the takeover-worthy moment |
| SpentRedeem | SpentRedeem | **Terminal: Completed** | purge when deep (§3) |
| SpentRefund | SpentRefund | **Terminal: Refunded** | purge when deep |
| SpentRefund | Unfunded (or vice versa) | **Terminal: RefundedPartial** | purge when deep + aged past the other leg's fund window |
| SpentRedeem | SpentRefund (any mix) | **Terminal: Mixed** | purge when deep, but emit a distinct anomaly event (this is the §7.4 loss window — post-mortem material, escalation E5) |
| any SpentUnknown | — | Anomaly | NEVER purge; flag in dock |
| any Unknown | — | Indeterminate | keep untouched (today's error discipline, `engine.rs:5177-5180`) |

Terminal purge condition (replaces the tip-advance memo where depth is knowable):
**every funded leg is Spent\* with `spend_confs ≥ max(n_a, n_b)`** — direct spend
depth from history heights, not `FOREIGN_RESOLVED_AT_PREFIX` tip-drift
(`engine.rs:5225-5271`). The memo mechanism is retained **only** for Tier L, where a
known-pointer spend is visible (`get_txout → None`) but its depth is not.

The same table serves v1 and v2; only §1.2 vs §1.3 differ. `RedeemedB`-style
intermediate driver states never appear here — they are *action* states, not chain
facts.

---

## 2. Evaluation architecture — idempotent, no mode switch

**The invariant:** persisted record state is a *cache of a derivation*; chain is the
only truth. Every tick, for every non-driven record:

```
tick → follow_one/follow_adaptor_one
     → classify_leg(A), classify_leg(B)     // pure chain reads, hints from record
     → derive(...)                          // pure function
     → persist hints (ptr, funding height, spend txid) — status-only writes,
       exactly today's discipline (engine.rs:5301-5312)
     → purge / flag / update progress from the DERIVED state only
```

There is no history-phase vs live-phase: the classifier answers the same question
whether the swap completed a minute or a month ago, so a restart, a gap, a missed
subscription callback, or a laptop lid closing mid-swap all converge on the next
evaluation. Ticks (30 s scheduler) are the baseline trigger; Electrum
`subscribe_spks` (btcx `backend.rs:181`, already feeding the #87 sync workers) can
be *added* as an accelerator in a later phase — as a trigger to re-evaluate, never
as a source of state.

**Cache-conflict rule:** a persisted hint contradicted by history (pointer's tx not
in history, height mismatch after reorg) is dropped and re-derived. Follower writes
never touch `rec.state` (the driving enum); derived phase surfaces through the
progress map / `listswaps` for the dock instead, so a later takeover can never
inherit a follower-fabricated driving state.

**How follow / rescue / adopted-drive share it (one reconstruction, three consumers):**

| consumer | uses reconstruction as | writes |
|---|---|---|
| follow (foreign scope, `drives()=false`, `engine.rs:4412`, `:4460`) | full state derivation + purge decision | hints + purge |
| takeover / #54 rescue adopt (`take_over_swap` `engine.rs:6298`, `rescue_from_blobs` `:6251`) | one-shot **fast-forward** at the adoption boundary (§5.2) | maps DerivedSwap → driving state, confirm-gated as today |
| adopted / own drive (`tick_one`, `adaptor_tick_one`) | **pre-action gate** at the funding/claim seams (§5.1) | none — refusals only |

---

## 3. Reorg safety

- Terminal/purge requires positive spend evidence at `spend_confs ≥ max(n_a,n_b)` —
  same finality budget as today's buffer, but measured on the spend itself.
- Idempotency is the reorg handler: a spend that reorgs away simply re-derives as
  `Funded` next tick; nothing latches (no memo to reset).
- Purge depth reads take the conservative direction: on MultiBackend, spend
  confirmations via the min-over-quorum discipline (`tx_confirmations_min`,
  `chain.rs:1457-1464`).
- Note an *improvement* over today: `follow_leg`'s "pointer now missing = spent"
  (`engine.rs:5205-5209`) inherits MultiBackend `get_txout`'s any-view-missing veto
  (`chain.rs:1544-1546`) — one laggy view can currently start the terminal buffer.
  History classification demands a *positive, self-verifying* spend tx instead.

---

## 4. Backend capability tiers + the timelock age-out

### 4.1 Tiers

| tier | backends | guarantee |
|---|---|---|
| **H — history** | any coin with ≥1 Electrum view: all nodeless coins (`wallet_bdk.rs` delegates to the pool), and Core-primary coins with Electrum views configured in the MultiBackend | full front-to-back reconstruction, §1 |
| **L+ — wallet-assisted** (#171) | Core-RPC-only coins, via the SHARED node wallet (the backup-session contract) | everything the merchant side did: fundings we sent (pointer survives the spend), claims/refunds we received (full spend classification incl. witness). Counterparty-only history stays invisible → resolved via the pointer-vanished tip-drift buffer |
| **L — live-only** | Core-RPC-only coins, wallet inconclusive (fresh node, oversized scan) | in-flight detection (live UTXO + pointer + bounded spend scan) + **age-out** as the terminal fallback |

Detection is structural, not probed: a new trait method returns `Ok(None)` where
unsupported (§7 item 2), and MultiBackend fans out to capable views — the tier is
simply "did any view answer". Important floor fact: Core has **no**
script→history index at all — `txindex=1` is txid→tx only, so Tier L cannot be
upgraded by txindex for *funding* discovery of an already-spent spk; only the
spend-of-known-outpoint scan benefits (`chain.rs:732-782` block-scan fallback).

### 4.1a Wallet-assisted evidence (#171)

The backup-session contract — a second machine on the same seed MUST use the
same node wallet URL — turns the Core wallet into shared, positive-only
evidence: `ChainBackend::wallet_txs_since` (one `listsinceblock` + a
`gettransaction` per wallet tx, bounded to the swap's era) feeds
`reconstruct::classify_leg_wallet`, which proves fundings the wallet sent and
classifies spends the wallet received. Claims of counterparty-funded legs are
identified by probe: v1 — the revealed witness script byte-equals the locally
rebuilt one; v2 — the refund leaf byte-equals, or a key-path spend sweeps to
the record's negotiated per-swap sweep address. Absence proves NOTHING (the
counterparty's transactions never touch our wallet): the evidence augments
classification (`LegClass::Vanished` for proven-funded-but-gone), it never
implements `spk_history`. Follow-evaluator scans are throttled to block
cadence (`follow_wallet_tip:` memo) and end once evidence is cached.

### 4.2 Timelock age-out (universal fallback — needs zero new chain capability)

The snapshot always carries absolute `t1`/`t2`. For a **followed** record:

```
aged_out :=
  min_MTP(chain of max(t1,t2)) ≥ max(t1,t2) + AGE_OUT_MARGIN
  AND neither leg currently classifies Funded (live check both legs)
  AND no leg is SpentUnknown (anomalies never silently vanish)
```

- Clock: `tip_median_time_min` (`chain.rs:1477-1484`) — the *laggiest* responding
  view must agree the window is over; MTP is consensus-monotone, so a reorg cannot
  un-pass it by more than the margin.
- `AGE_OUT_MARGIN = max(24h, 6 × max(n_a,n_b) × target_spacing_secs)`; `0` on
  regtest, matching the `action_margins` house style, so the e2e cell is cheap.
- Semantics when it fires: past T, every rational path is closed — any funding that
  existed was redeemed or refunded (we just can't see which on Tier L). Purge with
  a `followed-aged-out` event + the `PURGED_FOREIGN_PREFIX` memo (`engine.rs:5262`)
  so the lingering snapshot never re-imports (`engine.rs:6424-6433`).
- **The one deliberate exception:** a leg that still shows a **live funded UTXO**
  past age-out is money sitting claimable — that is precisely the "owner died"
  case #122 exists for. KEEP the record and escalate in the dock ("stale funded
  swap — take over to refund"), never purge it. (Escalation E2.)

Tier H rarely reaches the age-out (history classifies first); it remains the
backstop there too (e.g. all Electrum views permanently gone).

---

## 5. Driver-path hardening — the three seams (and why not more)

The driver is *already* reconstruction-shaped at its decision points: depth-gated
`get_txout` before every reveal (`engine.rs:3043-3064`), `find_spend_witness`
watches for the counterparty's spend (`engine.rs:5550`, `:3213`, `:2696`),
locate-first funding idempotency (`engine.rs:3885-3897`, `:2432-2445`,
`:2477-2485`), Signed-state pointer rediscovery (`engine.rs:2938-2960`), and MTP
clocks for every deadline. What it lacks is exactly the follower's disease in three
places:

### 5.1 Funding guards must see history, not just live UTXOs (closes a real double-fund)

`fund()`'s guard (`engine.rs:3893` via `locate_funding` `:4251-4297`) and the v2
guards (`engine.rs:2441`, `:2481`) ask `find_funding`/`get_txout` — live-only. A
**rescued or taken-over** record at `accepted`/`Signed` whose leg was already funded
*and spent* by the dead incarnation (tombstone is best-effort and provably lingers —
the field ghost came from exactly such a snapshot) sees "not funded" and **funds
again**. That violates the no-double-fund invariant the brief names. Fix: guard =
`classify_leg` — `Funded` → adopt (today's behavior); `SpentRedeem/SpentRefund/
SpentUnknown` → **refuse + fast-forward to the derived terminal** with a clear
event; `Unfunded` → proceed. Tier L keeps today's live guard plus the §7.4/age-out
clock refusals (a swap past its windows is never funded — cheap and universal).
Anti-DoS: a refusal based on a *spend* requires that spend confirmed ≥1 under the
quorum-min read, so a fabricated unconfirmed spend from one server cannot stall a
live swap (§6.4).

### 5.2 Takeover fast-forwards through the same reconstruction

`take_over_swap` (`engine.rs:6298-6310`) flips `adopted` on whatever state the
snapshot froze — then the driver acts on stale state (a v1 record adopted at
`accepted` re-enters the funding arm; a participant adopted at `funded` may never
learn the reveal already happened). On adoption (and equally on #54
`rescue_from_blobs` adoption), run reconstruction once and map:

- derived terminal → write the terminal state directly (Completed/Refunded), emit
  event, done — never drive a finished swap;
- v1: A funded → `FundedA`; both funded (or B spent-by-redeem) → `FundedB` — the
  existing arms (`engine.rs:5393`, `:5537`) then do the right thing, including
  claiming A from the on-chain preimage;
- v2: stay `Signed` (its rediscovery + two-phase gates already self-heal,
  `engine.rs:2938`, `:3093`, `:3111`) — reconstruction only vetoes (terminal) or
  supplies pointers/heights;
- refuse adoption entirely when derived = terminal-deep (surface "already over,
  nothing to take over" instead) — the confirm-gated door stays, it just can't
  open onto a corpse.

### 5.3 Persist funding heights for both legs, both protocols

Only `htlc_b_height` exists today (`store.rs:85`). The Core spend-scan bound for a
taken-over swap is broken without heights: `funding_scan_from_height`
(`engine.rs:2591-2600`) leans on `tx_confirmations`, which on Core answers via the
*wallet* (`gettransaction`, `chain.rs:797-811`) — a foreign machine's funding is not
in this wallet, `getrawtransaction` needs txindex, result 0 → scan starts at the
tip → a mined reveal below it is invisible → a taken-over participant on Tier L
never extracts `s`/`t`. Add `htlc_a_height` (v1) and `funding_a_height`/
`funding_b_height` (v2), stamped wherever a funding is first seen (follower hint
writes, `locate_funding`, the v2 rediscovery, `funded` message handling) and used as
`from_height` everywhere `find_spend_witness` is called.

### 5.4 What is deliberately NOT done

No rewrite of `tick_one`/`adaptor_tick_one` into a derived-state machine. The
driver holds secrets and spends money; its arms encode *policy* (§7.4 margins,
reveal ordering, two-phase funding, nurse cadence) that a chain-state function
cannot and should not derive. Big-bang refactor = new failure modes in the one
place that must not have them. The classifier gates it; it does not replace it.

---

## 6. Safety invariants — how each survives

1. **FOLLOWED never broadcasts.** Untouched: routing (`engine.rs:4412`, `:4460`),
   the `drives()` rule (`engine.rs:765-767`), the broadcast belt and the `fund()`
   belt (`engine.rs:3825-3829`) all stay. Reconstruction only reads, persists
   status fields, and deletes rows — the same write surface `follow_one` has today.
2. **`adopted` never set by reconstruction; no auto-adopt.** Fast-forward (§5.2)
   runs *inside* the already-confirm-gated `take_over_swap`/`restorefromrelay`;
   import still clears `adopted` (`with_adopted_cleared`, `engine.rs:6387`).
3. **No double-fund on takeover.** Strengthened — that is §5.1. The guard now also
   answers "already funded *and spent*", which today's live guard cannot.
4. **Fabricated-evidence resistance.** Positive evidence is self-verifying (v1
   preimage hashes to `hash_h`; v2 refund leaf byte-equals; v2 key-path is
   consensus-bound to the aggregate). The one fabricatable shape — a garbage
   "key-path" witness — lands in `SpentUnknown` (never purges) unless quorum-min
   confirmed; driver refusals additionally require a confirmed spend (§5.1). Worst
   case from a lying server remains withholding, i.e. delay — the `chain.rs:1-6`
   doctrine holds.
5. **No nonce secrets travel or import.** Snapshots carry record + `next_index`
   only (`engine.rs:6202-6205`, §1 invariant in `follow_foreign_from_blobs` docs
   `:6351-6358`); reconstruction adds nothing to the snapshot. v2 nonce sessions
   remain machine-local; nothing here re-signs.
6. **Reorg-buffered terminal decisions.** §3 — depth on the spend itself, min-over-
   quorum, idempotent re-derivation instead of latched memos.
7. **Own-scope / legacy stay confirm-gated.** Import filters unchanged
   (`engine.rs:6379-6385`); legacy (`derive_scope == 0`) records keep their
   never-purge belt (`engine.rs:5316-5318`, `:5374-5376`) — reconstruction may
   *display* their derived phase but never deletes them.
8. **Bounded chain load.** §8.

---

## 7. Surgical change list (no code here — files & functions)

1. **btcx crate:** nothing. `history`, `batch_history`, `find_spend_witness`,
   `subscribe_spks` already exist (`backend.rs:710`, `:620`, `:885`, `:181`).
2. **`chain.rs` — trait:** add one method, default = incapable:
   `spk_history(&self, spk) -> Result<Option<Vec<(String /*txid*/, i64 /*height*/)>>>`
   (`Ok(None)` = Tier L). Impl: Electrum adapter → `Some(history())`; Core →
   default; `Arc` forwarder (`chain.rs:363`); `wallet_bdk.rs` → delegate to pool;
   MultiBackend → fan out, first capable positive wins, zero *capable* responders
   errors (outage ≠ answer, the `find_funding` discipline `chain.rs:1574-1583`).
   Plus `fetch_tx(&self, txid, spk_hint) -> Result<Option<Transaction>>` for
   inspecting history txs (Electrum `transaction.get`; Core mempool/txindex
   best-effort; MultiBackend any-view, hash-verified).
3. **NEW `libswap/src/reconstruct.rs`:** `LegClass`, `classify_leg_v1`,
   `classify_leg_v2`, `derive_swap`, `aged_out` — pure functions over a
   `&MultiBackend` + params; the §1.2/§1.3 witness tables; unit-testable with
   fabricated witnesses, no engine dependency.
4. **`engine.rs` `follow_leg`/`follow_one`/`follow_adaptor_one`
   (`:5187`, `:5274`, `:5330`):** rebuild on `classify_leg` + `derive_swap`;
   persist pointer+height hints; purge from derived terminal + spend depth;
   keep the legacy-scope never-purge belts verbatim.
5. **`purge_followed_if_deep` (`:5225`):** becomes the Tier-L-only depth buffer;
   Tier H purges on direct spend depth. `PURGED_FOREIGN_PREFIX` memo unchanged.
6. **`follow_foreign_from_blobs` (`:6363`):** best-effort classify-at-import — a
   snapshot whose derived state is terminal-deep is skipped with a
   `followed-skipped-terminal` event (+ purged-memo) instead of ever appearing in
   the dock; on any chain error, import as today and let the tick decide.
7. **Funding guards (`fund()` `:3893` / `locate_funding` `:4251`;
   `adaptor_fund` `:2432-2445`; `adaptor_build_leg_b` `:2477-2485`; v2 Signed
   rediscovery `:2938-2960`):** classify-first refusal on `Spent*` (§5.1).
8. **`take_over_swap` (`:6298`) + `rescue_from_blobs` (`:6251`):** reconstruction
   fast-forward at adoption (§5.2).
9. **`store.rs`:** add `htlc_a_height`, v2 `funding_a_height`/`funding_b_height`
   (nullable, additive); thread as `from_height` into every `find_spend_witness`
   call site (`:2697`, `:3214`, `:4068`, `:5550`).
10. **pactd `listswaps` / progress:** surface the derived phase + anomaly/stale-
    funded flags for the dock (fixes the "Locking your BTC…" ghost text); the
    per-tick snapshot scan (#165, `nostr_service.rs:303-324`) is unchanged.
11. **Phase 2 (optional):** register followed spks with the coin's sync-worker
    subscriptions as re-eval pokes; throttle Tier-L `scantxoutset` for unfunded
    followed legs to every Nth tick (it is a full UTXO-set scan per call *today*).

## 8. Performance budget

- Tier H steady state: 2 `get_history` calls per followed swap per tick — same
  round-trip class as the `listunspent` the follower already issues; batchable
  across swaps (`batch_history`). Spend-tx fetch + witness classification happens
  once per transition, then is cached under a history fingerprint (txid set +
  heights); unchanged history → zero tx fetches.
- Terminal purge removes the row (and the memo blocks re-import), so steady-state
  load is bounded by *live* foreign swaps — small by construction.
- Tier L: strictly less than today once the scantxoutset throttle lands; the
  age-out costs one MTP read.
- Driver seams add one classification at funding/adoption boundaries only — rare,
  human-scale events.

## 9. Test strategy

Unit (`reconstruct.rs`):
- v1 witness table: redeem (preimage verifies), refund (empty selector +
  nLockTime), garbage → SpentUnknown; preimage in wrong position still found.
- v2: 64B and 65B key-path; refund with byte-exact leaf; wrong leaf bytes →
  SpentUnknown; multi-item non-refund shapes.
- derive table exhaustively (every LegClass pair, both protocols); age-out clock
  edges (margin boundary, MTP vs wall clock, regtest margin 0).

e2e cells (playground, extending the #54/#163 matrix):
1. **Follow-after-completion ×4** (the field bug): v1-redeemed, v1-refunded,
   v2-keypath-redeemed, v2-tapleaf-refunded — import the lingering snapshot on a
   fresh follower → classified terminal → purged (or skipped at import); assert no
   `accepted` ghost and no wallet calls.
2. **Follower restart mid-swap:** import at funded → stop follower → complete swap
   → restart → single tick converges to terminal → purge. (The no-mode-switch
   proof.)
3. **Takeover double-fund refusal:** completed swap, lingering snapshot, import +
   `take_over_swap` → `fund()`/`adaptor_fund` refuses; assert zero `wallet_send`/
   `wallet_build_funding`.
4. **Takeover post-reveal claim (v1 + v2):** owner reveals then dies; follower
   takes over → claims leg A from the on-chain `s`/`t` (v2 from the Signed
   snapshot's adaptor sigs). Exercises §5.2 + §5.3 heights.
5. **Tier L age-out:** Core-only coin, snapshot of a completed swap → no
   classification possible → age-out purge at margin 0.
6. **Stale funded escalation:** owner dies with a leg funded, past age-out →
   record KEPT + flagged, never purged.
7. Existing #54 rescue matrix and #163/#164 failover cells must pass unchanged.

## 10. Sequenced plan

- **P0 — plumbing:** store height columns; `spk_history`/`fetch_tx` trait +
  impls + MultiBackend fan-out. Mechanical, independently landable.
- **P1 — `reconstruct.rs`** + full unit suite. Pure code, no behavior change.
- **P2 — observer:** rewrite follow evaluators on P1; purge-on-depth; import-time
  skip; age-out; dock phase surfacing. e2e cells 1/2/5/6. **Fixes the field bug.**
- **P3 — driver seams:** funding-guard refusals; takeover/rescue fast-forward;
  heights threaded into spend scans. e2e cells 3/4.
- **P4 — docs:** MULTI_MACHINE_122.md §5 update, handbook "what a follower can
  see per backend" table, wiki sync.
- **P5 (optional):** subscription pokes, Tier-L scan throttle.

P2 alone stops the ghosts; P3 closes the takeover/rescue money seams. Ship P2+P3
together in one rc — the invariants in §6 are only all-true with both.

## 11. Escalations — decisions I am NOT making unilaterally

- **E1 — v2 refund-only takeover UX:** an accept-only v2 snapshot cannot redeem
  cooperatively (no adaptor sigs). Recommend: the takeover dialog states
  "refund-only" explicitly. Needs a product yes.
- **E2 — stale funded swaps never purge** (§4.2 exception): dock shows a permanent
  escalating flag until the user acts. Recommend yes; needs UX buy-in.
- **E3 — Tier-L degraded badge:** Core-only coins get "limited follow on this
  coin — add an Electrum view for full reconstruction" in the dock/Network page.
  Recommend yes (config nudge, no protocol change).
- **E4 — quorum-confirmed-spend precondition on driver refusals** (§5.1 anti-DoS):
  accepts the residual "a colluding confirmed-quorum can stall a swap" (already
  true of every quorum read). Recommend accept.
- **E5 — Mixed terminal (A redeemed + B refunded)** is the §7.4 loss window made
  visible: silent purge or loud anomaly event? Recommend loud (distinct event +
  log), purge after depth like other terminals.
- **E6 — out of scope but adjacent:** a takeover's claimed funds sweep to the
  *dead machine's* Core node wallet on RPC-backed coins (wallet exclusivity).
  Reconstruction doesn't change this; flagging so it isn't discovered in the
  field.

---

*Grounding note: every `file:line` above was read against master @ 5555fce
(2026-07-12); the btcx citations are against the pinned rev f1e2168.*
