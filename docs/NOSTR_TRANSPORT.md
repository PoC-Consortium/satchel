# Nostr transport (`pact-nostr`)

A second decentralized transport for Pact offers and relay messages, carried
over **Nostr** relays **alongside** the HTTP Corkboard rather than replacing
it. An operator configures HTTP boards and/or Nostr relays; offers and relay
traffic merge across all of them. At the **pactd/engine level the transport is
opt-in**: with no relays configured nothing is published, so a swap never
touches a public relay until the operator adds one. Satchel, however, **prewires
the six recommended relays on a fresh install** (see *Configuration and
Satchel*), so the default desktop experience ships with Nostr on; clearing the
list in Settings turns it back off.

This is a transport binding only — it carries the same signed envelopes and the
same `PACTSEALED1` sealed blobs the Corkboard already moves. The swap engine
(HTLC/adaptor build, fund, redeem, refund) is untouched.

- Normative wire mapping: [../spec/protocol.md](../spec/protocol.md) §8.8.
- Roadmap context: [TRADING_ROADMAP.md](TRADING_ROADMAP.md).
- Architecture / transport model: [ARCHITECTURE.md](ARCHITECTURE.md).

## Why Nostr fits

Three properties of the existing codebase make the binding small and
non-invasive:

1. **Pact identity already is a Nostr key.** A Pact identity is a BIP340 x-only
   Schnorr secp256k1 key (`keys.rs:identity_keypair`); that *is* a Nostr key.
   The same secret constructs a `nostr::Keys`, so a user's `npub` **equals**
   the `from` pubkey of an envelope (spec §8.2). No migration, no new primitive.
2. **The transport surface is a handful of signed-JSON methods.** Everything
   crossing the board boundary is a transport-agnostic signed `Envelope`, behind
   the `Noticeboard` trait (below). Nostr is just a second implementation.
3. **The blind relay maps onto a NIP-59-style gift wrap.** The existing seal
   (ephemeral-ECDH + ChaCha20-Poly1305 to a recipient x-only pubkey,
   `pact-proto/seal.rs`) already hides the sender. Wrapping it in an event
   signed by an *ephemeral* key and `#p`-tagged to the recipient reproduces the
   Corkboard relay's privacy exactly: the relay sees recipient + ciphertext,
   never the sender.

## The `Noticeboard` trait

`libswap/src/board.rs` defines the transport boundary as a five-method trait;
both transports implement it and the engine fans out across all of them:

```rust
pub trait Noticeboard {
    fn post_offer(&self, offer: &Envelope) -> Result<String>;
    fn offers(&self) -> Result<Vec<Envelope>>;
    fn revoke(&self, revocation: &Envelope) -> Result<()>;
    fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()>;
    fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>>;
}
```

- **`BoardClient`** (HTTP Corkboard) implements it directly.
- **`NostrBoard`** (`libswap/src/nostr_board.rs`) implements it against local
  SQLite buffers (below), never touching the network itself.

`engine.rs:boards()` returns `Vec<(String, Box<dyn Noticeboard>)>` built from
`board_url` (one entry per HTTP URL) and `nostr_relays` (one aggregate Nostr
entry). All fan-out call sites — `relay_send_all`, `post_offer`, `revoke`,
`sync_board` — run unchanged through the trait. All configured relays are one
logical board (`NostrBoard`), keyed by the cursor `relay_cursor:nostr` parallel
to `relay_cursor:{url}` for HTTP; cross-relay duplicates are absorbed by
`event_id` uniqueness in the buffer tables.

## The sync boundary

`nostr-sdk` is async (tokio websockets); the engine is synchronous. The
transport never bridges async into the engine. Instead the async relay client is
isolated behind **local SQLite buffer tables**, and each scheduler tick runs a
three-step pass (`pactd/src/nostr_service.rs`, driven from `tick` via
`nostr_pass`) so the engine lock is only ever held for fast SQLite work, never
across a relay round-trip:

| Step | Lock | Work |
|---|---|---|
| **A. `prep`** | engine lock | read the active merchant's identity, pending `nostr_outbox` rows, and the `offers`/`mailbox` fetch cursors |
| **B. `round`** | lock-free | publish the outbox to relays, then fetch new offers (by kind) and gift-wraps (`#p`=self) |
| **C. `apply`** | engine lock | write fetched events into the `nostr_*` buffers, mark outbox rows sent, advance cursors |

`NostrBoard`'s trait methods only ever **enqueue** outbox rows or **read** the
buffer tables — all synchronous:

