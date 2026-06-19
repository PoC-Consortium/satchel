# Satchel UI

The Satchel desktop app (Tauri + React + Vite + TypeScript + MUI v6) is the
graphical face of Pact. It is a **thin client** of the local pactd daemon: it
renders pactd's state and calls its RPCs, but holds **no swap logic and no
secrets** of its own. The look and feel follows the phoenix-pocx wallet — a
collapsible left sidenav with drawer-header branding, a content-area toolbar
with status indicators, and a Settings page.

See [SATCHEL.md](SATCHEL.md) for the app's architecture (the Tauri bridge,
merchant lifecycle, node-manager modes), [SATCHEL_BACKEND.md](SATCHEL_BACKEND.md)
for the pactd data contracts the UI codes against,
[PRIVATE_OFFERS.md](PRIVATE_OFFERS.md) for the off-market slip flow,
[V2_ADAPTOR_SWAPS.md](V2_ADAPTOR_SWAPS.md) for the Taproot/adaptor protocol,
[ARCHITECTURE.md](ARCHITECTURE.md) and [TRADING_ROADMAP.md](TRADING_ROADMAP.md)
for the overall design and plan, and the root [README.md](../README.md) for the
component map. The protocol spec lives under [../spec/](../spec/).

## Constraints the UI is built around

These are load-bearing and must not be "improved" away:

- **No swap logic in the UI, ever.** Every action that moves funds (post, take,
  fund, redeem, refund) is a pactd RPC; the UI only renders the result.
- **The Corkboard is a noticeboard, not an exchange.** It shows offers — even in
  an order-book *view* — but never matches, executes, prioritises, or auto-fills.
  Humans pick offers. This is load-bearing for the regulatory position.
- **Secrets live in pactd; Satchel persists nothing sensitive.** No seed, no
  passphrase, no OS keystore. A passphrase is passed through to pactd's `unlock`
  and held transiently in pactd memory for the session. The mnemonic is shown
  once and is the user's responsibility to back up.
- **Non-mainnet must be visually unmistakable** — a loud network stamp signals
  "not real funds".
- **pactd narration is shown verbatim** — the daemon's plain-language swap story
  is rendered as-is, never rewritten by the UI.

## Layout & navigation

The window is a phoenix-style shell: a collapsible left sidenav, a sticky
content-area toolbar (Header) at the top of the main column, and two fixed
bottom docks (active-swaps strip + activity log) that stay in view while the
page content scrolls.

### Sidenav

The left sidenav (`Sidebar.tsx`) collapses to zero width on desktop (a width
rail that lets content reflow) and becomes an overlay drawer on narrow windows;
its open/collapsed state persists per-install. Top to bottom:

- **Drawer header** — Satchel logo, the Montserrat wordmark, and the app version
  beneath it.
- **Active-merchant area** — the active merchant's identicon, label, and short
  identity; clicking it opens the Merchant Manager.
- **Primary nav**, in two venue groups plus two top-level items:
  - **PUBLIC** — **Corkboard** (browse the ladder, take public offers) and
    **Post an offer** (list a signed offer on the board).
  - **PRIVATE** — **Create slip**, **Take a slip**, and **My slips** (bilateral
    off-market offers; the two actions first, the review-and-cancel list last).
  - **Swaps** (the ledger) and **Wallets** (per-coin balances), top-level.
- **Footer** — **Settings** (Coins, theme, language, network, about all live
  inside it).

Routes: `board`, `post-offer`, `private-create`, `private-receive`,
`private-slips`, `swaps`, `wallets`, `settings`. The app opens on the Corkboard.

### Header (content-area toolbar)

The toolbar (`Header.tsx`) carries context and actions, not branding:

- A **menu toggle** appears on the left when the sidenav is collapsed.
- **Status indicators** (`StatusIndicators.tsx`) — a row of small monochrome
  icons that grey out when inactive and colour when active:
  - **Pact connection** — green when pactd is reachable, red when not.
  - **Per-coin health** — one glyph per configured coin, coloured by that coin's
    node/backend status, tooltip showing name and tip height (from `listcoins`).
  - **Live swaps** — lights up with a count badge when swaps are in flight;
    clicking it jumps to the Corkboard (where the active-swaps dock lives).
- A **network stamp** is centred in the bar (see below).
- An **active-merchant chip** (identicon + label) opens a phoenix-style dropdown:
  "Manage Merchants…" first, then the list of merchants to switch to, each
  showing its lock state. Switching is gated by pactd if the current merchant has
  a live swap.
- A **Settings** gear and a **language** selector (globe) sit on the right.

### Network stamp

`NetworkStamp.tsx` renders a "worn stamp" badge (coloured per network:
regtest/testnet/signet) as an unmistakable marker that funds are not real. On
**mainnet it renders nothing** — the absence of a stamp is the signal that funds
are real. It appears in the header, the wizard, and Settings → Network/Coins.

### Bottom docks

Two regions are pinned below the scrolling content so they never scroll away:

