# Post-mortem: settlement pointer lost an RBF race — swap never completes (2026-08-10)

**Status:** FIXED — spend-detection heal (`settlement-pointer-heal`), `-25` →
reconcile (`settlement-conflict`), and both e2e races (redeem + refund) landed
with the PR that carries this post-mortem. Fix-plan item 3 (remember replaced
sibling txids) was deliberately dropped: the wallet's conflict tracking plus
the spend-detection heal subsume it with no extra state. The incident itself
was field-remediated live (manual DB repoint) before the fix existed.
**Severity:** funds RECEIVED and safe the whole time; the record error-loops forever
(`RPC error -25: bad-txns-inputs-missingorspent` every tick, ~once/65 s), the swap never
reaches `completed`, and the dock shows a stuck "settlement 0/n".

## Summary

The v1 redeem nurse (`maybe_bump`) RBF-bumps an unconfirmed settlement toward
market and records the replacement as `final_txid`. When the **pre-bump version
wins the mining race** (fast chain, replacement not yet propagated to the
miner), the record points at the *losing* sibling: `tx_confirmations(final_txid)`
reads 0 forever, the arm falls through to `maybe_bump`, and the rebroadcast of
the dead version fails `-25` **as a tick error — which aborts the arm before
anything can request a reconcile**. No heal exists for settlement pointers
(`maybe_resync_funding_v1` covers funding legs only). The swap wedges in
`redeemed_b` while the redeemed coins sit confirmed in the wallet.

This is the third member of the RBF-bookkeeping family:
- #229 (fixed): *funding* pointer stale after a bump — replacement exists on chain → heal re-adopts by spk.
- 2026-08-09 post-mortem (open): funding orphaned by a *sibling's* bump — nothing on chain → re-fund.
- **This (open): *settlement* pointer adopted a replacement that then LOST the race — the winner is on chain → re-adopt by spend.**

## Incident (2026-08-10, UTC; mainnet, M-0fa9)

Swap `293a5c4cf30082f0` (initiator, 20k sat BTC → 200 BTCX, counterparty
`97cd65f8…`, created 22:00:54 on 08-09):

| fact | value |
|---|---|
| chain-B HTLC | `c33e4e43…:0` (BTCX, node wallet `Trading`) |
| winning redeem (pre-bump version) | `94bcf72f…` — mined, 104 confs at discovery, pays 199.99999845 BTCX to our address, preimage in witness |
| losing replacement in `final_txid` | `5a9c0c33…` — `gettransaction` confirmations **-104**, `walletconflicts: [94bcf72f…]` |
| symptom | `-25 bad-txns-inputs-missingorspent` warn-loop from ~00:00, swapprogress "settlement 0/5 @ 2 sat/vB" |

The node wallet had the full answer all along: negative confirmations plus
`walletconflicts` naming the winner.

## Why nothing healed

- `(Initiator, RedeemedB)` arm (engine.rs ~8126): `confs == 0` → `maybe_bump` →
  rebroadcast dead hex → `-25` propagates as the tick result. The error path
  requests nothing; reconcile is never re-armed.
- Reconcile would have terminalized it (leg-B `Spent(kind Redeem)` by our own
  tx → `v1_settled_terminal`) — but only runs when requested.
- The redeem-side never had a #229-style heal; `maybe_resync_funding_v1` is
  funding-legs-only by design ("spent or invisible — not ours to judge here").

## Remediation used (repoint recipe)

1. Prove the winner: `gettransaction <final_txid>` → negative confs +
   `walletconflicts`; `gettransaction <conflict>` (verbose) → deep, spends the
   same HTLC outpoint, `category: receive` to our address, preimage in witness.
2. Backup `pact.sqlite` (sqlite backup API), rewrite the record JSON:
   `final_txid` and `final_tx_hex` → the winner.
3. `pact-cli tick` → the arm's own logic sees confs ≥ n_b → `completed` +
   tombstone. (Observed live: immediate `completed` event.)

## Fix plan

1. **Spend-detection heal for settlement states** — in the `RedeemedB` /
   participant-`Completed` / `Refunded` arms, when `final_txid` reads 0 confs,
   check whether the HTLC outpoint is spent by a *different* tx
   (`find_spend_witness` / wallet `walletconflicts`); if that spend is ours
   (pays `spend_spk`, or reveals our preimage), adopt it as `final_*` instead
   of bumping. Mirrors `maybe_resync_funding_v1`, but for spends.
2. **`-25` must trigger reconciliation, not error-loop** — treat
   `bad-txns-inputs-missingorspent` from a settlement rebroadcast as "the
   outpoint is spent — find out by whom": request reconcile and return an
   event, not an error. The reconcile matrix already terminalizes both
   directions correctly.
3. **Bump bookkeeping keeps sibling txids** — record replaced settlement
   versions (or rely on wallet conflicts) so confirmation checks consider
   every version, not only the newest.
4. **e2e**: redeem RBF race — broadcast redeem v1, bump to v2, mine v1 behind
   the engine's back, assert the swap still completes (and the same for a
   refund).

## Notes

- v2 exposure: the v2 redeem is CPFP-bumped (no replacement ⇒ no race of this
  shape), but audit the v2 refund path for the same pattern.
- Interim: the session watchdog (`wedge_watchdog.py`) now also detects
  settlement states whose `final_txid` is conflicted on the Core-backed coin
  and auto-repoints to a proven winner (Electrum-backed coins: alert only).
