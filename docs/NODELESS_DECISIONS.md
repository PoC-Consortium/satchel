# Nodeless wallet — overnight decision log (epic #58)

Decisions taken autonomously during the 2026-07-03/04 overnight run, so they
can be reviewed, amended or reverted one by one. Format: what was decided,
why, and the revert surface. Ordered chronologically.

## D1 — `listcoins` gains a `nodeless` flag (engine `coin_nodeless`)

The UI keys the whole send/receive/activity surface (and the "pact seed"
wallet label) off one boolean in `listcoins`, mirroring the engine's D5
dispatch (first URL non-`http://`). Revert surface: `Engine::coin_nodeless`,
one field in pactd's listcoins JSON, `CoinInfo.nodeless` in the UI.

## D2 — Parity suite runs the PoCX node on :18443 (`Harness(pocx_rest=True)`)

bindex-pocx hardcodes its REST endpoint to the network-default RPC port, so
any electrs-bearing stack (parity suite, playground) moves the PoCX node to
18443 + `-rest=1`. The main e2e suite keeps 19443 untouched. Goes away when
bindex grows a `--rest-url`-style override.

## D3 — electrs fork wart worked around in the harness, not patched

`blockchain.headers.subscribe` PANICS (killing the server) when a client
connects while the initial index is still empty (`electrum.rs:220`,
`tip_height().unwrap()`). The harness probes `blockchain.block.header(0)` —
which error-returns cleanly — before subscribing. UPSTREAM FIX WANTED in
electrs-pocx; the workaround is harmless to keep afterwards.

## D5 — Nodeless config = `funding_wallet: "pact-seed"` + `extra_backends`

No new satchel.json fields: the existing `funding_wallet` kind field (built
for exactly this — "only core-rpc for now") flips to `"pact-seed"`, the
Electrum URLs ride in `extra_backends`, and `compose_chain_data` joins them
verbatim (`auth_method` stays None — nothing to recompose at launch).
Revert surface: one branch in compose.rs + the CoinSetup mode toggle.

## D6 — Wallet actions are dialogs on the wallet card, not a new screen

Receive (fresh address + copy), Send (locale-aware amount, overspend guard,
engine-side fee), Activity (listtransactions table) live in
`dialogs/WalletActions.tsx`, shown only when `listcoins.nodeless`. Matches
the existing dialog idiom; a dedicated screen can grow later if the feature
sprawls. No QR code (no QR dep in the tree — add one later if wanted).

## D7 — i18n: new keys OPTIONAL in Bundle, translations deferred to a pass

All new copy is in en.ts (`wallets.*`, `coins.*`); the Bundle type marks the
new keys optional (the existing `progress.funding` mechanism), so all 26
locales compile and fall back to English at runtime per key. A translation
pass fills the 25 locales at the end of the run (or in review) without
blocking the feature.

## D8 — Playground: Alice's BTCX goes nodeless, with a faucet

**AMENDED (user, 2026-07-04): split into two variants.** The classic
`playground-cork.ps1` is restored (all-Core Alice, PoCX :19443; its port
sweep keeps the electrum ports so stale cross-variant runs get cleaned);
the nodeless stack lives in **`playground-nodeless.ps1`**, which drives the
shared `satchel_playground.py` with `--nodeless`. Everything below
describes the nodeless variant.

`playground-nodeless.ps1` + `satchel_playground.py --nodeless`: PoCX node moves to :18443
(+REST, D2), one electrs serves Alice; her satchel.json btcx entry is the
pact-seed/Electrum form (BTC + LTC stay node-backed so both worlds show
side by side). Her bdk wallet can't be pre-funded (the seed is created in
the wizard), so the driver polls her pactd and drops 100 BTCX once the
merchant exists. Old 19443 stays in the teardown port sweep for stale runs.
Dress-rehearsed headless end to end: faucet fired, a v2 board take gave
47 BTCX from the bdk wallet and completed; balances + activity exact.

## D9 — Nodeless BTC leg: vanilla electrs on the testnet-port trick; Core v31

(2026-07-04, user-requested nostr+fully-nodeless playground.) The vanilla
upstream electrs has the same bindex REST hardcode as the PoCX fork, so the
BTC regtest node parks on the TESTNET default RPC port (18332, `-rest=1`)
and vanilla electrs runs `--network testnet` — bindex asserts no genesis
(the PoCX fork already proved that), it just indexes whatever the node
serves. Binaries (gitignored): `harness/bin/btc-electrs.exe` (user-built
vanilla 0.11.1 from C:\code\pocx\electrum\dist) and
`harness/bin/btc-bitcoind.exe` upgraded Core v30 → **v31.0** (official
bitcoincore.org win64 zip; bindex needs `/rest/blockpart`). The whole e2e
suite runs on the v31 node from here on.

`playground-nostr-nodeless.ps1` + `satchel_playground_nostr.py --nodeless`:
Alice runs ZERO nodes — btcx over PoCX electrs (:19750), btc over vanilla
electrs (:19760), both wallets on her Pact seed, Nostr transport, no LTC;
faucet drops 100 BTCX + 0.05 BTC post-wizard. Bob/Carol stay node-backed.
Dress-rehearsed headless: both directions completed over Nostr in ~2.5 min
(realistic confs), each redeem sweeping INTO the respective seed wallet —
the BTC one through the vanilla electrs.

## D4 — Parity scenarios chosen (test_nodeless_e2e.py)

(1) v1 nodeless maker (bdk `wallet_send` funds leg A), (2) v2 nodeless taker
(bdk two-phase `wallet_build_funding` leg B), (3) v2 cancel pre-broadcast
(live proof the phantom-funding release works: balance restored to the sat),
(4) v1 nodeless↔nodeless (redeem sweeps INTO the second bdk wallet). Both
parties share ONE electrs — wallets differ by seed, which is exactly the
community-server model. Locked-seed gating stays unit-level (deliberate).
