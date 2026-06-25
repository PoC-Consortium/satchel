# Design — active-swap observability (confirmation progress + live action)

**Status:** Proposal for review (companion to [`post_mortem.md`](post_mortem.md) and
[`fee-bump-design.md`](fee-bump-design.md)).
**Author:** drafted by Claude (Opus 4.8) with Johnny, 2026-06-25.
**Scope:** surface *quantitative* progress for an in-flight swap in Satchel — what's
confirming, how deep, and what the daemon is doing to the tx right now. No protocol or
swap-logic change; status plumbing only.

> Why: a trader watching an active swap today sees the **state** chip + an honest
> *qualitative* `narrate()` line (e.g. *"waiting for it to confirm"*), the txids, and a Dump
> button. They do **not** see confirmation depth (X/N), an ETA, or any fee/bump activity. The
> post-mortem storm was invisible in Satchel — it was only caught in Bitcoin-QT — precisely
> because the app never shows *"the daemon is bumping this tx right now, at N sat/vB."* The
> data already exists at tick time; it's just discarded.

---

## 1. The gap

| Question a trader asks | Answered today? |
|---|---|
| What phase is the swap in? | ✅ state chip + `narrate()` |
| Who is exposed / what's my refund deadline? | ✅ `narrate()` (t1/t2) |
| Are we waiting on a confirmation? | ◐ qualitatively only ("as soon as it is safely confirmed") |
| **How many confirmations so far, of how many needed?** | ❌ |
| **Which tx is being watched right now? Rough ETA?** | ❌ |
| **Is the daemon bumping the fee? At what rate?** | ❌ (invisible — see post-mortem) |
| Did something go wrong (reorg/stall)? | ◐ emitted as a tick event → only in `pactd.log` / Dump, not live |

The required depths (`n_a`/`n_b`) are in the swap record but unsurfaced; the *current* depth is
computed inside a tick (`tx_confirmations`) and thrown away; tick events (`fee-bump`,
`waiting on confirmations`, `reorg-alert`, `auto-redeem`) go to the log only.

---

## 2. What to surface — a per-swap progress snapshot

A small status object, one per active swap, attached to the `getswap`/`listswaps` response the
UI already polls:

```text
SwapProgress {
  watching:       enum   // which tx we're tracking now: ours_funding | their_funding | settlement
  watching_txid:  string // the live tx (tracks fee-bumps — equals the current final/funding txid)
  confs:          u32    // current confirmations of `watching_txid` (0 = in mempool / not yet seen)
  needed:         u32    // required depth for this leg (n_a or n_b)
  feerate_sat_vb: u32?   // current feerate of the live settlement/funding tx (None if n/a)
  last_event:     { action: string, detail: string, at: unix }?   // most recent TickEvent
  updated_at:     unix   // when this snapshot was taken (last tick that touched the swap)
}
```

Notes:
- `watching` / `needed` derive from `(role, state)` — the same mapping `narrate()` uses.
- `confs` is the number the tick *already* fetched; we just keep it instead of discarding it.
- `feerate_sat_vb` makes the post-mortem's fee activity visible; combined with `last_event`
  (`fee-bump → 159 sat/vB at block H`) the user finally sees bumps in-app.
- `last_event` is the existing `TickEvent { action, detail }`, plus a timestamp.

---

## 3. Where it comes from — piggyback on the tick (near-zero cost)

The scheduler tick **already** calls `tx_confirmations` / `get_txout` and **already** produces a
`TickEvent` for each active swap. The only change is to *retain* that result:

- Keep an **in-memory** `HashMap<swap_id, SwapProgress>` on the engine, written at the end of
  each tick's per-swap pass, read when assembling `getswap`/`listswaps`.
- **No extra node round-trips** (reuses the tick's existing chain queries) and **no new UI
  polling loop** (folds into the existing `listswaps` poll).
- **In-memory, not persisted** — it's ephemeral status, not ledger truth. Avoids writing the
  store every 30s for every active swap. After a restart it's empty until the next tick
  repopulates it (~one tick); the state chip + `narrate()` still render meanwhile.
- Secret-free by construction (confs, txids, feerate, event text — no preimage/keys), so it's
  safe to send to the UI and matches the existing Dump-bundle redaction rule.

> Alternative considered: compute `confs` on every `listswaps` read instead of caching from the
> tick. Rejected — it adds a node call per poll per swap (UI polls faster than the tick) and
> double-counts work the tick already does.

---

## 4. UI rendering (Satchel Swaps screen)

Additive only — `narrate()` stays **verbatim** (load-bearing UX), with a compact progress line
beneath it in the active-swap row:

- **Confirming:** `Redeem confirming · 2/6 · 159 sat/vB` (a small determinate progress bar on
  `confs/needed`).
- **Waiting on counterparty:** `Waiting for their BTC lock · 0/6`.
- **Just bumped:** if `last_event.action == "fee-bump"` within the last tick →
  `Fee-bumped to 159 sat/vB` (transient, accent colour).
- **Trouble:** `reorg-alert` / stall events → a warning-coloured line, lifting what is today
  log-only into view.
- Rough ETA (optional): `~(needed − confs) × chain_block_time` rendered as "≈40 min left".

The expandable audit (txids, Dump) is unchanged; this only enriches the always-visible row.

---

## 5. Principles / constraints

- **No swap-logic change.** Status plumbing only; the engine's decisions are untouched.
- **`narrate()` stays verbatim** — progress is a *separate* additive line, never a rewrite.
- **Reuse the tick's work** — no new node calls, no new polling loop, no store churn.
- **Secret-free** — same redaction rule as the Dump bundle.
- **Honest about staleness** — `updated_at` lets the UI grey out a snapshot older than ~2 ticks
  (e.g. daemon detached) rather than imply live data.

---

## 6. Open questions

- [ ] **`watching` granularity** — three buckets (ours/their funding, settlement) enough, or
  also distinguish "in mempool, unconfirmed" vs "0 confs but seen"?
- [ ] **ETA** — show it (needs a per-chain block-time estimate) or omit as guesswork?
- [ ] **Event history** — surface only the *latest* tick event, or a short rolling list (last
  3) in the expandable audit?
- [ ] **Corkboard ActiveSwaps panel** — mirror the same progress line there, or keep it on the
  Swaps screen only?
- [ ] Does `listswaps` or only `getswap` carry `SwapProgress`? (Prefer `listswaps` so the row
  shows it without expanding.)
