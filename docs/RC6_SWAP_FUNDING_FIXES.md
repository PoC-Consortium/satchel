# rc6 — Swap funding-failure hardening

Origin: a live mainnet v1 swap (`22e536c4653afc92`, 2026‑06‑29) stranded at `funded_a`
because the taker's BTCX Core wallet was **encrypted + locked** → the auto-fund of leg B
threw `RPC -13` ("Please enter the wallet passphrase with walletpassphrase first").
The take-time *balance* check passed (`getbalance` works on a locked wallet); only the
*sign* failed. The record was already persisted as `FundedA`, and there was no tick arm
to retry the fund — so the swap was stranded until manually recovered
(`walletpassphrase` unlock + `pact-cli call fund <id>`).

A v1+v2 audit (both roles, every broadcast step) was run to scope the real fix.

## Audit conclusions

- The **persist-before-broadcast + no-retry** stranding defect exists at **exactly one
  spot**: the v1 taker's leg‑B fund. Everything else (v1 maker fund‑A, all redeems/refunds;
  **all** v2 adaptor steps) is **broadcast-before-persist** and tick-retryable.
- **V2 is already robust**: every chain-touching step broadcasts first, then persists; a
  funding failure leaves an honest, recoverable `Accepted` (`funding=None`), resumable via
  a later relay message or `adaptor_fund` RPC. No correctness fix needed in v2.
- The locked-wallet `-13` is **fund-only**: redeem/refund broadcast a *seed-signed* tx via
  `sendrawtransaction` (no wallet unlock needed); only `fund` uses `sendtoaddress`
  (`signrawtransactionwithwallet`), which is what errors `-13`.
- **Commit rule** for any new retry: persist the advanced state **right after the broadcast
  is accepted** (mempool/txid) — never before, never only-after-confirmation (a restart
  between broadcast and confirmation must not re-broadcast → double-fund).

---

## Step 1 — CORE (the wallet-lock failure mode)

### #8 — Gate take/post on a locked wallet (prevention, top priority)
`ensure_can_fund` (engine.rs:724) checks balance (reads fine on a locked wallet) but not
lock state. Add a `getwalletinfo.unlocked_until == 0` check and reject before committing.
Applies to the taker's get-leg at **take** and the maker's give-leg at **post**
(`ensure_can_fund_new_offer`, engine.rs:804).

### #2 — Uniform funding self-retry (all funding steps, both roles, both versions)
Make every fund step tick-retryable so a locked/transiently-failed wallet self-heals next
cycle instead of stranding or aborting (user: "same retry mechanism here as well"):
- **v1 taker leg-B** — `(Participant, FundedA)` retry arm (covers the tick-`Accepted` edge
  *and* the relay-`funded` entry at engine.rs:5100). **Also fixes the one real stranding bug.**
- **v1 maker leg-A** — `(Initiator, Accepted)` retry arm (today a locked maker just
  time-out-aborts; with this a brief lock recovers).
- **v2 funding (both legs)** — a tick arm re-invoking `adaptor_fund` for pre-`Signed`
  records whose own leg is unfunded. **DEFERRED** (see the note in `adaptor_tick_one`):
  v2 funding fails *closed* into an honest, recoverable `Accepted` (`funding=None`),
  resumable by a relay re-drive or a manual `adaptor_fund` RPC — a liveness gap, not a
  stranding bug. A correct tick retry needs the counterparty identity on
  `AdaptorSwapRecord` (a schema add, not present today) to relay `funding_ready`, plus a
  locate-first idempotency guard on the Taproot funding (today's is pointer-based).

> **Step 1 status (in progress):** #8 done; #2 v1 done (idempotency guard in `fund()` —
> locate-first; `(Participant, FundedA)` + `(Initiator, Accepted)` retry arms;
> `fund_deadline_passed` helper). v2 retry deferred (above). 97 libswap unit tests green.
> **Testing:** the locked-wallet self-heal is an e2e scenario (encrypt the regtest wallet,
> take, lock, assert stranded→retrying, unlock, assert self-heal) — the unit harness uses
> real Core backends against fake URLs, so it can't mock `getwalletinfo.unlocked_until`.
> Follow-up: add that regtest e2e.

