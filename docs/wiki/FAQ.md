# FAQ

Short answers; follow the links for depth.

**Are there any fees?**
No platform fees — `platform_fee_sat` is hard-wired to 0. You pay only the on-chain mining fees for your own swap transactions.

**Who holds my coins during a swap?**
You do. The engine holds your seed and keys locally, signs everything itself, and auto-refunds via a timelock if the counterparty walks. No board, relay, or counterparty ever custodies your funds. See [Security Model](Security-Model).

**Can I just look at the board without setting up coins?**
Yes — pick **Browse in watch-only mode** on first run (or **Settings → Mode** later). You can read the whole board and withdraw offers you already own, but posting, taking, and funding stay blocked until you connect **two live coins**. See [Configuring Coins](Configuring-Coins).

**What languages does Satchel support?**
26 — switch any time from the globe-icon picker in the header, including during first-run onboarding.

**Why did the manual fee-step setting disappear?**
Fee-bumping is now automatic market-tracking: the engine bumps stuck swap transactions toward the live market feerate instead of a fixed manual escalation step, capped so you never pay more in fees than the amount being claimed. The old RBF-step knob is gone from the Fees page. See [Security Model](Security-Model).

**Do I need to run my own nodes?**
No — per coin you choose **your own node** (RPC; the node's wallet funds swaps) or **Electrum servers** (no node: chain data from the servers, the wallet on your Pact seed; mainnet requires ≥ 2 independent servers as cross-checking views). You still need **at least two coins live** before Satchel lets you trade. See [Configuring Coins](Configuring-Coins).

**Does this run on mainnet?**
Yes. Both v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) run on mainnet, reviewed. You alone hold your keys — safeguard your recovery phrase.

**Which coins are supported?**
The first pair is **BTCX ↔ BTC**. Litecoin (LTC) is the first added third coin. More coins can be added via `coins.toml` without recompiling — see [Configuring Coins](Configuring-Coins).

**What are the default ports?**
`pactd` JSON-RPC listens on **127.0.0.1:9737**; a Corkboard listens on **127.0.0.1:9780** by default. The RPC is loopback-only and refuses non-loopback addresses.

**Is my activity private?**
Coordination messages are sealed to the recipient (`PACTSEALED1`; gift-wrapped on Nostr), so boards and relays see only ciphertext. **Offers themselves are public and signed** by design. There is no plaintext downgrade. See [Transports](Transports).

**How do I add a coin?**
Drop a `[[coin]]` block (and an icon) into `coins.toml` next to the executable — no recompile. Satchel reads it for connection defaults and the engine reads it for chain params. Walkthrough in [Configuring Coins](Configuring-Coins).

**Where do my keys live?**
In the engine (`pactd`), on your machine only — derived from your BIP39 seed. Satchel stores no seed or passphrase. An encrypted seed is unlocked into engine memory per session. See [Security Model](Security-Model).

**What happens if my machine dies mid-swap?**
Your recovery phrase always restores your identity and keys. For an in-flight swap, `pactd` also backs up just enough state to your Nostr relays, encrypted to your own identity — so a fresh machine holding only your recovery phrase can rediscover it. Satchel only ever *warns* it found one; you explicitly confirm the restore, since two live machines driving one swap on the same seed could double-fund it. Only swaps started after this shipped are covered. See [Security Model](Security-Model) and the Satchel handbook chapter "Backup, Seeds & Safety".

**Can I build my own front-end / integrate the engine?**
Yes — `pactd` is a plain JSON-RPC 2.0 daemon and Satchel is just one client of it. Drive it with [pact-cli](pact-cli) or any HTTP client; the full method surface is in the [JSON-RPC API](JSON-RPC-API) page and the **Pact Developer Handbook** (<https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>).
