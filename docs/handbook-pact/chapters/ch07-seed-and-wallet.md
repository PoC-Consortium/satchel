# Seeds, Wallets & Merchants

Every key Pact ever uses — identity, swap keys, preimages, adaptor secrets,
refund keys — is derived from a single BIP39 *seed*. This chapter covers the
seed's lifecycle (create, import, encrypt, unlock), the encrypted-seed format,
the *merchant* model (one seed per data directory), and what lives on disk.

## The seed lifecycle

A data directory starts with **no seed**. From there, exactly one seed is
established, by one of these routes:

- `createseed` — generate and persist a fresh seed; the mnemonic is returned
  **once** and never again.
- `importseed` — install an existing BIP39 mnemonic.
- `--auto-init` — at boot, `pactd` creates the seed and state on first run
  (flat layout) if none exists. Used by launchers like Satchel.
- `PACT_PASSPHRASE` — supplies the passphrase to *open an encrypted seed* at
  boot; it does not create one.

Once established, the seed is **never written in plaintext**. It lands in one of
three at-rest states:

- **Passphrase-encrypted** (`PACTSEEDv1`) — created with a passphrase, in which
  case it lands *locked* and stays unusable until you `unlock` it (or supply
  `PACT_PASSPHRASE` at boot).
- **Machine-key encrypted** (`PACTSEEDv2-keyring`, Windows/macOS) — a
  no-passphrase seed wrapped under a random key held in the OS keystore. It
  *auto-unlocks* from the keystore, so it is encrypted-but-never-*locked*: the
  daemon keeps signing across restarts with nothing to type.
- **Obfuscated** (`PACTSEEDv2-obfs`, Linux and anywhere no keystore is
  compiled or available) — a no-passphrase seed wrapped under a constant key
  that ships in the binary. This only lifts the file off plaintext ASCII; it is
  treated as **unencrypted** everywhere trust is decided.

The daemon reports exactly where it stands through `WalletStatus`:

```json
{ "seed_exists": true, "encrypted": true, "locked": false }
```

`locked` is true precisely when the seed is **passphrase-encrypted**
(`PACTSEEDv1`) *and* the passphrase is not currently held in memory. A machine-key
(`PACTSEEDv2-keyring`) seed reports `encrypted: true` but is never `locked` — it
auto-unlocks from the OS keystore — while an obfuscated seed reports
`encrypted: false`. `unlock` verifies a passphrase by trial-decryption and then
holds it in memory for the process lifetime; `walletstatus` and `getinfo` both
surface these three flags.

> **Note** — A no-passphrase seed is convenient (auto-unlock, refunds survive a
> reboot with nothing to type) and, on Windows/macOS, is encrypted at rest under
> an OS-keystore machine key. But that key lives on the same machine, so it
> protects the file at rest — not against an attacker logged in as the same OS
> user, who can read it just as the daemon does. On Linux the no-passphrase seed
> is only *obfuscated* (a constant built-in key) and is treated as unencrypted.
> For real protection on testnet or mainnet, create the seed **with a
> passphrase** and supply `PACT_PASSPHRASE` (or call `unlock`) to open it.

## The encrypted seed format

The seed file is `seed.mnemonic`, always a single line in one of three wrapped
forms:

```text
PACTSEEDv1:<salt>:<nonce>:<ciphertext>          # passphrase (scrypt + ChaCha20)
PACTSEEDv2-keyring:<key_id>:<nonce>:<ciphertext>  # OS-keystore machine key (Win/macOS)
PACTSEEDv2-obfs:<nonce>:<ciphertext>            # built-in obfuscation key (Linux / no keystore)
```

For `PACTSEEDv1` the mnemonic is encrypted with **ChaCha20-Poly1305** under a key
derived by **scrypt** (`N = 2^15`, `r = 8`, `p = 1`) from the passphrase and
per-file salt. Both `PACTSEEDv2` wraps also use **ChaCha20-Poly1305**: the
`-keyring` form under a random 32-byte key stored in the OS keystore under
`<key_id>` (the entry survives a same-machine folder move but not a copy to
another machine — by design); the `-obfs` form under a constant key that ships in
the binary, so it is *not* a secret and counts as unencrypted.

> **Warning** — Seed installation is non-overwriting and atomic: `install_seed`
> **refuses to overwrite** a seed it can still read, and writes via a temp file
> with fsync and rename so a crash mid-write cannot corrupt or truncate the seed.
> The one exception is recovery: a `PACTSEEDv2-keyring` seed whose OS-keystore
> key has vanished (moved to a new machine, reset keychain) is unreadable, so
> re-importing the *same* recovery phrase re-provisions it under a fresh machine
> key. A seed the engine can still read is never clobbered for you.

## The merchant model

A *merchant* is one identity backed by one seed in one data directory. Pact
supports two layouts:

- **Flat (default).** A single seed lives in the data-dir root. This is the
  layout the harness and `pact-cli` rely on, and what `--auto-init` produces. It
  is internally a single `default` merchant.
- **Nested (`--merchants`).** Each merchant lives under
  `<data-dir>/merchants/<id>/`, with a `merchants.json` manifest in the parent.
  `pactd` owns an in-process registry; one merchant is *active* at a time, and
  the merchant RPCs (`createmerchant`, `listmerchants`, `loadmerchant`,
  `renamemerchant`, `unloadmerchant`, `getmerchantinfo`) create, rename, and
  switch between them at runtime.
  This is the layout Satchel uses to manage several trading identities.

