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

## D4 — Parity scenarios chosen (test_nodeless_e2e.py)

(1) v1 nodeless maker (bdk `wallet_send` funds leg A), (2) v2 nodeless taker
(bdk two-phase `wallet_build_funding` leg B), (3) v2 cancel pre-broadcast
(live proof the phantom-funding release works: balance restored to the sat),
(4) v1 nodeless↔nodeless (redeem sweeps INTO the second bdk wallet). Both
parties share ONE electrs — wallets differ by seed, which is exactly the
community-server model. Locked-seed gating stays unit-level (deliberate).
