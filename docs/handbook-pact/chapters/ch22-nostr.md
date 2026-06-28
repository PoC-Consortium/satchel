# Nostr Transport (pact-nostr)

Nostr is Pact's default offer/relay transport. The `pact-nostr` crate
(`pact-nostr/src/lib.rs`) is **pure mapping**: it converts Pact `Envelope`s to
and from Nostr events and back. It opens no relay connections, runs no async
code, and touches no engine state. The relay pool, buffering, and polling live
in the background service (`pactd/src/nostr_service.rs`); the `Noticeboard`
facade (`NostrBoard`) lives in `libswap`. This separation is what lets the
engine's existing `messages::verify` and `seal::open_envelope` still apply
unchanged on the far side of a relay.

## Event kinds and constants

```rust
pub const OFFER_KIND: u16 = 31510;   // addressable advert (NIP-33)
pub const GIFTWRAP_KIND: u16 = 1059; // sealed relay message (NIP-59)
pub const RELAY_TTL_SECS: u64 = 30 * 60;   // rolling relay TTL (1800s)
const DEFAULT_TTL_SECS: u64 = 24 * 3600;   // offer-body TTL fallback
```

### Offers — kind `31510`

An offer maps to an **addressable** (NIP-33) event of kind `31510`:

- The `d` tag is the `swap_id`, so each `(maker pubkey, swap_id)` has exactly one
  current event — a re-publish replaces it.
- The `content` is the **unchanged signed offer envelope JSON**. The inner Pact
  signature authenticates the terms; the outer Nostr signature lets relays and
  clients accept the event. Both are signed by the maker's identity key.
- Plain descriptive tags carry `network`, `give`, and `get` (the give/get coin
  ids). Discovery subscribes by kind and filters the pair/network client-side,
  mirroring the HTTP board.
- The custom kind keeps non-spendable swap offers out of generic Nostr
  marketplace clients.

The reader (`offer_from_event`) verifies the Nostr event signature, confirms its
author matches the inner `offer.from`, and returns the envelope; the engine still
verifies the inner Pact signature and freshness before trusting terms.

### Gift wraps — kind `1059`

A relay message maps to a NIP-59-style gift wrap of kind `1059`:

- The `content` is the existing `PACTSEALED1:` blob (already sealed by
  `pact_proto::seal`).
- A `["p", recipient]` tag addresses the recipient by x-only pubkey.
- The event is signed by a **fresh, one-time ephemeral key**
  (`Keys::generate()`), which hides the sender from relays.

So the seal hides the *contents* from everyone, and the ephemeral author hides
the *sender* from the relay. The recipient unwraps the event and opens the seal
with their identity key, which also proves the message was addressed to them.

> **Note** — One-time gift-wrap keys are never reused. A relay learns only that
> "pubkey X has a mailbox"; it cannot link a wrapped message back to its sender.

## NIP-40 rolling expiration

Offer events carry a NIP-40 `expiration` tag, and it is **rolling**, not a fixed
`ttl_secs`. For a publish at time `now`:

```text
expiration = min(now + RELAY_TTL_SECS, created + ttl_secs)
```

That is, a published listing drops from relays a short while
(`RELAY_TTL_SECS` = 1800s) after each publish *unless the maker re-publishes*,
but never later than the offer's own final expiry (`created + ttl_secs`). The
engine refreshes live offers on a shorter cadence (`Engine::REFRESH_SECS`, 10
minutes) so a genuinely live offer never lapses, while an abandoned one falls off
relays quickly — a listing therefore reflects a maker who is actually online.

> **Warning** — Earlier docs described the offer expiration as `= ttl_secs`. The
> shipped behaviour is the rolling `min(now + 1800, created + ttl_secs)` above.

## Revocation — NIP-09

Revoking an offer (when it is taken or withdrawn) publishes a NIP-09
`EventDeletion` referencing the addressable coordinate
`31510:<maker pubkey>:<swap_id>` via an `["a", …]` tag, telling relays to drop
the listing.

