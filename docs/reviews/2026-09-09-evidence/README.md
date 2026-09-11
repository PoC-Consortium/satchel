# Review evidence

The parent report describes evidence strength and limitations for each finding.

- `merchant_perf_release.py`: actual release-daemon merchant-switch probes and synthetic-history benchmark. Output: `satchel-review-release-perf.log`.
- `merchant_perf_probe.py`: debug-daemon version, also used as scaffolding by `lock_perf_probe.py`.
- `lock_perf_probe.py`: private local delayed-board service and concurrent daemon RPC probe. Output: `registry-lock.log`.
- `ui_probe.cjs`: executes the actual TypeScript formatting/terminal predicates. Output: `ui-predicates.log`.
- `nonce_probe.rs`: isolated malformed-nonce conversion probe.
- `v2_depth_probe.py`: reuses the project's regtest scenario to call direct redeem with zero confirmations.
- `refund_outage_probe.py`: regtest automatic-refund outage probe.
- `satchel-review-e2e-20260909.log`: full initial harness execution, including Windows encoding failures. UTF-8 rerun logs accompany it when complete.

These are review probes rather than installed regression tests. They contain paths for the reviewed Windows workspace and require its existing built binaries/toolchains. Regtest probes reuse the harness's fixed ports and must run sequentially with other harness tests. They create private temporary data directories; no production wallet or merchant database is a target. The merchant probe inserts synthetic records directly into its own SQLite database to isolate lifecycle checks. Do not point it at a real wallet database.

The full test log includes temporary paths and ephemeral regtest identifiers. No real seed/cookie files or wallet databases are included.
