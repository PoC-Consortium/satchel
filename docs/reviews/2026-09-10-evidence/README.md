# Review evidence — 2026-09-10

Supporting material for [the parent report](../2026-09-10-protocol-to-implementation.md). These are review probes, not installed regression tests. They reference the reviewed Windows workspace and its built binaries.

- `handshake_probe.rs`: in-process reproduction of N1 (a pinned counterparty's `funding_ready` re-points the other side's leg pointer after `Signed`) and N2 (a replayed `init` with the same anchor resets a live participant record). Standalone crate depending on `libswap` by path; run with `cargo run` and read the printed traces. Uses temporary datadirs with `PACT_DISABLE_KEYRING=1`.
- `funding_ready_overwrite_test.diff`: an independent second reproduction of N1 as a unit test added to a scratch copy of `engine.rs`; also demonstrates that the nonce store refuses a re-sign under a changed pointer (the "what is sound" nonce claim).
- `wire_probe.rs`: library-level probes for the sealer nonce-length panic (N3: decoded lengths 0, 1, 11, 13, 24 panic; 12 errors cleanly), canonical-JSON determinism and float rejection, and the envelope signature covering version and type.
- `core_rpc.sh`: the JSON-RPC helper used for the regtest probes against a private Core v31 node on port 18899 (N7: `fundrawtransaction` without `minconf` selects unconfirmed change; prior #12: input reservations left locked after signing errors). The node was a throwaway regtest instance; no production node was touched.
- `cargo-audit-*.txt`: `cargo audit` output per lockfile (pact, pact-nostr, pact-proto, corkboard, satchel, crier, tools/relay-prober).
- `npm-audit-prod.txt`, `npm-audit-all.txt`: `npm audit --omit=dev` and `npm audit` for `satchel/ui`.
- `ui-build.log`: `npm run build` output with the bundle-size warning.

The Windows command-injection probe for N16 created and removed a marker file in the session scratchpad; its command line is quoted in the finding. No seed, cookie or wallet material is included.
