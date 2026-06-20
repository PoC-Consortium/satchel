# Wire Format (pact-proto)

Every message that crosses a noticeboard — offers, takes, and the swap
handshake — is a single, signed JSON object called an *Envelope*. The
`pact-proto` crate (`pact-proto/src/`) defines the envelope, its canonical
encoding, the BIP340 signing rule, the sealed-blob format used by the blind
relay, and the private-offer slip codec. The crate is deliberately
chain-agnostic: it knows nothing about HTLCs or Taproot. Typed message *bodies*
live in the engine (`libswap`), keeping `pact-proto` a pure wire layer.

## The `Envelope` struct

Defined in `pact-proto/src/envelope.rs`:

```rust
pub struct Envelope {
    pub v: u32,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub swap_id: String,
    /// 32-byte x-only identity pubkey, hex.
    pub from: String,
    pub body: Value,
    /// 64-byte BIP340 signature, hex. Empty until signed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sig: String,
}
```

| Field | Wire name | Meaning |
|---|---|---|
| `v` | `v` | Envelope version (`u32`). |
| `msg_type` | `type` | Message type (serde-renamed to `type` on the wire). |
| `swap_id` | `swap_id` | The swap this message belongs to. |
| `from` | `from` | Sender identity: a 32-byte BIP340 x-only pubkey, hex. |
| `body` | `body` | Opaque JSON payload; shape depends on `type`. |
| `sig` | `sig` | 64-byte BIP340 Schnorr signature, hex. Omitted entirely when empty. |

> **Note** — On the wire the field is `type`, not `msg_type`. The Rust field is
> renamed because `type` is a reserved word. When you craft envelopes by hand,
> use `"type"`.

## Message types

The `type` value is one of:

`offer`, `take`, `revoke`, `relay_poll`, `init`, `accept`, `funded`,
`redeemed`, `abort`

The first four are board/coordination messages; `init`/`accept`/`funded`/
`redeemed`/`abort` drive the swap handshake itself. The *bodies* for these live
in the engine, not in `pact-proto`.

### `OfferBody`

The body of an `offer` envelope (`libswap` `board.rs`):

| Field | Type | Meaning |
|---|---|---|
| `protocol` | string | `pact-htlc-v1` or `pact-htlc-v2`. |
| `network` | string | `regtest` / `testnet` / `mainnet`. |
| `give_asset` | string | Coin id the maker gives. |
| `give_amount` | `u64` | Amount, smallest units. |
| `get_asset` | string | Coin id the maker wants. |
| `get_amount` | `u64` | Amount, smallest units. |
| `t1_secs` | `u32` | T1 timelock as a **duration**, not an absolute time. |
| `t2_secs` | `u32` | T2 timelock duration (`t2_secs < t1_secs`). |
| `ttl_secs` | `u64?` | Offer lifetime; defaults to 24h when omitted. |
| `created` | `u64` | Unix creation time, **inside** the signed body, so expiry is verifiable from the envelope alone. |

A `take` body simply echoes the maker's full signed offer back:
`{ "offer": <full signed offer envelope> }`. The maker can therefore rebuild the
terms statelessly and cannot be tricked into different terms — the embedded
offer's signature is re-verified and must carry the maker's own identity.

## Canonical JSON

Signatures commit to a *canonical* JSON encoding (`envelope.rs`, spec §8.1) so
that two implementations always hash the same bytes:

- Object keys are sorted **bytewise**.
- **No whitespace** between tokens.
- Integers are rendered in **decimal**.
- **Floats are rejected** — a number that is not an `i64`/`u64` is an error, not
  a rounding hazard.

The canonical writer is implemented explicitly (not via serde map ordering) so
that a `preserve_order` feature enabled by some unrelated dependency can never
silently change a signature.

## BIP340 signing

Signing (`envelope.rs`) is BIP340 Schnorr by the sender's identity key:

1. Clear `sig` (sign over the unsigned envelope).
2. Serialize to canonical JSON.
3. Compute the digest: `tagged_hash("pact/msg/v1", canonical_bytes)`.
4. Sign with deterministic `sign_schnorr_no_aux_rand`; hex-encode into `sig`.

Verification decodes `from` (32 bytes) and `sig` (64 bytes), recomputes the same
digest, and checks the Schnorr signature. **The caller must additionally check
that `from` matches the identity pinned for this swap** — `verify` only proves
the signature is internally consistent, not that it came from the expected
party.

The tagged-hash construction itself (`pact-proto/src/crypto.rs`) is the BIP340
form: `tagged_hash(tag, msg) = SHA256(SHA256(tag) || SHA256(tag) || msg)`.

## `swap_id` derivation

`swap_id` is 16 hex characters (8 bytes), derived via a tagged hash
(`crypto.rs`):

- v1 HTLC: `swap_id = hex(tagged_hash("pact/swapid/v1", H)[..8])`, where
  `H = SHA256(preimage)`.
- v2 adaptor: the same shape with tag `pact/swapid/v2` over the adaptor point
  `T`.

Offer swap_ids (before a concrete swap exists) are random 8-byte nonces.

## The `PACTSEALED1` sealed-blob format

The blind relay never sees plaintext. A coordination envelope addressed to a
recipient is sealed client-side (`pact-proto/src/seal.rs`, spec §10) into:

```text
PACTSEALED1:<hex(ephemeral_pubkey 33B)>:<hex(nonce 12B)>:<hex(ciphertext)>
```

Construction:

1. Lift the recipient's x-only identity pubkey to its **even-Y** point.
2. Generate a fresh ephemeral secret key; ECDH against the recipient point.
3. Derive the symmetric key: `tagged_hash("pact/relay/ecdh/v1", shared)`.
4. Pick a random 12-byte nonce; encrypt the envelope JSON with
   **ChaCha20-Poly1305**.

Opening reverses this with the recipient's identity key (negating the secret if
its parity is odd, to match the even-Y lift). The operator sees only the
ephemeral pubkey, the nonce, and the ciphertext — sender, type, and contents are
all hidden.

> **Warning** — There is **no plaintext downgrade**. `open_envelope` rejects any
> blob that does not start with `PACTSEALED1`. A board cannot inject an
> unencrypted envelope and have the engine parse it.

## The private-offer slip codec

A *slip* is a private (off-market) offer serialized for an out-of-band channel
such as a chat message (`pact-proto/src/slip.rs`). It is the **same** signed
`offer` envelope, with no new fields and no protocol bump:

```text
pactoffer1:<base64url(canonical_json(offer_envelope))>
```

The base64url is the RFC 4648 URL-safe alphabet, **unpadded** (no `=`).

`decode_slip` is the only trust gate. Before returning anything to the caller it
rejects, in order:

1. an unknown or missing `pactoffer1:` prefix,
2. malformed base64url,
3. bytes that are not a valid `Envelope`,
4. an envelope whose `type` is not `"offer"`,
5. a bad BIP340 signature over the canonical JSON (verified against `from`).

Because a slip is *bearer* — anyone holding it can act on it — this validation
is what guarantees a pasted slip is the maker's genuine, untampered offer before
a card is ever shown. Private offers are covered end-to-end in the chapter
"Private (Off-Market) Offers".
