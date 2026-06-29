# Satchel User Guide

**Satchel** is the desktop app — the face of the suite. It bundles and supervises the [pactd](Running-pactd) swap engine, manages your seeds, and renders everything over loopback JSON-RPC. The window is titled **"Satchel — trustless swaps"**. All swap logic lives in the engine; Satchel only drives it.

This page orients you to the screens. For step-by-step chapters see the **Satchel handbook**: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## First run

Satchel uses a **merchant** model: one merchant = one seed = one trading identity = one data directory (the Bitcoin-Core-wallet analog), owned by pactd. Before you can trade, first-run walks three gates:

1. **Merchant** — create or import a merchant (name it, e.g. "Main"). You can later rename the active merchant inline from the sidebar — only the label changes, so it's safe even mid-swap.
2. **Seed** — generate a BIP39 mnemonic (reveal → verify, skipped on regtest) or import one, then choose **No passphrase (recommended)** or **Encrypt**. Importing uses a numbered word-grid with per-word BIP39 autocomplete, invalid-word highlighting, paste support, and a live status line that validates the checksum before you can continue. Satchel itself stores no seed or passphrase; an encrypted seed is unlocked per session in pactd memory only.
3. **≥2 live coins** — you must connect at least two coins with status `ok` before reaching the trading screens. See [Configuring Coins](Configuring-Coins). To skip this and just look around, pick **Browse in watch-only mode** (see below).

> **Watch-only mode** — browse the whole board and withdraw your own offers with **no coins configured**. A **"Watch only"** badge sits in the header, the post/take screens show a watch-only notice, and the Take button is disabled. Trading is blocked until you connect two live coins; toggle the mode under **Settings → Mode** (it reboots the engine).

## The screens

- **Corkboard** — the home screen: a two-sided order-book ladder for the selected pair (bids left, asks right), with a spread banner, an All/Mine toggle, a denomination toggle, a **Hide blocked offers** toggle, and a board selector. Pick a price level to see its offers and **Take offer**, or **Withdraw** your own. Your own offers appear instantly with a **"posting…"** badge (italic + dimmed, a hollow ladder dot) until a relay echoes them back, then turn live; posted offers survive a restart and re-advertise on the next boot. An active-swaps dock and activity log are docked below. Click any counterparty's identicon (here or anywhere) to add or edit a local **contact** (see below); taking from a contact you've marked **Blocked** shows a warning and an extra confirm.
- **Post an offer** — a centered card wrapping the shared offer form: pick a **Pair**, a **Sell / Buy** direction on the base coin, an **Amount** in the base coin (with live balance), and a **Price** as quote-per-base with a **denomination dropdown** (BTC/mBTC/µBTC/sat — the price is invariant to direction, and there is no ⇅ flip button). A **Swap type** appears when a pair supports more than one protocol (**Standard (HTLC)** / **Private (Taproot)** — Private is selectable on every network including mainnet), plus a **Safety timelock** preset (**Short / Medium / Long**, default Medium). A funds check blocks the confirm if you can't cover the amount plus the funding fee. A review dialog confirms before posting.
- **Create slip** — the same form, but instead of posting it produces a copyable `pactoffer1:` slip you hand to a friend over chat. See [Private Offers](Private-Offers).
- **Take a slip** — paste a `pactoffer1:` slip, review the decoded terms, and take it (the engine re-verifies the signature).
- **My slips** — your outstanding private offers with expiry countdowns and a **Cancel** action.
- **Swaps** — a read-only ledger of every swap (**In flight** + **History**), with verbatim engine narration and expandable on-chain detail. every row carries a protocol chip — muted **Standard (HTLC)** for v1, accented **Private (Taproot)** for v2 — on the page and in the active-swaps dock. Each swap has a **Dump logs** button (here and in the dock) that copies a secret-free diagnostics bundle for a developer. Actions live in the active-swaps dock, not here — and the dock's only manual button is **Cancel** (available until you've funded), since funding, redeeming, and refunding all happen automatically.
- **Relays** — a read-only monitor of your Nostr relay connections (status, latency, uptime; "{up} / {total} connected"). Relays are added/removed under **Settings → Network**.
- **Wallets** — **read-only** per-coin balances (one card per configured coin), each showing the node wallet its RPCs are scoped to. There is no send/receive by design — these are your own nodes' wallets, and a full send/receive wallet arrives with the nodeless build.
- **Contacts** — your private, **local-only** address book: a searchable table (with an **All / Trusted / Blocked** filter) mapping a counterparty's identity to a **nickname**, an optional **note**, and a **standing** (Trusted / Neutral / Blocked). See [Local contacts](#local-contacts) below.
- **Settings** — tabs for **General** (theme + language), **Coins** (coin setup), **Network** (Nostr-relay + Corkboard URL lists + **Save & reconnect**; the read-only network-mode row is gone — the mode is launch-fixed and shown only in the top-bar badge), **Fees** (advanced, optional per-merchant fee-bump policy: max feerate, funding-bump reservation ×, redeem over-provision ×; the old manual RBF-step knob is gone — fee-bumping is now automatic market-tracking), and **About** (version, key-storage trust note, risk disclaimer). Your side of every swap is funded automatically — there is no manual-funding mode.

## Local contacts

A **contact** is a private label you attach to a counterparty's identity — the identicon plus its fingerprint (the BIP340 public key). Each contact carries a **nickname**, an optional **note**, and a **standing**: **Trusted**, **Neutral**, or **Blocked**. Add or edit one by clicking a counterparty's identicon anywhere it appears — Corkboard, Swaps, the active-swaps dock, or the take-confirm dialog — or manage them all from the **Contacts** tab.

Contacts are **local only**. They live on this device and are never shared, published, signed, or sent to a relay — the engine never sees them. The nickname is shown **alongside** the identicon and fingerprint and never replaces them: the identicon + fingerprint remain the real identity, which is what defends against impersonation (a stranger can copy a nickname but not the key). The standing is **soft and personal** — the protocol itself derives trust only from atomicity, so the chain enforces the deal regardless of how you've labelled someone.

> **Blocked never stops a trade.** It only changes your local view — hiding those offers when **Hide blocked offers** is on, and adding a warning plus an extra confirm if you take from them. The swap still works exactly the same on-chain.

## Wallet lock & funding

If a coin's node wallet is **encrypted and locked**, posting or taking an offer for that coin is refused **up front**, with a prompt to unlock it via `walletpassphrase` and keep it unlocked until the swap completes. A locked wallet can read its balance but **cannot sign the funding transaction**, so the swap could not be funded otherwise.

While a swap's own funding is pending or retrying, its row shows **"Locking your &lt;coin&gt; — unlock wallet if stalled"** with a **"+N blocks"** liveness count. A count that keeps growing signals a stall — usually a wallet that locked mid-swap. If a v1 fund fails to broadcast (for example because the wallet locked), the engine **retries it automatically on every tick**, so simply unlocking the wallet lets the swap **self-heal** — there is no manual step to perform.

## Themes & languages

Light / Dark / System themes, repainting immediately and persisted. The UI ships **26 languages** — switch from the globe-icon picker in the header (always visible), and during first-run onboarding the same picker floats in the dialog's top-right corner so you can change language before finishing setup.

> **Warning** — real funds, your keys alone. The chain enforces the deal, and the protocol and implementation are reviewed. When you close Satchel with a live swap, the exit dialog lets you **keep running** so the engine can finish or refund — heed it.

## See also

- [Configuring Coins](Configuring-Coins) · [Private Offers](Private-Offers) · [Transports](Transports)
