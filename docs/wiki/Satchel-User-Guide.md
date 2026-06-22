# Satchel User Guide

**Satchel** is the desktop app — the face of the suite. It bundles and supervises the [pactd](Running-pactd) swap engine, manages your seeds, and renders everything over loopback JSON-RPC. The window is titled **"Satchel — trustless swaps"**. All swap logic lives in the engine; Satchel only drives it.

This page orients you to the screens. For step-by-step chapters see the **Satchel handbook**: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## First run

Satchel uses a **merchant** model: one merchant = one seed = one trading identity = one data directory (the Bitcoin-Core-wallet analog), owned by pactd. Before you can trade, first-run walks three gates:

1. **Merchant** — create or import a merchant (name it, e.g. "Main").
2. **Seed** — generate a BIP39 mnemonic (reveal → verify, skipped on regtest) or import one, then choose **No passphrase (recommended)** or **Encrypt**. Satchel itself stores no seed or passphrase; an encrypted seed is unlocked per session in pactd memory only.
3. **≥2 live coins** — you must connect at least two coins with status `ok` before reaching the trading screens. See [Configuring Coins](Configuring-Coins).

## The screens

- **Corkboard** — the home screen: a two-sided order-book ladder for the selected pair (bids left, asks right), with a spread banner, an All/Mine toggle, a denomination toggle, and a board selector. Pick a price level to see its offers and **Take offer**, or **Withdraw** your own. Your own offers appear instantly with a **"posting…"** badge (italic + dimmed, a hollow ladder dot) until a relay echoes them back, then turn live; posted offers survive a restart and re-advertise on the next boot. An active-swaps dock and activity log are docked below.
- **Post an offer** — a centered card wrapping the shared offer form: pick a **Pair**, a **Sell / Buy** direction on the base coin, an **Amount** in the base coin (with live balance), and a **Price** as quote-per-base with a **denomination dropdown** (BTC/mBTC/µBTC/sat — the price is invariant to direction, and there is no ⇅ flip button). A **Swap type** appears when a pair supports more than one protocol (**Standard (HTLC)** / **Private (Taproot)** — Private is selectable on every network including mainnet), plus a **Safety timelock** preset (**Short / Medium / Long**, default Medium). A funds check blocks the confirm if you can't cover the amount plus the funding fee. A review dialog confirms before posting.
- **Create slip** — the same form, but instead of posting it produces a copyable `pactoffer1:` slip you hand to a friend over chat. See [Private Offers](Private-Offers).
- **Take a slip** — paste a `pactoffer1:` slip, review the decoded terms, and take it (the engine re-verifies the signature).
- **My slips** — your outstanding private offers with expiry countdowns and a **Cancel** action.
- **Swaps** — a read-only ledger of every swap (**In flight** + **History**), with verbatim engine narration and expandable on-chain detail. v2 rows carry a "Private (Taproot)" chip. Each swap has a **Dump logs** button (here and in the dock) that copies a secret-free diagnostics bundle for a developer. Actions live in the active-swaps dock, not here.
- **Relays** — a read-only monitor of your Nostr relay connections (status, latency, uptime; "{up} / {total} connected"). Relays are added/removed under **Settings → Network**.
- **Wallets** — **read-only** per-coin balances (one card per configured coin), each showing the node wallet its RPCs are scoped to. There is no send/receive by design — these are your own nodes' wallets, and a full send/receive wallet arrives with the nodeless build.
- **Settings** — tabs for **General** (theme + language), **Coins** (coin setup), **Network** (Nostr-relay + Corkboard URL lists + **Save & reconnect**; the read-only network-mode row is gone — the mode is launch-fixed and shown only in the top-bar badge), **Fees** (advanced, optional per-merchant fee-bump policy: max feerate, funding-bump reservation ×, redeem over-provision ×, RBF escalation step %), and **About** (version, key-storage trust note, risk disclaimer). Your side of every swap is funded automatically — there is no manual-funding mode.

## Themes & languages

Light / Dark / System themes, repainting immediately and persisted. The UI ships an English bundle today (the i18n layer is in place for more).

> **Warning** — real funds, your keys alone. The chain enforces the deal, and the protocol and implementation are reviewed. When you close Satchel with a live swap, the exit dialog lets you **keep running** so the engine can finish or refund — heed it.

## See also

- [Configuring Coins](Configuring-Coins) · [Private Offers](Private-Offers) · [Transports](Transports)
