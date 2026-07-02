# About this Handbook

This is the **Pact Developer & Integrator Handbook** — the reference for the
software that actually executes atomic swaps in the Satchel project. It is
written for people who run, drive, or build against the swap engine, not for
end users trading from the desktop app. If you are an end user, read the
*Satchel User Handbook* instead; this handbook assumes you are comfortable with
Bitcoin script, key derivation, JSON-RPC, and the Rust toolchain.

## Who this is for

You will get the most out of this handbook if you are one of the following:

- An **operator** running `pactd` — the local swap daemon — as a long-lived
  service: wiring it to your Bitcoin-Core-compatible nodes, configuring coins
  and confirmation depths, managing seeds and merchants, and keeping the
  auto-refund scheduler alive.
- A **front-end or tooling developer** building against the engine's JSON-RPC
  2.0 API: a custom UI, a bot, a monitoring dashboard, or an integration that
  posts and takes offers programmatically.
- A **protocol implementer** who wants to understand — or independently
  reimplement — the on-chain swap construction: the HTLC scripts, the
  Taproot/MuSig2 adaptor scheme, the key-derivation paths, the timelock margins,
  and the message flow.

> **Note** — The canonical, machine-checkable description of the wire format and
> on-chain construction lives in the `spec/` directory of the repository,
> together with deterministic test vectors. This handbook explains and contextualises
> that material; where the two ever disagree, the spec and the code win.

## How this handbook is organised

The handbook is grouped into six parts that move from orientation to deep
detail:

1. **Overview** — what Pact is, the crate map, and the architecture and trust
   boundaries. Start here.
2. **Running Pact** — building from source, running `pactd`, configuring coins
   and pairs, and managing seeds, wallets, and merchants.
3. **The JSON-RPC API** — the complete method surface: node/info, the
   seed/wallet lifecycle, coins and pairs, v1 and v2 swaps, board operations,
   private offers, and fees.
4. **The Swap Protocol** — the v1 HTLC and v2 adaptor state machines step by
   step, the on-chain scripts, timelocks and action deadlines, fee management,
   and auto-refund.
5. **Transports** — how identity-signed offers and sealed coordination blobs
   move over Nostr relays and the self-hostable Corkboard.
6. **Building Against Pact** — driving swaps with `pact-cli`, the generic
   `call` escape hatch, and end-to-end worked examples.

You do not have to read straight through. Operators can jump to the chapter
*Running pactd*; integrators to *The pact-cli* and the API part; implementers to
the protocol part.

## Conventions

This handbook follows a few consistent conventions:

- **Callouts** highlight things worth pausing on:

  > **Tip** — a shortcut or a recommended way to do something.

  > **Note** — context, a clarification, or a cross-reference.

  > **Warning** — something that can lose funds, corrupt state, or fail boot if
  > you get it wrong.

- **Text styles** carry meaning. `monospace` marks commands, flags, RPC method
  names, field names, file paths, and any exact text you type. **Bold** marks
  component names and UI labels. *Italics* mark a new term on first use.
- **Cross-references** name the target chapter by its title — for example, *see
  the chapter "Running pactd"* — never by number, because numbers shift as the
  handbook grows.
- **Code blocks** are fenced with a language hint (`sh`, `json`, `rust`,
  `text`) and are meant to be copy-pasteable.

## Source revision and status

> **Status** — Both swap protocols — v1 (HTLC) and v2 (Taproot/MuSig2 adaptor)
> — are reviewed and live on mainnet.

Rather than a release-version number, this handbook tracks the **source revision**
it was checked against: it was verified against commit `ae3cb0c` (July 2026), the
hash printed on the copyright page. The **code is the ultimate source of truth** —
when a precise detail matters for your integration, confirm it against the code at
that revision and the pinned test vectors before you rely on it. When the engine
moves, the hash is bumped and the affected pages are updated.

## Where to get help

- The repository `README.md` and the `docs/` directory carry the architecture
  and per-feature design notes.
- The `spec/` directory is the authoritative protocol description plus the
  `htlc_v1.json` / `htlc_v2.json` test vectors.
- The end-to-end harness under `pact/harness/` (notably `test_swap_e2e.py` and
  `test_adaptor_swap.py`) is the best executable reference for how a full swap
  is driven — when in doubt, read the harness.