- `post_offer` / `revoke` / `relay_send_blob` → insert a `nostr_outbox` row
  (kind `offer` / `revoke` / `giftwrap`).
- `offers` → read non-expired rows from `nostr_offer_cache`.
- `relay_poll(since_id)` → `SELECT … FROM nostr_inbox WHERE id > since_id`,
  returning the same `(i64 id, blob)` contract the HTTP board uses — so the
  engine's existing cursor/retry/dispatch logic in `sync_board` is unchanged.

Three buffer tables back this (migrations in `store.rs`): `nostr_outbox`
(pending publications), `nostr_inbox` (received gift-wrap blobs, autoincrement
`id` as the poll cursor), and `nostr_offer_cache` (deduped public offers).
Fetch cursors live in `meta` as `nostr_since:offers` / `nostr_since:mailbox`.

> **Design note — polling, not subscriptions.** The service fetches per tick
> (`fetch_events` with a `since` filter) rather than holding long-lived
> subscriptions. This matches how the engine already drives the HTTP relay each
> tick and sidesteps subscription/reconnect lifecycle; cross-relay overlap is
> absorbed by `event_id` uniqueness. One `NostrService` (relay pool) exists per
> pactd process — relays are process-level config — and the per-merchant
> identity is supplied per `round`.

## Nostr wire mapping

`pact-nostr` is the pure mapping + crypto crate (no relay I/O): it converts
envelopes ↔ Nostr events and builds subscription filters. The `pactd` service is
its only async consumer.

| Pact concept | Nostr representation | NIPs |
|---|---|---|
| Identity | secp256k1 Schnorr key; `npub` = x-only pubkey = envelope `from` | NIP-01, NIP-19 |
| **Offer** (public advert) | **addressable** event, kind **`31510`** (`OFFER_KIND`), `d` = `swap_id`, content = the signed `offer` `Envelope` JSON, NIP-40 `expiration` tag = `ttl_secs` | NIP-01, NIP-33, NIP-40 |
| Offer discovery | `REQ {kinds:[31510], since}` → `nostr_offer_cache`; pair/network filtering is client-side | NIP-01 |
| Offer revoke | NIP-09 deletion of the coordinate `31510:<pubkey>:<swap_id>` (`revocation_event`) | NIP-09 |
| **Relay message** (`take`/`init`/`accept`/`funded`/`redeemed`/`abort`) | **gift wrap**, kind **`1059`** (`GIFTWRAP_KIND`), signed by a fresh one-time key, `#p`=recipient, content = the `PACTSEALED1:` sealed blob | NIP-59-style |
| Mailbox poll | `REQ {kinds:[1059], #p:[me]}` → `nostr_inbox`; opened by `open_envelope` exactly as for the HTTP relay | NIP-59-style |

### Offers — signed and public

Offers are public adverts; the maker wants to be identified, so the offer event
is **signed by the identity key** and its content is the unchanged offer
`Envelope` (so `messages::verify` validates it independent of the Nostr layer).
An **addressable** kind with `d = swap_id` lets one maker hold and individually
replace/revoke many offers; NIP-40 `expiration` mirrors `ttl_secs` so relays
drop stale offers.

A **custom kind `31510`** (rather than NIP-99 classified listings, kind 30402)
keeps non-spendable swap offers out of generic Nostr marketplace clients that
couldn't act on them. A NIP-99 mirror for free discoverability remains a
possible later bonus.

### Relay messages — blind, sender-hidden

The handshake preserves the Corkboard relay's property — the relay learns the
recipient and ciphertext, **not the sender** — via a simplified gift wrap:

1. Seal as today → `PACTSEALED1:<eph_pk>:<nonce>:<ciphertext>` (`seal.rs`,
   reused verbatim; the ephemeral ECDH key already hides the sender).
2. Wrap in a kind-`1059` event **signed by a fresh one-time key**, with a single
   `["p", recipient_xonly]` tag and the sealed blob as `content` (`giftwrap`).
3. The recipient subscribes `{kinds:[1059], #p:[me]}`, runs `open_envelope` on
   each (skipping junk as today), and the resulting `Envelope` enters the normal
   `handle_relay_envelope` dispatch.

Full NIP-17/44/59 interop is deliberately **not** adopted: nothing else speaks
Pact swaps, so reusing `seal.rs` avoids a second crypto stack. Only the
kind-1059 + ephemeral-author + `#p` *structure* is borrowed; the payload stays
ours.

