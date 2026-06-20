# Private (Off-Market) Offers

A *private offer* lets you trade with a specific counterparty without ever
posting to a public board — "trade with a friend". The artifact is a *slip*: the
exact same signed `offer` envelope that would otherwise go to the Corkboard,
serialized for an out-of-band channel like a chat message. No new wire fields, no
protocol bump — just a different distribution path.

## The slip

A slip is the signed `offer` envelope, base64url-encoded (unpadded) with a
version prefix:

```text
pactoffer1:<base64url(canonical_json(offer_envelope))>
```

The codec lives in `pact-proto/src/slip.rs` and is described byte-for-byte in
"Wire Format (pact-proto)". The decode side, `decode_slip`, is the **only trust
gate**: before handing back anything it rejects a bad prefix, malformed base64url,
a non-`Envelope`, an envelope whose `type` is not `"offer"`, and a bad BIP340
signature. A slip is therefore self-authenticating — a pasted slip is either the
maker's genuine, untampered offer or it is refused.

## Engine surface

The engine (`libswap` `engine.rs`) exposes four operations:

| Operation | Behaviour |
|---|---|
| `make_private_offer(network, give, get, t1_secs, t2_secs, ttl_secs, protocol)` | Builds and signs the offer, stores it **locally** under `private_offer:<swap_id>`, and returns the slip. It does **not** post to any board. |
| `take_offer_slip(slip)` | Decodes and verifies the slip, runs the same gating as a board take, records a pending take, and `relay_send_all`s the sealed `take` to every configured board. |
| `list_private_offers()` | Lists the maker's outstanding private offers. |
| `cancel_private_offer(id)` | Marks the offer revoked and deletes its row. |

Because `take_offer_slip` seals the take and fans it out over the blind relay,
the taker reaches the maker even though the offer was never public — the relay
addressing is by the maker's identity pubkey, which is inside the slip.

## RPCs

| Method | Params | Returns |
|---|---|---|
| `makeprivateoffer` | `give`, `get`, `t1_secs`, `t2_secs`, `protocol?`, `ttl_secs?` | `{ slip }` |
| `takeoffer` | `slip` | `{ taken: true }` |
| `listprivateoffers` | — | `{ offers }` |
| `cancelprivateoffer` | `offer_id` | `{ cancelled: true }` |

> **Warning** — On `makeprivateoffer` the optional `protocol` is parameter **4**
> and `ttl_secs` is parameter **5** (positional). Earlier docs listed them in the
> opposite order. Use named params to avoid the ambiguity:
> `{ "give": "btcx:1.0", "get": "btc:0.5", "t1_secs": 86400, "t2_secs": 43200,
> "protocol": "pact-htlc-v1", "ttl_secs": 3600 }`.

## The relay-assisted flow

1. **Maker** calls `makeprivateoffer` and gets a slip.
2. **Maker** pastes the slip into a chat (or any out-of-band channel) to the
   friend.
3. **Friend** calls `takeoffer` with the slip. The engine decodes/verifies it,
   gates the terms, and sends a sealed `take` through the blind relay of every
   configured board.
4. **Maker** polls their mailbox (the scheduler's `tick`), finds the take, and
   the swap proceeds exactly like a board-driven swap from there.

> **Note** — The maker **must be online and polling** to receive the take and
> drive the swap. A slip handed to a friend who acts on it while the maker is
> offline simply waits in the relay mailbox until the maker next polls (subject to
> the offer's TTL).

## Bearer-slip security

A slip is a **bearer** instrument: whoever holds it can take the offer. There is
no taker allow-list. Safety comes from the swap's own properties, not from
secrecy of the slip:

- **Fixed terms** — the amounts, assets, and timelock durations are signed into
  the offer; a slip cannot be edited without breaking its signature.
- **Atomic settlement** — the swap is an atomic cross-chain swap; either both
  legs settle or both refund. Taking a slip cannot make the maker lose funds
  one-sidedly.
- **Maker-funds-first** ordering and **TTL auto-expiry** — an unused slip lapses
  at `created + ttl_secs`.

So a leaked slip is, at worst, an offer someone else takes on the stated terms —
not a theft vector.

## Fully-manual CLI fallback

Everything above is automated, but the slip is just text, so a fully manual flow
works too:

```sh
# Maker: produce a slip and copy it out of band.
pact-cli --data-dir ~/.pact call makeprivateoffer \
  '{"give":"btcx:1.0","get":"btc:0.5","t1_secs":86400,"t2_secs":43200}'

# Friend: take it.
pact-cli --data-dir ~/.pact call takeoffer "pactoffer1:…"

# Maker: poll for the take and drive the swap.
pact-cli --data-dir ~/.pact call tick
```

For the swap handshake that follows the take, see the chapters on the v1 and v2
APIs and "The Swap Lifecycle".

> **Tip** — A slip carries **only** an `offer`. Other handshake envelopes
> (`take`, `init`, `accept`, …) are not slips; when moved manually they travel as
> raw JSON or as `PACTSEALED1:` blobs through the relay, not behind the
> `pactoffer1:` prefix.
