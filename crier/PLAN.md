# crier — Discord orderbook announcer (design plan)

Status: PLANNED (no code yet). Lives in `crier/` as a standalone server crate,
sibling to `corkboard/` (same pattern: independent crate at repo root, path-deps
into `pact-nostr` / `pact-proto`).

## 1. What it is

A headless, read-only server bot that watches the public Pact orderbook on Nostr
and announces it to Discord:

- **On demand** — slash commands render the current book / spread.
- **On change** — when the top of the book moves for a pair, it posts to a
  configured announce channel (debounced, so refresh churn stays silent).

crier holds no funds, signs nothing on-chain, and never takes offers. It has no
Pact identity; it only *verifies* other people's signatures.

## 2. What it consumes (wire contract)

All of this already exists and is reused, not reinvented:

| Concern | Source of truth |
|---|---|
| Offer advert | kind **31510** (NIP-33 addressable), `d` = swap_id — `pact-nostr/src/lib.rs` `OFFER_KIND` |
| Offer payload | `content` = signed Pact `Envelope` JSON (`type: "offer"`), body = `OfferBody` |
| Revocation | kind **5** NIP-09 deletion with `a` = `31510:<maker>:<swap_id>`, same author |
| Relay TTL | NIP-40 `expiration` tag = `min(now + 30min, created + ttl_secs)` |
| Final expiry | body: `created + ttl_secs` (default 24 h) |
| Identity | npub == Pact identity (BIP340 x-only); valid offer ⇒ `envelope.from == event.pubkey` |
| Verification | Nostr event sig **and** inner envelope sig over canonical JSON, tagged-hash `pact/msg/v1` (`pact-proto`) |

Key asymmetry to design around: **takes are not public** (they travel as
kind-1059 giftwraps). An outside observer sees a taken offer only as an offer
that stops being refreshed (drops off relays ≤ ~30 min later via NIP-40) or is
explicitly revoked (kind 5, immediate). So book *removals* have up to ~30 min
latency in the silent-death case; announcements must be worded as "no longer
advertised", never "filled".

## 3. Architecture

```
             ┌─────────────────────────── crier (single tokio binary) ──────────────────────────┐
 Nostr       │  ingest task                  book engine (lib)            discord task           │
 relays ─────┼─► nostr-sdk 0.44 pool ──► verify + decode ──► BookState ──► announcer (watch ch.) │
 (wss)       │  sub: kind 31510 + 5      (pact-nostr/-proto)   │  ▲                              │
             │  + periodic refetch                             └──┴──── slash cmds (/book …)     │
             └──────────────────────────────────────────────────────────────────────────────────┘
```

Three tasks over one shared `BookState` (`tokio::sync::watch` / `RwLock`):

