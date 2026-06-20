# Corkboard (self-hostable board)

*Corkboard* is the HTTP noticeboard: a single `axum` + SQLite binary that anyone
can run. It is deliberately dumb. It stores and serves signed offer envelopes and
blind relay blobs, and does nothing else — it never matches orders, never
executes a trade, never holds keys or funds, charges no fees, and has no
accounts. Humans pick offers; the swap happens entirely between the two `pactd`
instances. Running multiple independent operators is the goal (a Bisq-style
model), which is why two parties need only one board in common (see "The
Noticeboard Abstraction").

The source is `corkboard/src/main.rs`.

## Running it

```sh
corkboard --listen 127.0.0.1:9780 --db corkboard.sqlite
```

| Flag | Default | Meaning |
|---|---|---|
| `--listen` | `127.0.0.1:9780` | Bind address. |
| `--db` | `corkboard.sqlite` | SQLite database path (created if absent). |

The database is created on first run. The process serves until interrupted
(graceful shutdown on Ctrl-C).

## HTTP API

All write endpoints require a valid BIP340 envelope signature
(`pact_proto::envelope::verify`), so listings cannot be forged. The board never
talks to any chain — proof-of-funds verification is the *client's* job.

| Method | Path | Request | Response |
|---|---|---|---|
| `GET` | `/health` | — | `"ok"` |
| `POST` | `/v1/offers` | Envelope (`type=offer`) | `{ "offer_id": <swap_id> }` |
| `GET` | `/v1/offers` | query `give`, `get`, `network` (all optional) | `{ "offers": [Envelope…] }` |
| `POST` | `/v1/offers/revoke` | Envelope (`type=revoke`) | `{ "revoked": <bool> }` |
| `POST` | `/v1/relay` | `{ "to": <x-only hex 32B>, "blob": <string> }` | `{ "id": <i64> }` |
| `POST` | `/v1/relay/poll` | Envelope (`type=relay_poll`, `body.since_id`) | `{ "messages": [ { "id": <i64>, "blob": <string> }… ] }` |

Notes on each:

- **`GET /v1/offers`** returns only unrevoked, unexpired offers, newest first,
  `LIMIT 500`. The `give`/`get`/`network` query params filter on the offer
  body's `give_asset`/`get_asset`/`network`.
- **`POST /v1/offers/revoke`** flips `revoked` only on the signer's *own* offer
  (matched on `offer_id` **and** `identity`); `revoked` reports whether a row
  changed.
- **`POST /v1/relay`** is the **blind deposit**. It is **not** signed — by design
  (see Auth, below). It stores `{ to, blob }` and returns the new row id. The
  board never inspects the blob.
- **`POST /v1/relay/poll`** is signed; it returns only the caller's own mail with
  `id > since_id`, `LIMIT 100`.

## Storage model

Two tables (`open_db`):

```sql
CREATE TABLE offers (
    offer_id  TEXT PRIMARY KEY,
    identity  TEXT NOT NULL,
    envelope  TEXT NOT NULL,
    created   INTEGER NOT NULL,
    expires   INTEGER NOT NULL,
    revoked   INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE relay (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    recipient TEXT NOT NULL,
    blob      TEXT NOT NULL,
    created   INTEGER NOT NULL
);
```

The `relay.id` autoincrement is the cursor returned by `relay_poll`; the engine
persists the highest id it has seen.

## Limits and errors

| Constant | Value | Effect |
|---|---|---|
| `MAX_BLOB_BYTES` | 64 KiB | Relay blobs larger than this are rejected. The relay is for coordination envelopes, not file transfer. |
| `DEFAULT_OFFER_TTL_SECS` | 24h | Offer TTL when the body omits `ttl_secs`. |
| Offer TTL cap | 7 days | `ttl_secs` is clamped to a week — offers are not archives. |

All errors are returned as **HTTP 400** with a body of
`{ "error": <message> }`. Examples: a `to` that is not a 32-byte x-only pubkey, a
blob over 64 KiB, or an envelope whose `type` does not match the endpoint.

## Blind relay

The relay is content-blind. A blob arrives pre-sealed (`PACTSEALED1:` — ephemeral
ECDH + ChaCha20-Poly1305, sealed in the client) and the board stores and forwards
the ciphertext verbatim. The operator can see *that* recipient X has mail and how
big it is, never *what* it says or *who* sent it. Sealing is covered in "Wire
Format (pact-proto)".

## Reset hygiene

If a board's SQLite file is wiped or replaced, a returning client's saved cursor
could be *ahead* of every message on the fresh board, silently swallowing all new
mail. `relay_poll` guards against this: if the caller's `since_id` exceeds the
maximum id the board still holds for that recipient, it serves from `0`. Ids are
`AUTOINCREMENT` (never reused), so a cursor greater than the max only happens on a
real reset, not ordinary pruning. The poll is signed, so this only ever re-serves
the caller's own mail.

## Auth model

- **Writes to offers** (`/v1/offers`, `/v1/offers/revoke`) require a valid BIP340
  envelope signature. Revocation additionally checks the offer's `identity`
  matches the signer.
- **`/v1/relay` deposit is unauthenticated by design.** Anyone may drop a sealed
  blob addressed to a pubkey — this is how a counterparty who has never met you
  reaches your mailbox. Confidentiality comes from the seal, not from access
  control.
- **`/v1/relay/poll` is signed**, proving the caller owns the recipient identity,
  so strangers cannot drain someone else's mailbox.
- `GET /health` and `GET /v1/offers` are unauthenticated reads.

## Self-hosting walkthrough

1. **Build and run** the board:

   ```sh
   cargo run -p corkboard -- --listen 127.0.0.1:9780 --db corkboard.sqlite
   ```

2. **Verify** it is up:

   ```sh
   curl -s http://127.0.0.1:9780/health   # -> ok
   ```

3. **Point `pactd` at it** with `--board-url` (comma-separate to use several):

   ```sh
   pactd --data-dir ~/.pact --board-url http://127.0.0.1:9780 \
         --coin btcx=… --coin btc=…
   ```

   In Satchel, set the board URL in the noticeboard settings instead.

4. From there, posting an offer fans out to this board (and any others
   configured), and the two `pactd`s coordinate the swap through its blind relay.

> **Tip** — To expose a board beyond loopback, bind it to a routable address with
> `--listen` and front it with your own TLS terminator. The board itself is plain
> HTTP and does no transport encryption; the *payloads* it relays are already
> end-to-end sealed.
