# Private (off-market) offers

Two friends can swap without advertising it. The maker produces a small text
artifact — a **slip** — and hands it to the friend over whatever chat they
already use (Signal, Discord, SMS). The friend pastes it into their Satchel and
takes it. No public listing, no new server, no new protocol.

This is **not** a private channel, presence, or messaging inside Satchel. The
chat is the friends' existing chat; Satchel only defines the slip artifact and
the import/take.

## 1. Key insight — only the *listing* is public

Everything after an offer is picked up is **already private**. The
`take → init → accept → funded → redeemed` handshake travels through the
Corkboard's blind, end-to-end-sealed relay (`../pact-proto/src/seal.rs`:
ephemeral-ECDH + ChaCha20-Poly1305, addressed to the recipient's x-only
pubkey). The relay server stores ciphertext keyed to a pubkey and learns
nothing — not message type, not terms, not the counterparty.

The *only* public surface is `GET /v1/offers` — an offer being **listed** on
the board. An offer is a self-verifying signed JSON envelope (`OfferBody`
wrapped in `pact-proto::Envelope`), with a BIP340 signature over its canonical
JSON and its own `created` + `ttl_secs` expiry.

Therefore **"off-market" is not a new protocol.** It is: sign the same offer,
**don't `POST /v1/offers`**, and hand the signed envelope to a friend directly.
The friend imports it and takes it exactly as if they had seen it on the board.

This *strengthens* the MiCA position (see `../README.md` and
`TRADING_ROADMAP.md`): a bilateral slip passed friend-to-friend has no listing,
no discovery, and no matching at all.

## 2. The slip — the thing you paste into chat

A **slip** is a private offer serialized for an out-of-band channel: the exact
same signed `offer` envelope that would otherwise go to the board, base64url-
encoded with a version prefix.

```
pactoffer1:<base64url(canonical_json(offer_envelope))>
```

- The encoded bytes are the **unchanged** offer envelope (`v`, `type:"offer"`,
  `swap_id`, `from`, `body`, `sig`). No new fields, no protocol bump.
- `swap_id` is the random 16-hex nonce, so a maker can have any number of
  private and public offers concurrently.
- An offer is ~300 bytes → ~400 base64url characters, which pastes into any
  chat.
- The friend's Satchel decodes the slip, verifies `sig` against `from`, checks
  expiry and that the pair is supported, then shows the **existing**
  take-confirmation card. Identical safety story to a board offer.

The codec lives in `../pact-proto/src/slip.rs` alongside
`../pact-proto/src/envelope.rs`: `encode_slip(&Envelope) -> Result<String>` and
`decode_slip(&str) -> Result<Envelope>`. `decode_slip` is the only trust gate —
it rejects, in order and **before** returning anything: an unknown/missing
version prefix, malformed base64url, bytes that are not a valid `Envelope`, an
envelope whose `type` is not `"offer"`, and a bad BIP340 signature over the
canonical JSON (verified against `from`).

## 3. Flow — relay-assisted

One drop. After the friend imports the slip, the rest is automatic over the
existing blind relay.

```
Maker (Alice)                                   Friend (Bob)
  makeprivateoffer  --> slip string
        |  (Alice pastes slip into their chat)
        | ----------------- chat ----------------> |
        |                              takeoffer <slip>
        |                              decode + verify + show card + confirm
        | <=== sealed `take` via relay (POST /v1/relay) ===|
  (Alice's pactd polls its mailbox, recognizes its own signed offer)
        |======== sealed `init` via relay ========> |
        | <======= sealed `accept` via relay =======|
        |   ... normal swap: fund / funded / redeem ...
```

The take envelope echoes Alice's full signed offer (the `take` body is
`{ "offer": <offer envelope> }`), so Alice verifies her *own* signature and
proceeds — she never needed the offer to be on a board. The only requirement is
that **Alice's pactd is running and polling** when Bob takes, the same as any
relay-coordinated swap.

The relay endpoint is the public Corkboard's `/v1/relay` (blind, so no privacy
loss) or any private relay the friends configure. The offer was never listed,
so it stays off-market regardless.

## 4. Engine and RPCs

The offer envelope, signing/verification, the blind relay
(`relay_send`/`relay_poll`/`seal`), the take→init→accept→swap path, the
take-confirmation flow, fee preview, and pending-take persistence are all
reused unchanged. The only new surface is small:

