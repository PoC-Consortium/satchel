# Getting Started

Two fast paths: one for people who just want to trade, one for developers who want to run the engine. Pick yours.

> **Status** — live; v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) are reviewed and running on mainnet. You alone hold your keys.

## For users (the desktop app)

1. **Download Satchel.** Grab the bundle for your OS from the [releases page](https://github.com/PoC-Consortium/satchel/releases) and launch it. Assets follow the Phoenix-PoCX naming scheme: Windows ships an NSIS installer **`Satchel-<version>-Windows-x64-Setup.exe`** (no MSI), and Linux has **`.AppImage`**, **`.deb`**, and **`.rpm`** builds. The app bundles and supervises the swap engine for you — there is nothing else to install. The picker in the header offers **26 languages**.
2. **Create a merchant + seed.** On first run, Satchel walks you through creating a *merchant* (one trading identity = one seed = one data dir). Write down the recovery phrase, then choose **No passphrase** (simplest) or **Encrypt** (a passphrase you'll type each session).
3. **Connect at least two coins.** Satchel will not let you trade until **two coins are live** (e.g. BTCX and BTC). For each, choose **your own node** (point Satchel at its RPC) or **Electrum servers** (no node — the wallet lives on your Pact seed) and let it validate the connection. See [Configuring Coins](Configuring-Coins).
   - *Just looking?* Choose **Browse in watch-only mode** to skip coin setup entirely — you can read the whole board, but you can't post, take, or fund until two coins are live.
4. **Browse the Corkboard.** The board is a two-sided order-book ladder of open offers. Pick a price level, read the terms (amounts, safety timelocks, swap type).
5. **Take an offer.** Confirm the dialog and the engine drives the swap to completion, auto-refunding if the counterparty walks away. Follow progress on the **Swaps** page and the active-swaps dock.

Full walkthrough: the **Satchel User Handbook** — <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>. See also [Satchel User Guide](Satchel-User-Guide), [Configuring Coins](Configuring-Coins), and [Private Offers](Private-Offers).

## For developers (the engine)

```sh
cd pact
cargo build && cargo test            # unit + protocol-vector tests (v1 + v2)

# run the daemon on regtest and drive it with the CLI
cargo run -p pactd -- --network regtest \
  --coin btcx=<rpc-url> --coin btc=<rpc-or-electrum-url>
cargo run -p pact-cli -- getinfo

# end-to-end on regtest
python harness/test_swap_e2e.py      # full BTCX↔BTC v1 swap
python harness/test_adaptor_swap.py  # v2 adaptor swap end to end
```

One-shot regtest playground (regtest nodes + headless counterparties + Satchel):

```sh
./tools/playground-cork.ps1            # over a Corkboard
./tools/playground-nostr.ps1           # over a local Nostr relay
./tools/playground-nostr-nodeless.ps1  # Nostr + nodeless: wallets on the Pact
                                       # seed via local electrs (LTC rides
                                       # along as the one local-node coin)
```

Each script builds the whole stack, brings it up, and blocks on the Satchel window — close it and everything tears down (`-Down` force-tears a stale run).

Details: [Running pactd](Running-pactd) · [pact-cli](pact-cli) · [JSON-RPC API](JSON-RPC-API) · [Building from Source](Building-from-Source). Deep reference is the **Pact Developer Handbook** — <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.