1. **Ingest** — `nostr-sdk` relay pool, long-lived subscription on
   `offers_filter()` + `deletions_filter()` (both already in `pact-nostr`),
   plus a periodic full refetch (resilience against missed events / relay
   flaps). Applies the same hygiene pactd does: clamp peer `created_at` to
   `now + 15 min` before advancing any `since` cursor (#146 lesson).
2. **Book engine** — pure library (`crier::book`), no I/O, fully unit-testable:
   - Key: `(author_pubkey, swap_id)`; keep highest `created_at` (NIP-33 replace).
   - Tombstone on verified kind-5 (same-author check via
     `revoked_offer_from_event`); tombstones outlive the offer TTL so lagging
     relay copies can't resurrect it.
   - Sweep task drops entries past NIP-40 expiration / body expiry.
   - Derives per-pair ladders exactly like the Satchel UI (`format.ts` logic,
     ported): bid = maker gives quote / gets base, ask = the reverse;
     `price = quote_sats / base_sats`; levels grouped by reduced rational
     (gcd) price key; bids high→low, asks low→high. (Classification math
     only — *display* uses unit-price orientation, see §5, which flips
     base/quote relative to the Corkboard's `QUOTE_PRIORITY`.)
3. **Discord** — serenity 0.12 + poise (slash-command framework):
   - `/book [pair]` — ladder embed, depth 8 per side (same `DEPTH_CAP` as UI),
     spread + mid banner.
   - `/top` — one line per active pair: best bid / best ask / spread.
   - `/status` — relay connectivity, offer count, last event age, uptime.
   - **Announcer** — watches `BookState`; fires when a pair's top of book
     changes (see §4).

### Code layout

```
crier/
  Cargo.toml            # standalone crate; path deps: ../pact-nostr, ../pact-proto
  PLAN.md               # this file
  src/
    main.rs             # config load, rustls CryptoProvider install, task spawn
    config.rs           # crier.toml + env overrides
    ingest.rs           # relay pool, filters, event → book ops
    book.rs             # BookState, ladders, top-of-book diff (pure, tested)
    offer.rs            # minimal OfferBody mirror + envelope verification glue
    discord/
      mod.rs            # client bootstrap
      commands.rs       # /book /top /status
      announce.rs       # change detector + debouncer + embed rendering
      render.rs         # amount/price formatting (ports satchel/ui format.ts rules)
  crier.example.toml
```

### `OfferBody` dependency decision

`OfferBody` lives in `libswap` (`pact/libswap/src/board.rs`), which drags in the
whole engine (SQLite, Electrum, chain code). crier instead carries a **minimal
serde mirror** of the offer body (`protocol`, `wire`, `network`, `give_asset`,
`give_amount`, `get_asset`, `get_amount`, `ttl_secs`, `created` — unknown fields
tolerated), documented as wire-compat with `board.rs`. Follow-up (separate PR,
not blocking): hoist `OfferBody` into `pact-proto` so all three consumers
(libswap, corkboard, crier) share one definition.

## 4. Announcement semantics (the interesting part)

**Trigger**: per pair, the announcer compares a *top signature*
`(best_bid_price_key, best_bid_size, best_ask_price_key, best_ask_size)` after
every book mutation. Changes that fire:

- best bid/ask **price level** changed (incl. side appearing/disappearing),
- size at the top level changed by ≥ `announce.min_size_delta_pct` (default 10 %),
- a pair appeared on / vanished from the board.

**Non-events** (explicitly silent): offer refreshes (new event id, same terms —
NIP-33 replace with identical body), deeper-book changes, and anything on a
network other than the configured one.

**Debounce / anti-spam**:
- Coalesce window `announce.debounce_secs` (default 30): collect changes, post
  one message per pair with the net effect.
- Floor `announce.min_interval_secs` (default 60) per pair; if still churning,
  the next post summarizes the interval ("best ask moved 3× → now …").
- Comfortably inside Discord's 5 msg / 5 s channel limit by construction.
- Wording is provable-facts-only: "new best ask", "offer revoked", "no longer
  advertised" — crier never claims *filled/taken* (not observable; same product
  principle as the no-staleness-guessing rule in Satchel).

**Message content (user-reviewed 2026-07-26)**: unit prices in mBTC with a
"was X" comparison on the changed side; top-of-book both sides; spread + mid.
Explicitly EXCLUDED everywhere: ask/bid counts, per-level offer counts,
maker npub, protocol version (v1/v2), accumulated book volume (spoofable by
unbacked offers), and any "take it in Satchel" footer. A ladder level is just
`<summed size> BTCX @ <price>` regardless of how many offers sit at it.

**Line format (user-decided 2026-07-26): order convention, size before
price** — `<size> BTCX @ <price>`, e.g. `37 BTCX @ 0.676`. Order lines are
bare numbers, symmetric across sides; the unit is stated exactly ONCE per
message as a legend: the Discord embed footer (small type) `prices in
mBTC/BTCX` for announcements, the header line for the `/book` ladder. Ladder
rows right-align sizes so the `@` and price columns line up. The `/book`
header is ONLY `<pair> — prices in mBTC/BTCX`: no network name (one crier
instance = one network, it's config) and no "updated N s ago" (a static
message can't stay current; Discord timestamps every message anyway).

**Restart hygiene**: last-announced top signatures persisted to a small state
file (`state.json`); on restart crier rebuilds the book from relays first
(initial-sync grace period, default 60 s) and only announces *diffs vs the
persisted signatures* — no announcement storm on every deploy.

## 5. Configuration

`crier.toml` (+ `CRIER_DISCORD_TOKEN` env override — token never in the file):

```toml
network = "mainnet"                # offers on other networks ignored
relays  = ["wss://relay.damus.io", "wss://nos.lol", "wss://relay.primal.net",
           "wss://nostr.mom", "wss://nostr-pub.wellorder.net", "wss://offchain.pub"]
           # default = RECOMMENDED_NOSTR_RELAYS (satchel/src/main.rs)
coins_file = "../satchel/coins.toml"   # symbols/decimals for rendering; optional,
                                       # falls back to built-in btcx/btc/ltc/doge table

[discord]
guild_id = 0                       # command registration scope (0 = global)
announce_channel_id = 0            # 0 = announcer disabled, commands only

[announce]
debounce_secs = 30
min_interval_secs = 60
min_size_delta_pct = 10
pairs = ["btc/btcx"]               # whitelist of pairs crier cries for.
                                   # DEFAULT = ["btc/btcx"]; empty list = announce
                                   # nothing (commands still browse all pairs).

[render]
btc_unit = "mbtc"                  # unit for BTC-quoted prices: btc|mbtc|sat
```

Pair entries are unordered coin-id sets — `"btc/btcx"` and `"btcx/btc"` mean
the same pair. Unknown coin ids in `pairs` are a startup error (fail fast,
not silent). Slash commands are not restricted by the whitelist; only the
announcer is.

**Display orientation (DECIDED 2026-07-26): unit prices.** crier deliberately
diverges from the Corkboard's `QUOTE_PRIORITY` orientation: for BTC pairs, BTC
is the *quote*, so a price is always "what 1 BTCX costs", shown in **mBTC**
(`render.btc_unit`, default `mbtc`) — comparable at a glance across offers of
any size. Sizes render in base-coin units (BTCX). Internal book math is
unchanged (exact rational price keys); only rendering flips.

Relays are duplicated from Satchel's defaults on purpose — they live in
`satchel.json` (per-user state), not in any shared config; `coins.toml` is the
only shared artifact and carries no relay info.

## 6. Verification & hardening

- Install `rustls::crypto::aws_lc_rs::default_provider().install_default()`
  **before** the relay pool connects (same dual-provider footgun pactd hit —
  `pactd/src/main.rs:2170`; without it every `wss://` handshake fails).
- Accept an offer into the book only if **all** hold: valid Nostr sig; content
  parses as `Envelope` with `type == "offer"`; `envelope.from == event.pubkey`;
  inner BIP340 sig verifies over canonical JSON; `network` matches config;
  amounts > 0; not expired. Anything else is dropped and rate-limit-logged.
- Kind-5 honored only when the deletion author == offer author (already what
  `revoked_offer_from_event` enforces).
- Untrusted text (asset ids, etc.) is never interpolated into Discord markdown
  unescaped; only known coin symbols render as-is.
- No inbound ports; outbound wss to relays + Discord gateway only. Memory
  bounds: cap tracked offers (e.g. 10 000) and per-author offers (e.g. 100) to
  shrug off relay spam.

## 7. Testing

- **Unit (bulk of coverage)**: `book.rs` + `announce.rs` are pure — feed
  synthetic events built with `pact-nostr`'s own constructors (`offer_event`,
  `revocation_event`) and assert ladder shape, replace semantics, tombstone
  wins, expiry sweep, top-signature diffing, debounce coalescing.
- **Golden render tests**: embed/markdown output snapshots for a fixture book.
- **Integration (manual, feature-gated)**: `cargo run -- --dry-run` connects to
  real relays, prints the book and would-be announcements to stdout, no Discord.
  (No relay-in-CI: nostr-rs-relay doesn't build on Windows, and the e2e harness
  is pactd-shaped; dry-run against public relays is the field check.)
- CI: `cargo fmt --check`, clippy, unit tests — piggyback on existing workflow
  matrix; no new required checks beyond build+test for the crate.

## 8. Milestones

1. **M1 — book engine**: crate skeleton, config, offer mirror + verification,
   `BookState` + ladders + tests. Deliverable: `--dry-run` prints a live book
   from mainnet relays.
2. **M2 — Discord read**: serenity/poise bootstrap, `/book` `/top` `/status`.
3. **M3 — announcer**: top-signature diffing, debounce, state file, wording.
4. **M4 — ops**: Dockerfile + systemd unit sample, structured logs (tracing),
   and `crier/README.md` with a full **deploy runbook**: create the Discord
   application + bot, invite URL (scopes `bot` + `applications.commands`;
   perms: Send Messages, Embed Links), obtain channel/guild ids (developer
   mode), write `crier.toml`, set `CRIER_DISCORD_TOKEN`, verify with
   `--dry-run`, then run under systemd or Docker; upgrade + log-reading notes.

Out of scope for v1: per-channel `/watch` subscriptions, price-move thresholds
in quote terms, historical charts, multi-network instances (run one crier per
network instead), any write path to Nostr.

## 9. Decisions & open questions

- **Language: Rust** (DECIDED 2026-07-26) — matches the repo toolchain,
  corkboard is the precedent for a sibling server crate, and `pact-proto`'s
  canonical-JSON + tagged-hash verification is reused instead of reimplemented.
  Discord via serenity 0.12 + poise.
- Announce channel: one global channel per crier instance for v1 (config), not
  per-guild discovery.
- Depth shown fixed at 8 (UI parity) — could become a `/book depth:` option.