Requirements:
1. **Idempotent** — each retry must `locate_funding` first and re-broadcast **only** if the
   leg isn't already on chain (else a retry after a silent-success broadcast = double-fund,
   real loss). v2 already guards this (engine.rs:1662‑66); **v1 `fund()` needs the same guard**.
2. **Composes** with the `(_, Created|Accepted)` timeout-abort: retry within the pre-funding
   window, abort after.
3. Persist the advanced state only after the broadcast is accepted (the commit rule).

### #1 — Persist-after-broadcast — VERIFIED ALREADY SATISFIED (no work)
Audit confirmed the codebase persists-after-broadcast everywhere except the one
structurally-required v1 `FundedA` write, which is closed by #2's retry arm.

---

## Step 2 — OBSERVABILITY / honesty

### #6 — Maker records leg-B pointer on FIRST chain detection (not at `n_b`)
`(Initiator, FundedA)` only writes `htlc_b_txid` at the `n_b`-conf flip (engine.rs:3564),
while the funded **message** writes it early (engine.rs:2504). So a chain-discovered
(no-relay) leg B shows `awaiting_lock` with no burial count until it jumps to `funded_b`.
Fix: when `locate_funding('b')` first finds the HTLC (conf ≥1), persist
`htlc_b_txid/vout/height` immediately (keep state `funded_a` until `n_b`) → progress shows
`their_lock confs/n_b` from conf 1, identical to the relay path. Safe: `locate_funding`
already verifies script+amount; redeem still gates on `n_b` + §7.4. (Also closes the #10
invariant gap.)

### #3 — Fix the FAILED ACTOR's own self-view
The participant stuck at `FundedA` renders `awaiting_claim` ("awaiting their receipt"),
implying it's the maker's turn when really the taker's own fund failed. Show "funding leg B
failed — wallet locked, retrying / unlock to fund". (The cpty/maker's view was already
honest.)

### #4 — Surface the locked wallet + manual fund fallback
The `-13` is buried in `dumpswap`; the UI shows only Cancel/Refund/Dump-logs (no fund
button). Show "wallet locked — unlock to fund" and a manual retry.

### #5 — Manual `fund` RPC should relay the `funded` envelope
The auto-fund path does `relay_send_all`; the RPC (main.rs:780) just returns it. So a
hand-recovered swap doesn't notify the maker.

### #9 — Maker awaiting-count ticks in the AWAITED chain (BTCX), not BTC
`(Initiator, FundedA)` awaiting count is `confs_a - n_a` (BTC blocks; engine.rs:3162‑3166),
anchored to leg A for restart-survival. Count in `chain_b` (BTCX) instead. For
BTCX + restart-survival, persist the `chain_b` tip when leg A buries and count blocks since.

### #10 — INVARIANT: swap completes via chain-watch alone after the handshake
Verified mostly true (2026‑06‑29): maker discovers leg B via `scantxoutset`; taker extracts
the preimage via `find_spend_witness`+`extract_preimage` (engine.rs:3641) from chain, not the
relay `redeemed` msg; both redeems self-driven. Today's relay-bypassed manual-fund swap
completed end-to-end on chain. Relay `funded`/`redeemed` are pure accelerators. The last gap
(maker leg-B observability) is closed by #6. Preserve this for any future message-driven shortcut.

---

## Step 3 — BONUS (CI)

### release.yml matrix race
The release workflow's matrix can create **duplicate / partial draft releases** (bit the
rc5 re-release: two drafts, one with 5 assets, one with 3; macOS "Tidy asset names" 404'd).
Fix: pre-create the draft (or serialize the create step) so all matrix legs upload to one draft.

---

## Dropped / WONTFIX

- **#7 take→fund balance reservation** — DROPPED. Over-provisioning (topping up the funding
  wallet between take and fund) is a desired workflow; a hold would block it. Take-time
  balance check stays as-is.
- **`nostr_revoked` tombstone pruning** — WONTFIX. Revokes are bounded; the full tombstone
  trail is kept for audit.

These are funding-failure-handling + observability gaps, not crypto/safety; the chain-watch +
`scantxoutset` discovery itself is sound.
