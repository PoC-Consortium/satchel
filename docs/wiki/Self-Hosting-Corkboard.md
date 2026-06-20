# Self-Hosting a Corkboard

**Corkboard** is the self-hostable alternative to Nostr (see [Transports](Transports)): a single axum + SQLite binary that stores signed offers and blind-relays encrypted blobs. No accounts, no fees, no custody, and it never matches trades. Communities run one when they want their own noticeboard.

For the full chapter see the **Pact handbook "Corkboard" chapter**: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Build and run

```sh
cd corkboard
cargo run -- --listen 127.0.0.1:9780 --db corkboard.sqlite
```

`--listen` defaults to `127.0.0.1:9780` and `--db` to `corkboard.sqlite`. Bind a public address and put it behind TLS to let others reach it.

## HTTP endpoints at a glance

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | Liveness — returns `"ok"`. |
| `POST` | `/v1/offers` | Submit a signed offer envelope → `{ "offer_id": … }`. |
| `GET` | `/v1/offers` | List unexpired, unrevoked offers (filter by `give`/`get`/`network`). |
| `POST` | `/v1/offers/revoke` | Revoke your own offer (signed). |
| `POST` | `/v1/relay` | Deposit a sealed blob for a recipient (blind store-and-forward). |
| `POST` | `/v1/relay/poll` | Fetch sealed blobs addressed to you (signed). |

Writes to `/v1/offers` and the poll endpoint require a valid BIP340 envelope signature; `/v1/relay` deposit is unauthenticated by design (the blob is already sealed). Max blob size 64 KiB; offers default to a 24h TTL, capped at 7 days.

## What it stores — and never sees

- **Stores:** signed public offers (id, identity, envelope, expiry) and blind relay blobs (recipient + ciphertext).
- **Never sees:** your keys, your funds, or any plaintext coordination. Relay blobs are sealed client-side (`PACTSEALED1`); the operator only ever sees the recipient pubkey and ciphertext.

A malicious or flaky board can withhold or drop messages, but cannot steal — funds are protected by on-chain timelocks, not by trusting the board.

## Pointing clients at it

- **pactd:** `--board-url http://your-host:9780` (comma-separate multiple boards).
- **Satchel:** **Settings → Network** → add the URL under **Corkboards**, then **Save & reconnect**.

## See also

- [Transports](Transports) · [Running pactd](Running-pactd) · [Building from Source](Building-from-Source)
