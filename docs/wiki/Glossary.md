# Glossary

One line each, for both users and developers.

- **Atomic swap** — a cross-chain trade where either both legs complete or neither does; no third party ever custodies the coins.
- **HTLC** — Hash Time-Locked Contract; the v1 swap output, spendable by a hash preimage before a timelock or refunded after it.
- **Adaptor signature** — a signature missing one secret offset; revealing the offset (the adaptor secret) completes it and, on-chain, leaks that secret to the counterparty. Basis of the v2 swap.
- **MuSig2** — a 2-round Schnorr multisignature scheme; v2 legs are co-signed 2-of-2 with it.
- **Taproot / tapleaf** — Bitcoin's Schnorr+Merkle output type; v2 uses a key-path spend for the cooperative redeem and a *tapleaf* script for the timelock refund.
- **CLTV** — `OP_CHECKLOCKTIMEVERIFY`; the opcode enforcing a timelock refund branch.
- **MTP** — Median Time Past; the block-time measure used to decide when a timelock has matured.
- **RBF** — Replace-By-Fee; rebroadcasting a transaction with a higher fee to speed confirmation.
- **CPFP** — Child-Pays-For-Parent; attaching a higher-fee child to bump a stuck parent (used to speed the un-bumpable v2 redeem).
- **Preimage** — the secret *s* whose hash *H* locks a v1 HTLC; revealing it claims the leg and links the two sides.
- **Maker / taker** — the maker posts an offer and funds first; the taker accepts an open offer and funds second.
- **Offer** — a signed advert of swap terms (amounts, coins, timelocks, protocol) posted to a board.
- **Slip** — a private (off-board) offer: the same signed offer envelope encoded as a `pactoffer1:` string, shared directly.
- **Corkboard** — a self-hostable order board that stores signed offers and blind-relays sealed blobs.
- **Noticeboard** — the engine's transport abstraction; a Corkboard or the Nostr aggregate each implement it.
- **Nostr / relay** — the default transport; offers are Nostr events and a *relay* is a server that stores and forwards them.
- **Merchant** — one trading identity = one seed = one data dir; the Bitcoin-Core-wallet analog inside the engine.
- **Seed / recovery phrase** — the BIP39 mnemonic from which all keys are derived; back it up.
- **Machine label** — the short one-way tag (e.g. `M-7f3a`) of an install's key-derivation scope; identifies which machine a swap belongs to when one seed runs on several.
- **Followed swap** — another machine's swap on the same seed: this machine monitors it read-only and never broadcasts, until an explicit **Take over** adopts it.
- **Passphrase** — optional secret that encrypts the seed at rest, entered per session to unlock it.
- **Timelock** — a deadline (`t1`/`t2`) after which a leg's refund path becomes spendable; the taker's is always earlier than the maker's.
- **Refund / redeem** — *redeem* claims the counterparty's leg (the trade succeeds); *refund* reclaims your own leg after the timelock (the trade is unwound).
- **Confirmation depth** — how many blocks deep an output must be before the engine treats it as final (per-coin, reorg safety).
- **RPC** — the loopback JSON-RPC 2.0 interface `pactd` exposes (default port 9737).
- **Pair** — a tradeable coin combination (e.g. BTCX ↔ BTC), derived from the coins you have configured.
- **Base / quote** — order-book convention: price is quoted as units of the *quote* coin per one unit of the *base* coin.
- **BTCX / PoCX** — Bitcoin-PoCX; the first supported asset, traded against BTC.
- **pactd / Pact / Satchel** — `pactd` is the engine daemon (≈ `bitcoind`); **Pact** is the engine project (`libswap` + `pactd` + `pact-cli`); **Satchel** is the desktop app (≈ `bitcoin-qt`).