- **Active-swaps dock** (`ActiveSwaps.tsx`) — shown on trading views (the
  Corkboard today). It renders nothing when no swap is in flight; otherwise each
  in-flight swap appears with its state, amounts, role, the verbatim narration,
  the refund time, and the relevant action buttons (Fund / Redeem / Cancel /
  Refund — each a pactd RPC).
- **Activity log** (`LogPanel.tsx`) — a docked, collapsible footer with its own
  scroll, newest line on top, monospace. A quiet running record of what
  Satchel/pactd just did. Load-bearing for following swap progress, so it stays
  visible regardless of page scroll.

## Corkboard

`CorkboardScreen.tsx` is the primary trading view: a **two-sided order-book
ladder** rendered purely for reading the board. pactd never matches or
prioritises — a negative ("crossed") spread is even possible because nothing
executes.

Control row:

- **Trading-pair select** (left) — no "all pairs"; the first supported pair is
  the default. Offers are filtered to supported pairs (from `listpairs`); offers
  on unsupported pairs are counted with a link to Settings → Coins.
- **All / Mine toggle** — "Mine" filters to your own posted offers, which carry a
  "your offer" chip and a Withdraw button.
- **Denomination toggle** — switches the quote-coin unit (the choice persists).
- **Noticeboard select** (right) — switches between configured boards when more
  than one is set up (`boardlistoffers` honours the selected board).

The ladder shows **bids and asks** as exact-rate price levels, each side
rate-sorted toward the spread, with a depth bar, an offer count per level, a
"mine here" dot, and a cap of 8 levels per side ("Show N more" reveals the
rest). A spread banner relates the top bid to the top ask (or flags a crossed
book). Selecting a level opens a **detail pane** below the ladder listing the
individual offers at that price.

Each offer row shows the counterparty (identicon + short fingerprint, via
`CounterpartyTag.tsx`), the offer state, a "Private (Taproot)" chip for
`pact-htlc-v2` adaptor offers, the give/receive amounts, the safety-timelock
timing, a freshness/age cue, and a **Take** or **Withdraw** action.

**Taking an offer keeps you on the Corkboard.** A shared take-confirmation card
(`useTakeConfirm.tsx`) shows the counterparty, you-give/you-receive amounts, the
timelock timing, a "maker funds first" note, and the network-cost preview; on
confirm it calls `boardtake`, and the taken offer surfaces as an active swap in
the dock below. The board auto-refreshes (~12s) so stale/withdrawn offers drop.

**Offer state** is reflected, not executed: open offers have no badge;
"taken-by-us" (correlated to our own swaps), "revoked", and "expired" are
badged. Board-side tracking of takes by *other* parties is a deliberate non-goal
(blind relay), so an offer can stay listed after it is effectively gone — hence
the freshness cues. Nothing is locked until funding, so hitting a stale offer
wastes time, not funds.

## Swaps

`SwapsScreen.tsx` is the comprehensive book-keeping ledger: **active/in-flight
swaps on top** (walking their live states), then **terminal history** below,
each section newest-first by `created_at`. This screen only *renders* pactd's
swap list — it carries no action buttons (those live in the Corkboard's
active-swaps dock).

Each row shows the swap id, role, state chip, amounts, timestamp, and a
truncated final txid; a "Private (Taproot)" chip marks adaptor swaps. The row
expands to show pactd's **verbatim narration** plus an on-chain **audit trail**:
per leg, who locked what, the funding txids, and *your* settlement (redeem or
refund) txid, each copyable. The counterparty's settlement tx and the swap
secret are never shown.

## Wallets

`WalletScreen.tsx` is **read-only**: one card per configured coin (`listcoins`
filtered to configured) with its glyph, display name, symbol, and balance
(`getbalance`). There is deliberately **no Send and no Receive** here — the
balance *is* the node's own core wallet, so spending would duplicate the node's
wallet UI and swap txs already surface on the Swaps page. A banner reinforces
the **hot-seed** framing, nudging users to sweep sizeable balances to their own
cold/core wallet. A full send/receive/activity wallet (via `getnewaddress` /
`sendtoaddress`) lands only with the **nodeless** build, where Satchel carries
its own `bdk` + Electrum wallet rather than fronting a node.

## Settings

`SettingsScreen.tsx` splits configuration into MUI tabs:

- **General** — theme (light / dark / system, via MUI's color-scheme support;
  the choice persists per-install) and language (English ships; the i18n layer
  is built so further languages are just new bundles).
- **Coins** — embeds the coin-setup screen (below).
- **Network** — shows the active network (as a stamp on non-mainnet) and lets
  the user configure the noticeboard URL(s) via the Board Config dialog.
- **About** — the app version, an update-check line, and the trust-model note
  (where secrets live, encrypted-at-rest option, "Satchel stores nothing",
  hot-seed warning).

> **TODO:** update check — the version indicator and "up to date" line are a
> static placeholder; live GitHub-releases polling and an update badge land
> later.

### Coins