| Layer | Item | Behavior |
|-------|------|----------|
| `pact-proto` | `slip.rs` codec | `encode_slip` / `decode_slip` |
| `libswap` engine | `make_private_offer(network, give, get, t1_secs, t2_secs, ttl_secs, protocol) -> slip` | sign the offer, **store it locally** (`private_offer:<swap_id>`), and return the slip — but do **not** POST to the board |
| `libswap` engine | `take_offer_slip(slip)` | decode → verify → check expiry + pair support → `put_pending_take` → seal a `take` to `offer.from` via `relay_send_all`. This is `take_board_offer` with the offer sourced from the slip instead of a board GET |
| `libswap` engine | `list_private_offers()` / `cancel_private_offer(offer_id)` | list the locally stored outstanding offers; cancel = mark `offer_revoked:<id>` and delete the `private_offer:<id>` row so later takes for that `swap_id` are ignored |
| `pactd` RPC | `makeprivateoffer`, `takeoffer`, `listprivateoffers`, `cancelprivateoffer` | thin dispatchers that mirror `boardpostoffer` / `boardtake` |

The crucial difference from `boardpostoffer` is exactly one thing: no HTTP POST
to the board. `make_private_offer` stores the offer locally so the maker's
engine recognizes the incoming `take` and emits `init` through the unchanged
path.

End-to-end coverage lives in `../pact/harness/test_swap_e2e.py`.

## 5. Satchel

Private offers have their own section in Satchel, separate from the Corkboard
make-offer screen. See `SATCHEL_UI.md` for the navigation.

- **Private ▸ Create slip** (`PrivateCreateScreen.tsx`) reuses the shared
  `OfferForm` to collect give/get coins, amounts, and a timelock preset, then
  calls `makeprivateoffer`. It shows the returned `pactoffer1:` slip in a
  copy box with a one-line explainer ("Send this to your friend. They paste it
  into Satchel to take it. Nothing is locked; it expires in ~24h."). The slip's
  ttl defaults to ~24 h regardless of the swap timelock preset.
- **Private ▸ My slips** (`PrivateSlipsScreen.tsx`) lists the maker's
  outstanding slips (`listprivateoffers`) with pair, amounts, and an expiry
  countdown, plus a **Cancel** action (`cancelprivateoffer`) to stop honoring a
  slip before its ttl lapses. It polls so a slip that expires or is taken
  updates without a manual refresh.
- **Private ▸ Receive a slip** (`PrivateReceiveScreen.tsx`) is the taker entry:
  a text field for the pasted slip. On submit it decodes the slip locally
  (display only, `format-slip.ts`) to populate the **same** take-confirmation
  dialog the Corkboard uses (amounts you give/receive, maker-funds-first note,
  fee preview), then calls `takeoffer`. pactd is the authority — it re-decodes
  and verifies the BIP340 signature, checks expiry and pair support, then relays
  the take. From there the swap is indistinguishable from a board take and
  appears in the active swaps.

## 6. Security and behavior

- **Bearer, by design.** Whoever holds the slip can take it — exactly like a
  board offer, which is also takeable by anyone. The safety net is identical:
  fixed terms, atomic settlement, maker funds first, and the built-in
  `ttl_secs` auto-expiry (default 24 h). A forwarded or leaked slip is no more
  dangerous than a board offer a stranger sees; if it is stale it is simply
  expired.
- **Double-take.** Two people could both take the same slip before it expires.
  This is already possible with board offers and handled the same way: the
  maker funds exactly one `init`; concurrent funding is the existing UTXO-
  collision case already tracked in the harness. No new exposure.
- **Maker must be online** to receive the relayed take. If that is
  unacceptable, the fully-manual fallback (§7) is async.
- **Cancel.** A private offer is not on any board, so there is nothing to
  revoke remotely. Instead the maker keeps a local list of outstanding slips
  and can cancel one (mark it revoked + delete the local row, so any later take
  for that `swap_id` is ignored); otherwise it lapses at `ttl_secs`. Surfaced
  as **Private ▸ My slips** (§5).
- **Maker identity.** The slip reuses the maker's normal board identity pubkey.
  The friend learns it is them (fine — they are friends), and the maker's pactd
  already polls that mailbox, so the relayed take is recognized with zero extra
  setup. A throwaway per-slip identity (slip ↔ persona unlinkability) is a
  possible later hardening knob, not part of v1.
- **Tamper-evidence.** The slip is signed; any edit in transit fails
  `decode_slip` verification before the user ever sees a card.

## 7. Fully-manual fallback (CLI only)

For users who refuse any shared relay, the friends can exchange *every*
envelope as copy-paste blobs over chat — `offer → take → init → accept →
funded → redeemed` — with no server at all. This is a **`pact` CLI** capability
and is intentionally not surfaced in Satchel. The engine already supports it:
`../spec/protocol.md` §8.1 defines the messages as manual file/copy-paste
transport. The cost is ~4+ round-trips instead of one drop, in exchange for
being fully async with zero infrastructure. The same slip codec carries each
envelope.

## 8. Non-goals

- No in-app messaging, presence, or contact list — the friends' chat is the
  channel.
- No change to `../spec/protocol.md`, the HTLC scripts, or the offer envelope
  schema.
- No matching, discovery, or listing of private offers anywhere — that would
  reintroduce exactly what off-market avoids.
- No named or bound slips in v1 (revisit only if leaked-slip takes become a
  real problem; see §6).
