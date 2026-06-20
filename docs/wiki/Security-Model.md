# Security Model

What you trust, what you don't, and the risks stated honestly.

## You hold your own coins

- **Keys never leave your machine.** The engine (`pactd`) holds the BIP39 seed, derives every swap key from it, signs locally, and broadcasts its own transactions. Satchel persists no seed or passphrase, and the engine's RPC is loopback-only.
- **Refunds are automatic.** Every locked output has a timelock refund path. If a counterparty walks away, the engine refunds you once the timelock matures — it watches the chain and acts without you. The taker's deadline is always earlier than the maker's, so the maker cannot be stranded.
- **An encrypted seed is locked at rest.** Choose a passphrase and the seed is stored as scrypt + ChaCha20-Poly1305 and unlocked only in the engine's memory, per session.

## The transports are untrusted and blind

- **They see ciphertext, not deals.** Coordination messages are sealed to the recipient (`PACTSEALED1`; on Nostr, gift-wrapped under a one-time key). An operator sees only a recipient hint and an opaque blob. **There is no plaintext downgrade** — the engine refuses any message that isn't sealed.
- **Offers are public on purpose.** An offer is a signed advert of terms; that's meant to be readable. Only the coordination that follows is sealed.
- **A relay can withhold, not steal.** The worst a board or relay can do is delay, drop, or censor messages — *liveness*, not *safety*. Funds stay protected by the timelocks. Mitigate withholding by posting to multiple boards and refreshing offers.

## No reputation, no fees, no middleman

- **Trust = atomicity only.** There are no scores, receipts, or reputation systems — the protocol's safety is the only thing you rely on. Bearer *slips* (private offers) are takeable by whoever holds them; safety comes from fixed terms, maker-funds-first, atomic settlement, and TTL expiry.
- **No platform fees.** `platform_fee_sat` is hard-wired to **0**. You pay only on-chain mining fees.

## Things worth knowing

- **You are your own custodian.** No one can recover your funds for you. Keep your recovery phrase backed up offline, and never share it — the whole model rests on you holding your keys.
- **v2 cooperative redeem is not RBF-bumpable.** Its fee is sealed into the pre-signed adaptor signature. This is handled by over-provisioning the fee at swap start and by a CPFP child that bumps the redeem if the network gets busy. (The v2 single-key refund *is* bumpable; v1 redeem and refund are both bumpable.)
- **Liveness depends on relays.** If every board you use goes dark mid-swap, coordination stalls — but your timelock refund still protects the funds.

For the threat model in full, see the **Pact Developer Handbook** — <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>. Related: [How Atomic Swaps Work](How-Atomic-Swaps-Work) · [Architecture](Architecture).