## No reputation or receipts

The transport carries **offers and gift-wrapped relay messages only**. It has no
receipt event, no reputation tally, and no `post_receipt`/`reputation` trait
methods — consistent with the rest of the suite, where trust rests on swap
atomicity (the timelock guarantee), not on an accrued score. A relay that
withholds or delays events affects **liveness only, never safety** — identical
to the Corkboard relay.

## Crates and dependencies

- **`pact-nostr`** — pure mapping + crypto (`offer_event`/`offer_from_event`,
  `giftwrap`/`unwrap_giftwrap`, `revocation_event`, `offers_filter`/
  `mailbox_filter`, `keys_from_secret_hex`). Depends on `pact-proto` (Envelope,
  seal) and `nostr`. No async.
- **`nostr` / `nostr-sdk`** — event/key/tag types and the relay pool. The
  async pool lives only in `pactd/src/nostr_service.rs`.
- Reused: `pact-proto::seal` (gift-wrap payload), the identity `Keypair`
  (builds `nostr::Keys` from the same secret), `rusqlite` (buffer tables).

## Configuration and Satchel

- **pactd**: `--nostr-relay <wss,…>` parallel to `--board-url`; the value flows
  into `EngineConfig.nostr_relays` and `build_engine` (`merchants.rs`).
- **Satchel**: `Config.nostr_relays: Vec<String>`, passed as `--nostr-relay` in
  launch wiring; `save_nostr_relays` mirrors `save_board`. On a fresh install the
  config is **prewired to `RECOMMENDED_NOSTR_RELAYS`** — six prober-verified
  public relays (`relay.damus.io`, `nos.lol`, `relay.primal.net`, `nostr.mom`,
  `nostr-pub.wellorder.net`, `offchain.pub`) — so Nostr is on by default. The
  user can clear the list in Settings to turn the transport off.
- **UI**: `BoardConfig` manages both the Corkboard URL(s) and the relay
  `wss://` list in one dialog ("Use recommended relays" pre-fills the curated
  set); the Settings screen shows the configured relays or "Off — using
  corkboard only". The front-end holds no transport logic.

## Security and privacy

- **Blindness preserved.** Gift wrap (ephemeral author + `#p` + sealed content)
  gives relays the same view as the HTTP relay: recipient + ciphertext, no
  sender. Offers remain intentionally public and signed.
- **Relay trust is liveness-only.** Relays can withhold/delay/drop events; funds
  are protected by timelocks regardless (spec §8.8, §10). Publishing to several
  relays and republishing offers before TTL mitigates liveness.
- **Metadata.** A relay sees *that* pubkey X has a mailbox and receives wraps; it
  cannot read content or learn senders. Offer events are public by design.
- **Key hygiene.** Nostr identity == Pact identity links a user's offers to their
  swaps — intended. Gift-wrap one-time keys are never reused.

## What is not touched

The swap engine (HTLC/adaptor build/fund/redeem/refund), offer semantics, and
all swap UI flows. The private-offer slip feature is unaffected — a slip is just
an envelope, so it rides Nostr gift-wraps identically. Only transport and config
change.

## Verification status

- **In-process data-path test — green.** `pact-nostr` unit tests pin event↔
  envelope round-trips, gift-wrap unwrap, and NIP-40 expiry. A pactd-level test
  drives a maker's outbox through the same `build_event` / `pact-nostr` mapping
  the relay round uses, hands the events to a taker by hand, and checks the
  taker's `NostrBoard` buffers surface them — covering everything **except** the
  websocket hop.

- **Live-relay end-to-end test — green.** `test_nostr_relay_swap`
  (`pact/harness/test_swap_e2e.py`) spins a throwaway bundled `nostr-rs-relay`
  alongside the regtest harness and runs a full board-driven swap over Nostr
  only, exercising the actual relay publish/fetch round-trip the in-process test
  skips.

  > **TODO:** a mixed HTTP-Corkboard + Nostr end-to-end run (offers fanned across
  > both transports within one swap) is still unbuilt — only the Nostr-only path
  > is covered today.

## Future / out of scope

Anti-spam **NIP-13 proof-of-work** on offers (revisit once there is real
traffic); full **NIP-17/44/59** DM interop; a **NIP-99** marketplace mirror for
free discoverability; **Tor** relay routing; reusing the same `pact-nostr` crate
to back a mobile Satchel. Retiring the HTTP Corkboard is explicitly **not** a
goal — the two transports coexist.
