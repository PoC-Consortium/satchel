# How Atomic Swaps Work

An *atomic swap* trades coins on two different chains so that **either both legs complete or neither does** — no third party ever holds your coins. Three ideas make it work:

1. **The maker funds first.** Whoever initiates locks their coins on chain A; the taker then locks on chain B. Both legs are constructed so a counterparty cannot grab one without releasing the other.
2. **The chain enforces the deal.** A single on-chain secret links the two legs: the act of claiming one leg reveals what the other side needs to claim the other. The deal settles itself.
3. **A timelock refunds you if a side walks.** Every locked output has a *refund* path that becomes spendable after a deadline. If the counterparty disappears, the engine automatically refunds you when the timelock matures. The taker's deadline is always earlier than the maker's, so the maker can never be left exposed.

Pact ships **two protocol versions**, both running on mainnet under audit. For a given pair the engine prefers HTLC and only uses the adaptor route for Taproot-only pairs.

## v1 — HTLC (hashlock + timelock)

The maker picks a secret *s* and publishes its hash *H*. Each leg is a Pay-to-Witness-Script-Hash output that says "spend with the preimage of *H* before the timelock, or refund after it." When the maker claims their leg by revealing *s*, that same *s* is exposed on-chain, letting the taker claim the other leg. The shared preimage is what links the two legs.

## v2 — Taproot / MuSig2 adaptor

Each leg is an ordinary-looking Taproot output co-owned 2-of-2 via MuSig2, with a single-key timelock refund tucked in a tapleaf. The legs are linked by an *adaptor secret* baked into pre-signed redeem signatures: when the maker broadcasts their (adapted) signature, the secret leaks out of the 64-byte on-chain signature, and the taker uses it to finish their leg. On chain a completed swap looks like two normal key-path spends — no hash, no unusual script.

## At a glance

| | v1 HTLC | v2 adaptor |
|---|---|---|
| On-chain footprint | Visible HTLC script (hash + branches) | Looks like ordinary Taproot payments |
| Privacy | Lower — legs are linkable by *H* | Higher — no shared on-chain marker |
| Links the legs via | Hash preimage *s* | Adaptor secret in the redeem signature |
| Cooperative redeem RBF-bumpable | Yes | **No** (sealed into the pre-signed sig; mitigated by fee over-provisioning + a CPFP child) |
| Refund RBF-bumpable | Yes | Yes (single-key timelock path) |
| Mainnet | Yes (under audit) | Yes (under audit) |

The deep protocol detail — exact scripts, key derivation, the adaptor mechanism, timelock margins — lives in the **Pact Developer Handbook** protocol chapters: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>. See also [Security Model](Security-Model).
