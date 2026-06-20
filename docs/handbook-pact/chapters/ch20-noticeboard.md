# The Noticeboard Abstraction

A *noticeboard* is where offers are advertised and coordination messages are
relayed. Pact treats the noticeboard as a pluggable transport behind a single
trait, so the swap engine does not care whether an offer travelled over an HTTP
Corkboard or a set of Nostr relays. Everything crossing the boundary is a
transport-agnostic, signed `Envelope` (see the chapter "Wire Format
(pact-proto)"); the relay carries only opaque, end-to-end-sealed blobs.

This chapter covers the trait itself, its two shipped implementations, and the
fan-out/browse model that lets two parties trade as long as they share *one*
board.

## The `Noticeboard` trait

The trait lives in `libswap` at `pact/libswap/src/board.rs`. It has exactly
five methods — there is no reputation, scoring, or accounts surface anywhere on
it:

```rust
pub trait Noticeboard {
    fn post_offer(&self, offer: &Envelope) -> Result<String>;
    fn offers(&self) -> Result<Vec<Envelope>>;
    fn revoke(&self, revocation: &Envelope) -> Result<()>;
    fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()>;
    fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>>;
}
```

| Method | Purpose |
|---|---|
| `post_offer` | Publish a signed `offer` envelope. Returns the offer id (the envelope `swap_id`). |
| `offers` | List the board's currently active offers (signed `Envelope`s). |
| `revoke` | Withdraw one of our own offers via a signed `revoke` envelope. |
| `relay_send_blob` | Store-and-forward a pre-*sealed* blob to recipient `to` (an x-only identity pubkey, hex). The blob is opaque to the board. |
| `relay_poll` | Fetch our coordination mail. Takes a signed `relay_poll` envelope (proving we own the recipient identity); returns `(cursor, blob)` pairs. |

> **Note** — The trait is deliberately **not** `Send + Sync`. Boards are built
> on demand and used synchronously inside a single engine call — no `await`, no
> thread hop. `NostrBoard` also borrows the engine's SQLite `Store`, whose
> connection is `!Sync`.

### Sealed relays, signed offers

The split between offers and relay is intentional:

- **Offers are public and signed.** Anyone may read them; the BIP340 signature
  proves the maker authored the terms, and listings cannot be forged.
- **Relay messages are sealed.** `relay_send_blob` only ever carries a
  `PACTSEALED1:` blob produced client-side. A board operator sees the recipient
  pubkey and ciphertext — never the sender, message type, or contents. There is
  no plaintext relay path. (Sealing is covered in "Wire Format (pact-proto)".)

## The two implementations

### `BoardClient` — HTTP Corkboard

`BoardClient` (`board.rs`) speaks the Corkboard HTTP API over a base URL. Each
trait method maps to one HTTP call:

| Method | HTTP call |
|---|---|
| `post_offer` | `POST <base>/v1/offers` → `{ "offer_id": … }` |
| `offers` | `GET <base>/v1/offers` → `{ "offers": [ … ] }` |
| `revoke` | `POST <base>/v1/offers/revoke` |
| `relay_send_blob` | `POST <base>/v1/relay` with `{ "to", "blob" }` |
| `relay_poll` | `POST <base>/v1/relay/poll` → `{ "messages": [ … ] }` |

Here `cursor` is the Corkboard's own autoincrement row id. The full wire surface
is documented in the chapter "Corkboard (self-hostable board)".

### `NostrBoard` — local buffer over Nostr

`NostrBoard` (`pact/libswap/src/nostr_board.rs`) implements the same trait but
performs **no network I/O at all**. Every method reads or writes the engine's
`nostr_*` SQLite buffers:

- `post_offer` / `revoke` push a row onto `nostr_outbox`.
- `offers` reads the active rows of `nostr_offer_cache`.
- `relay_send_blob` queues a gift-wrap for the recipient on `nostr_outbox`.
- `relay_poll` reads `nostr_inbox` past the cursor.

A separate background service (`pactd/src/nostr_service.rs`) drains the outbox to
relays and fills the inbox/cache from subscriptions. Because the inbox uses a
local autoincrement id, `relay_poll` returns the *exact* `(i64, blob)` contract
the HTTP board does, so the engine's cursor/dispatch loop is shared unchanged.
The relay-to-event mapping is covered in "Nostr Transport (pact-nostr)".

## Fan-out vs browse

The engine wires every configured board into one list, via `boards()`
(`engine.rs`): one entry per comma-separated HTTP `--board-url`, plus a single
aggregate entry named `"nostr"` if any Nostr relay is configured. From there,
the two directions are asymmetric.

### POST fans out to **all** boards

When you post or relay, the engine seals once and loops over *every* configured
board. `relay_send_all` (`engine.rs`) is the canonical example:

```rust
fn relay_send_all(&self, to: &str, envelope: &Envelope) -> Result<()> {
    let blob = crate::board::seal_envelope(to, envelope)?;
    let mut sent = false;
    for (_, board) in self.boards()? {
        if board.relay_send_blob(to, &blob).is_ok() {
            sent = true;
        }
    }
    // … succeeds if ANY board accepted
}
```

The same one sealed blob goes to all boards; the call **succeeds if any single
board accepts it**. Offer posting fans out the same way. This is best-effort
redundancy: a flaky or hostile board cannot block you as long as one path works.

### Browse selects **one** board

Browsing is per-board selection. `list_board_offers(sel)` (`engine.rs`, the
method behind the `boardlistoffers` RPC) picks the **one** board named `sel` (an
HTTP URL, or the literal `"nostr"`), defaulting to the first configured board if
`sel` is absent. It then filters out any offer you have locally revoked. The UI
browses one board at a time; it does not merge listings across boards.

> **Note** — All Nostr relays collapse into a single logical board, keyed in the
> engine as `relay_cursor:nostr`. Whether you configure one relay or six, the
> browse view and the cursor treat them as one board. Adding relays adds
> redundancy, not separate listings.

## The key property: one board in common

Because posting fans out to every board while browsing reads one, two parties
need only **one board in common** to trade. The maker may post to a Corkboard
*and* a half-dozen Nostr relays; the taker only has to be watching one of them.
There is no global, canonical order book — just overlapping noticeboards, any
one of which is enough.

## The `relay_poll` cursor model

Coordination mail is pulled, not pushed. Each board exposes a **per-board
monotonic cursor**:

- The engine persists the highest cursor it has processed per board
  (`relay_cursor:<board>`), so a poll only ever returns *new* messages.
- For the HTTP Corkboard the cursor is the `relay` table's `AUTOINCREMENT` id;
  for Nostr it is the local `nostr_inbox` autoincrement id. Either way,
  `relay_poll` returns `Vec<(i64, String)>` of `(cursor, blob)`.
- The poll envelope is **signed** (type `relay_poll`, with `body.since_id`),
  proving the caller controls the recipient identity, so no one can read another
  party's mailbox.

> **Tip** — A board reset (the SQLite file replaced) would leave a stale cursor
> *ahead* of every message on the fresh board. The Corkboard guards against this
> by serving from `0` whenever the polled cursor exceeds the maximum id it
> holds. See "Corkboard (self-hostable board)".
