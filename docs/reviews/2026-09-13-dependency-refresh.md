# Dependency refresh

Base: `2794d23` (PR #262). Versions were queried from npm, crates.io, and official GitHub release metadata on 2026-09-13. This update targets current stable, supported dependency combinations; it does not replace the pinned wallet implementation with an unreviewed fork merely to remove version differences.

## Updated

| Area | Updated versions / changes |
|---|---|
| UI | React/React DOM 19.3, MUI 9.4, Vite 8.3, React plugin 6.1, TypeScript 6.0.3, ESLint 10.10, typescript-eslint 8.70, current Tauri JS APIs and BIP39 |
| Rust services | Axum 0.8.9, Nostr 0.45.5 / SDK 0.45.3, Poise 0.7, Reqwest 0.13.5, TOML 1.1.6, dirs 7, current Tokio/Serde/Clap and transitives |
| Wallet transport / protocol | Bitcoin 0.32.102, Electrum client 0.25, rustls 0.23.44, miniscript 13.1, MuSig2 0.4.1, protocol sealing ChaCha20-Poly1305 0.11; existing patched TLS verifier retained |
| Corkboard | rusqlite 0.40.2; explicit checked signed-integer conversions preserve SQLite timestamp representation |
| CI and tests | Node 24; current release SHAs for checkout/setup-node/cache/upload-artifact/Tauri action; Playwright browser regressions added to the required UI job |
| Regtest nodes | Bitcoin Core 31.1 and Litecoin Core 0.21.5.8, with updated official Linux archive checksums; matching Windows test binaries downloaded and SHA-256 verified locally |

All seven Rust lockfiles and the UI lockfile were refreshed. Unused workspace dependency declarations were removed rather than advertising versions not actually used by the extracted upstream crates.

## Compatibility migrations

- MUI inputs use `slotProps`; autocomplete merges the entire supplied slot set so refs, labels, adornments, and keyboard handling survive. Removed icon aliases are replaced. Non-dismissible dialogs remain controlled without a close callback. React refs have explicit initial values.
- Nostr builders use `FinalizeEvent`; the mapping crate declares `os-rng` itself rather than relying on feature unification from the SDK. Fetch operations retain their explicit timeout through the new request-builder API.
- Reqwest explicitly enables `query` for the Telegram poller and uses the new rustls feature name. Discord retains native certificate validation because its Serenity dependency still carries the older TLS stack otherwise.
- TOML documents use `toml::from_str` rather than the new standalone `Value` parser. A Crier regression loads a relative coins file, overrides builtin metadata, and validates a pair containing a new coin. The live Electrum probe exercises the shipped document too.
- ChaCha20-Poly1305's new fixed-array API uses a checked conversion of the untrusted nonce; malformed lengths remain errors. Existing signing/sealing vectors and upgrade scenarios protect wire compatibility.

## Security advisory reconciliation

All seven `cargo audit` runs report zero vulnerabilities, with no ignored advisory IDs. npm reports zero vulnerabilities. The eight open GitHub alerts on the previous default-branch lockfile were also checked directly against the new lockfile, independently of npm's result:

| Alert package | New resolved state |
|---|---|
| baseline-browser-mapping | 2.11.22, above patched 2.11.0 |
| browserslist | 4.28.9, above patched 4.28.7 |
| postcss (two alerts) | 8.5.28, above both patched ranges |
| js-yaml (two alerts) | No longer in the dependency tree |
| brace-expansion (two version lines) | Only 5.0.9 remains, above patched 5.0.7; old 1.x is absent |

GitHub may continue displaying default-branch alerts until this PR is merged and its dependency graph is rescanned. Machine-readable audit results and advisory-range reconciliation are recorded in `2026-09-13-dependency-audits.json`. No alerts were dismissed to obtain these results. Dependabot stays report-only; version-update PRs remain disabled.

## Deliberate constraints

- TypeScript 7.0.2 is published, but the latest typescript-eslint supports `<6.1.0`. Use the latest supported TypeScript 6.0.3 without forcing peer dependencies.
- The pinned `btcx` wallet exposes BDK 2 types. It remains on BDK 2.4.0, and the shared native SQLite link remains rusqlite 0.31 in the engine. Even BDK 3.1 still pins SQLite 0.31. Corkboard is independent and moves to 0.40.2. Moving the wallet to a different BDK major belongs in its upstream repository with persistence/migration validation.
- The upstream seed-store implementation retains its compatible keyring 3 / scrypt 0.11 / ChaCha20-Poly1305 0.10 dependencies. This PR does not change existing seed formats or native-keystore addressing. Their resolved versions pass Cargo audit.
- The `btcx` revision `7a6ec87` already includes one change beyond upstream default-branch `237cafd`; selecting the latter would roll back a fix. The pin stays. The latest electrs-btcx release is already the pinned `v0.11.1-btcx.1`. Custom PoCX and relay fixtures stay pinned; the rc19 daemon remains intentionally old for upgrade/mixed-version tests.

## Validation

- Rust tests: 241 Pact workspace + 11 protocol + 8 Nostr + 6 Corkboard + 18 Crier + 21 desktop = **305 passed**; relay-prober compiles and its zero-test target passes.
- Clippy with warnings denied passed for the workspace and all six standalone crates/tools.
- UI build, lint, and three browser regressions passed. The browser tests run the production bundle against an in-memory Tauri boundary: first-run and locked dialogs survive Escape, seed autocomplete works, invalid checksums block continuation, and valid phrase entry does not create/import a merchant before final confirmation.
- Live production Electrum probe: **9/9 passed**, verifying TLS, protocol negotiation, and chain genesis with a fresh isolated pin directory.
- YAML/TOML parse and whitespace checks passed.
- Full Windows regtest sweep with `python -X utf8 test_runner.py --rebuild-cache`: **83/83 scenarios passed**, all nine test files, including nodeless, rescue, takeover, and old/new daemon upgrade coverage. Bitcoin 31.1 and Litecoin 0.21.5.8 were used.
- Hosted Linux CI and security job results are attached to [PR #288](https://github.com/PoC-Consortium/satchel/pull/288/checks); the full hosted regtest suite remains a required merge gate.

The local sweep was built with the SDK's OS-randomness feature already enabled; adding the explicit feature to the standalone mapping crate does not change the daemon's feature set. Browser tests, CI configuration, and the Crier/diagnostic TOML parsing fixes were completed while that sweep ran. The latter fixes do not affect the daemon exercised by the sweep; their tests and the live probe were rerun separately.
