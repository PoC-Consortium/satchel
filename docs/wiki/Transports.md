# Transports

A *transport* is the noticeboard that carries identity-signed offers and forwards encrypted coordination blobs between counterparties. It never matches, executes, custodies, or charges — operators see ciphertext only. Satchel speaks two transports side by side, and you and a counterparty need just **one board in common**.

For the wire-level details see the **Pact handbook** transport chapters: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Nostr vs Corkboard at a glance

| | **Nostr** | **Corkboard** |
|---|---|---|
| Role | Default transport | Alternative, self-hostable |
| Infrastructure | Nothing to run | Run an HTTP board |
| Default | 6 prewired relays (in Satchel) | `127.0.0.1:9780` |
| Posting | Fans out across all configured relays | One HTTP board |
| Style | Censorship-resistant, public relays | Bisq-style, community-operated |

## Nostr (the default)

Satchel ships with six recommended relays prewired on a fresh install, so there is nothing to host — post and go. Offers map to Nostr events:

- Public offers are **addressable events of kind `31510`** (NIP-33), signed by your identity key (identity == npub).
- Coordination messages are **gift-wrapped, kind `1059`** (NIP-59), authored by a fresh ephemeral key so the sender is hidden.
- Offers carry a **rolling NIP-40 expiry**: `min(now + 1800s, created + ttl_secs)` — not a flat `ttl_secs`.
- Revocations are **NIP-09 deletions (kind `5`)**: when an offer is taken or withdrawn, viewers fetch the deletion, verify the author owns the offer, and drop it **immediately** — so a taken offer leaves every board at once instead of lingering until its expiry.

> **Note** — the engine's `--nostr-relay` defaults empty; the default relay list lives in Satchel. Saving an empty relay list disables Nostr.

## Corkboard (self-hostable)

Corkboard is a single small HTTP+SQLite binary a community can run for its own noticeboard. Default listen address `127.0.0.1:9780`. Point pactd at it with `--board-url`, or add it in Satchel under **Settings → Network**. See [Self-Hosting a Corkboard](Self-Hosting-Corkboard).

## Both are blind

Whichever transport you use, the privacy model is the same:

- **Offers** are public but **signed** — anyone can read terms, nobody can forge them.
- **Coordination** travels as **sealed `PACTSEALED1` blobs** — encrypted to the recipient, so the board/relay only ever sees ciphertext. There is no plaintext downgrade.
- Relays can withhold, delay, or drop messages, but **your funds are protected by timelocks**, not by trusting the relay. Mitigate liveness risk by using multiple boards and refreshing offers.

Boards and relays are independent, equal noticeboards — a post fans out across all of them at once, while browsing selects one board at a time.

## See also

- [Self-Hosting a Corkboard](Self-Hosting-Corkboard) · [Private Offers](Private-Offers) · [Satchel User Guide](Satchel-User-Guide)
