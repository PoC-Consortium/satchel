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

Once established, the seed is either **plaintext** (the regtest intent) or
**encrypted** — created with a passphrase, in which case it lands *locked* and
stays unusable until you `unlock` it (or supply `PACT_PASSPHRASE` at boot).

The daemon reports exactly where it stands through `WalletStatus`:

```json
{ "seed_exists": true, "encrypted": true, "locked": false }
```

`locked` is true precisely when the seed is `encrypted` *and* the passphrase is
not currently held in memory. `unlock` verifies a passphrase by trial-decryption
and then holds it in memory for the process lifetime; `walletstatus` and
`getinfo` both surface these three flags.

> **Note** — A plaintext seed is appropriate for regtest and disposable test
> setups. For testnet or mainnet, create the seed with a passphrase so it is
> encrypted at rest, and supply `PACT_PASSPHRASE` (or call `unlock`) to open it.

## The encrypted seed format

The seed file is `seed.mnemonic`. In plaintext form it holds the BIP39 mnemonic
directly. Encrypted, it is a single line:

```text
PACTSEEDv1:<salt>:<nonce>:<ciphertext>
```

The mnemonic is encrypted with **ChaCha20-Poly1305** under a key derived by
**scrypt** (`N = 2^15`, `r = 8`, `p = 1`) from the passphrase and per-file salt.

> **Warning** — Seed installation is non-overwriting and atomic: `install_seed`
> **refuses to overwrite** an existing seed, and writes via a temp file with
> fsync and rename so a crash mid-write cannot corrupt or truncate the seed. To
> replace a seed you must remove the data directory's seed deliberately — the
> engine will never clobber it for you.

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
  `unloadmerchant`, `getmerchantinfo`) create and switch between them at runtime.
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
| `seed.mnemonic` | The BIP39 seed — plaintext, or `PACTSEEDv1:…` if encrypted. |
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