> **Warning** — Switching identities has a fund-safety guard: `loadmerchant`
> and `unloadmerchant` **refuse to switch away from a merchant that has a live
> (non-terminal) swap**. This prevents you from orphaning a swap that still needs
> its scheduler ticks to redeem or refund. Finish, refund, or let the swap reach
> a terminal state before switching away.

The `--merchants` flag is ignored once a flat seed already exists in the
data-dir root — an existing flat install stays flat.

## What lives on disk

Per merchant data directory (the root in flat mode, `merchants/<id>/` in nested
mode):

| File | Contents |
|---|---|
| `pact.sqlite` | All swap, offer, nonce, and Nostr state (see below). |
| `seed.mnemonic` | The BIP39 seed — never plaintext: `PACTSEEDv1:…` (passphrase), `PACTSEEDv2-keyring:…` (OS-keystore machine key), or `PACTSEEDv2-obfs:…` (obfuscated fallback). |
| `.cookie` | The per-run RPC cookie (data-dir root only). |
| `pact.conf` | Optional `rpcuser` / `rpcpassword` for RPC auth. |
| `merchants.json` | The merchant manifest (parent data dir, nested mode only). |
| `logs/pactd.log.<date>` | Rolling daily log files (data-dir root). Secret-free; see the chapter "Running pactd". |

The SQLite database is the engine's durable state. At a high level its tables
are: `swaps` and `adaptor_swaps` (the v1 and v2 swap records), `meta`
(counters, the board `relay_cursor`, and private offers), `pending_takes`,
`nonce_sessions` (the v2 MuSig2 use-once nonce state machine), `my_offers`, and
the Nostr `nostr_outbox` / `nostr_inbox` / `nostr_offer_cache` tables.

> **Note** — Records are strict: a missing required field fails the load rather
> than silently defaulting. There is no serde-default migration path — this is a
> deliberate "no backward compatibility" stance, so do
> not hand-edit the SQLite state and expect old shapes to be tolerated.

## Seed-only swap rescue (#54)

Losing the data directory entirely — a dead machine, a wiped disk — no longer
strands an in-flight swap. `pactd` opportunistically backs up in-flight swap
state to the configured Nostr relays, **encrypted to its own identity key**
(sealed-to-self, kind `31512`; see the chapter "Nostr Transport"), so a fresh
install restored from the seed alone can rediscover and finish or refund a swap
the local database has no record of — no separate backup step, and nothing
readable by anyone but you.

Snapshots are minimal and sparse, published at exactly these points:

- **v1** — once, at `accepted`. Any funding either side commits is then
  refundable (from the derived keys) and, once the counterparty's leg is
  spent on-chain, completable (the preimage extracts from the redeem witness).
- **v2** — at `accepted`, and again at `signed`. The `signed` snapshot
  additionally carries the assembled adaptor signatures — the *only* datum
  that is neither seed- nor chain-derivable — so a rescued swap can be
  **completed**, not just refunded.

Everything after that is re-derived from chain-watching once the snapshot is
adopted; the scheduler drives a rescued swap exactly like any other. A swap
that reaches a terminal state (`completed`, `refunded`, `aborted`) publishes a
NIP-09 tombstone so a machine restored later never resurrects a finished swap;
even a missed tombstone is safe, since re-driving a settled swap just detects
the spend and idempotently finalizes it.

### Restore is gated, never automatic

On boot, on unlock, and on every merchant load, `pactd` runs a **read-only**
preview (`Engine::rescue_preview`) against the configured relays and only
**warns** if rescuable snapshots exist — it never adopts one on its own. Two
live machines driving the same swap from one seed can double-fund it, so
adoption is an explicit, human decision:

- `restorefromrelay` — adopts every rescuable snapshot the relays hold that
  isn't already a local record, returning `{ restored, seen }`. Only call this
  once the machine that ran those swaps is genuinely retired.
- `rescuestatus` — the read-only twin: reports `{ pending, seen, warning }`
  without adopting anything, for a status check or a UI badge.
- CLI: `pact-cli restore` / `pact-cli rescue-status`.

A restored swap's local record always wins over a snapshot (adoption is
skipped if we already hold that `swap_id`), and restoring also raises the
seed's next-swap-index high-water mark from the snapshots, so a reissued index
can never reuse a completed swap's keys.

### Anchored participant keys

Restoring on a new machine has to re-derive the **participant's** swap keys
too — not just the initiator's. Since rc8, the two roles index their keys
differently (spec §4.2): the initiator still uses its local counter `i`, but
the participant derives its swap and refund keys from the swap's own public
anchor (`H` for v1, the adaptor point `T` for v2) via a hardened tagged-hash
path, needing no counter at all. Two machines holding the same seed therefore
derive the identical key for the identical swap, and can never issue one key
for two different swaps. `swap_index` is `Option<u32>` in both record types —
`None` means "participant, anchored." Existing pre-rc8 records keep their
original counter-based derivation; this is fully backward compatible with
databases created before this change.

> **Warning** — Only swaps **started on rc8 or later** are covered — a swap
> already in flight when you upgrade was never snapshotted and is not
> retroactively rescuable. And a v2 **maker** wiped in the narrow window
> between `accepted` and `signed` cannot complete the swap even after
> restoring: the assembled adaptor signatures are the one datum that isn't
> seed- or chain-derivable, and re-running the MuSig2 handshake is
> structurally forbidden (persisted nonce sessions refuse to sign twice, by
> design). The timelocked refund is the exit in that case.

See the chapter "Nostr Transport" for the wire format (kind `31512`, the
opaque per-swap `d`-tag, the tombstone event), and the Satchel handbook's
"Backup, Seeds & Safety" chapter for the rescue story told for end users.