Because relays may not honour NIP-09, viewers also **enforce revocations
client-side**. The sync loop subscribes to deletions (`{ kinds: [5] }`, its own
cursor) alongside offers, and for each deletion it **verifies the event signature
and that the deleting author matches the maker pubkey in the coordinate** — so a
maker can only revoke offers it signed, never someone else's. A verified deletion
writes a persistent `nostr_revoked:<swap_id>` tombstone and evicts the offer from
the cache; the tombstone is applied *before* upserts each round, so an offer a
relay keeps serving (NIP-09 ignored) never reappears. The effect: a taken or
withdrawn offer leaves **every** board immediately, instead of lingering on other
viewers' boards until its NIP-40 expiration lapses.

## Subscription filters

- **Offers:** `{ kinds: [31510] }` — subscribe by kind; pair/network filtering is
  client-side.
- **Deletions:** `{ kinds: [5] }` — NIP-09 deletions, so revocations are enforced
  client-side (see "Revocation" above).
- **Mailbox:** `{ kinds: [1059], #p: [me] }` — kind-1059 events tagged to our
  identity.

## Identity equals npub

A Pact identity *is* a Nostr npub. Both are BIP340 x-only keys over the same
32-byte secp256k1 secret, so `keys_from_secret_hex` yields a Nostr key whose
public key equals the Pact identity pubkey exactly. There is no separate Nostr
account.

## The sync / inbox model

`NostrBoard` does no I/O; the background service polls. Each scheduler tick runs
**three steps**, so the engine lock is held only for fast SQLite work and never
across a relay round-trip:

1. **`prep`** *(under the lock)* — read the active merchant's identity, pending
   `nostr_outbox` rows, and the offer/mailbox fetch cursors.
2. **`round`** *(lock-free)* — **fetch first**, then publish the outbox, with
   `FETCH_TIMEOUT` = 10s. The order matters: offers are addressable (replaceable)
   state pulled back behind a high-water `since` cursor, so fetching *before* we
   publish pins the cursor below anything we then send and guarantees the next
   round re-catches our own just-published offer. Publishing first could let the
   cursor skip past it (a busier maker's newer event, or a slow relay), so our own
   live offer would fall off our own board and show a stuck "posting…" badge. Mail
   and deletions are true append-only logs and are unaffected by the order.
3. **`apply`** *(under the lock)* — mark outbox rows sent, insert inbox rows,
   upsert the offer cache, and advance the cursors.

Polling (fetch-per-tick) rather than long-lived subscriptions keeps this aligned
with how the engine already drives the HTTP relay each tick and sidesteps
subscription/reconnect lifecycle. Cross-relay duplicates are absorbed by
`event_id` uniqueness in the buffer tables.

The buffer tables:

| Table | Role |
|---|---|
| `nostr_outbox` | Offers/revocations/gift-wraps awaiting publication. |
| `nostr_inbox` | Received gift-wrap blobs; autoincrement id = the `relay_poll` cursor; deduped by `event_id`. |
| `nostr_offer_cache` | Offers discovered from relays, with expiry. |

## Default relay list

The default relay list lives in **Satchel**, not the engine. A fresh Satchel
install is prewired with `RECOMMENDED_NOSTR_RELAYS` (`satchel/src/main.rs`), six
relays:

```text
wss://relay.damus.io
wss://nos.lol
wss://relay.primal.net
wss://nostr.mom
wss://nostr-pub.wellorder.net
wss://offchain.pub
```

`pactd`'s `--nostr-relay` flag defaults to **empty** — the engine itself ships no
relays, so the transport is opt-in. Satchel passes its configured relays to
`pactd` at launch. An explicit empty list a user saves is respected (transport
off).

> **Tip** — The Nostr transport runs *alongside* `--board-url`, not instead of
> it. Configure both for maximum redundancy; recall that two parties need only
> one board in common (see "The Noticeboard Abstraction").