`CoinsScreen.tsx` (reached under Settings → Coins) shows one card per registry
coin with its glyph, display name, symbol, connection status (tip height when
connected), capability chips (CLTV / SegWit / Taproot), and the configured
backend URL. **Set up / Edit connection** opens the coin-setup dialog
(validate-then-save against the active network); **Remove** disconnects a coin.
A derived-pairs list below shows each pair's buildability and supported
protocols (HTLC and "Private"/adaptor). The screen states the active network and
validates each coin's genesis against it, rejecting a mismatch.

`CoinGlyph` renders the real per-coin logo asset (`src/assets/coins/`, keyed by
coin id: `btc.svg` = the canonical orange Bitcoin mark; `btcx.svg` = the
official Bitcoin PoCX coin mark) and falls back to the generated text glyph
(₿ / ◈ / first letter of the symbol) for any coin id without a bundled asset —
so future coins still render until their logo ships. The header's per-coin
health indicator keeps the text glyph deliberately: its colour encodes
connection status (green / red / disabled), which a full-colour logo would
mask.

## First run, wizard & merchants

A **merchant** = one pactd seed = one trading identity = one data dir. pactd owns
the merchant registry; Satchel switches the active merchant via pactd RPCs.

The first-run **wizard** (`Wizard.tsx`) walks: **connection mode**
(run a managed pactd, or connect to an external one) → **load pact core** (shows
the client's single network) → **name the merchant** → **provision the seed**.

> **TODO:** wizard connection/load steps are a presentation shell — managed mode
> is the default and the dedicated spawn/health flow and coin-setup-by-template
> step are not yet wired into onboarding.

**Seed provisioning** (`SeedForm.tsx` / `SeedProvision.tsx`) offers create-new or
import, with an encrypt (passphrase) vs no-passphrase choice. A freshly created
mnemonic is shown **once**, in a grid, behind an explicit "I've backed this up"
acknowledgement. An encrypted merchant comes up locked and is unlocked per
session via the Unlock gate.

The **Merchant Manager** (`MerchantManager.tsx`) is a phoenix-style selector:
click a merchant row to select, then act via buttons — Create new, Import, Close,
Load. Each row shows the identicon, label, a single "loaded" badge for the active
merchant, a lock chip for encrypted-and-locked merchants, and the raw data-dir id
demoted to a small copyable detail (the label is the identifier). One merchant is
loaded at a time; "Load" switches the active merchant, and pactd gates the switch
if the current merchant has a live swap.

## Private offers (off-market slips)

The PRIVATE nav group is the bilateral, never-listed counterpart to the public
board (see [PRIVATE_OFFERS.md](PRIVATE_OFFERS.md)):

- **Create slip** (`PrivateCreateScreen.tsx`) — the shared offer form →
  `makeprivateoffer` produces a `pactoffer1:` slip shown in a copy box with an
  "expires ~24h" explainer and links to *My slips* / *Make another*. The slip is
  never posted to a board; the maker hands it to a friend over their own chat.
- **Take a slip** (`PrivateReceiveScreen.tsx`) — paste a `pactoffer1:` slip; it
  is decoded locally for display, run through the **same** take-confirmation card
  as a board take, then submitted via `takeoffer`. pactd is the authority: it
  re-decodes, verifies the BIP340 signature, and checks expiry/pair support. From
  there the swap is indistinguishable from a board take and appears in Swaps.
- **My slips** (`PrivateSlipsScreen.tsx`) — the maker's outstanding slips
  (`listprivateoffers`, polled), each with its pair, amounts, expiry countdown,
  and a **Cancel** action (`cancelprivateoffer`).

## Shared offer form & fee preview

Both **Post an offer** and **Create slip** use the shared, destination-agnostic
`OfferForm.tsx`: give/get coin selectors with **live wallet balances** per coin,
amount fields, and a **Short / Medium / Long** timelock preset (Long is the
default, with safe 2:1 T1:T2 gaps) instead of raw timelock hours. It enforces
give ≠ want and includes the "Corkboard charges nothing; nothing is locked by
posting" note.

`FeePreview.tsx` surfaces **network (miner) fees** — never platform fees, which
are always zero — on the take-confirmation card: the shape of a swap's cost (the
two on-chain txs you pay for, two chains, two coins), with per-leg estimates from
pactd's `estimateswapfees`. A "estimated" chip flags a conservative fallback
rate when a node is unreachable; an older pactd without the method falls back to
the static note.

## Theme & internationalisation

The MUI theme (`theme.ts`) ships a phoenix-derived **dark** palette plus a
matching **light** palette, both as CSS variables so the **light / dark /
system** toggle repaints live. Colour tokens are accessed through the semantic
`C.*` handles rather than raw hex.

The i18n layer (`i18n/`) is a small dot-addressed lookup with a single **English**
bundle today; all user-visible chrome routes through it, so adding a language is
just registering a new bundle. (pactd's `narrate()` output is backend-generated
and shown verbatim — outside the frontend i18n layer.)
