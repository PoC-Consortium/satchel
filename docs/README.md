# Documentation

This folder holds the project's documentation. It comes in three forms, all
kept in sync and checked against the code.

## 1. Handbooks (build to PDF)

Long-form, authoritative manuals written as per-chapter Markdown and built into a
single PDF with Pandoc + xelatex (see each handbook's `README.md` for the build).

| Handbook | Audience | Source |
|----------|----------|--------|
| **Satchel — User Handbook** | End users trading with the desktop app | [`handbook-satchel/`](handbook-satchel/) → `satchel-handbook.pdf` |
| **Pact — Developer & Integrator Handbook** | Developers, integrators, operators running `pactd`, building a front-end, or implementing the protocol | [`handbook-pact/`](handbook-pact/) → `pact-handbook.pdf` |

Build either with `./build.ps1` from its directory (requires Pandoc + a LaTeX
distribution providing `xelatex`).

## 2. GitHub wiki (concise orientation)

[`wiki/`](wiki/) stages the GitHub wiki pages — short, link-rich orientation
that points readers at the handbooks for depth. To publish, push the contents of
`wiki/` to the repository's wiki remote (`…/satchel.wiki.git`); the files are laid
out with GitHub's conventions (`Home.md`, `_Sidebar.md`, `_Footer.md`, and one
file per page).

## 3. Design docs (engineering artifacts)

The remaining Markdown files here are the original design/architecture documents
written during development. They remain useful as deep background and rationale,
but where they disagree with the code, **the handbooks (and the code) win** — the
handbooks were re-verified against the implementation. Notable points the design
docs predate:

- **v2 (Taproot/MuSig2 adaptor) swaps now run on every network, including
  mainnet** (`ADAPTOR_MAINNET_ENABLED = true`), under external audit — older docs
  still describe v2 as "refused on mainnet."
- The Nostr transport is **shipped and prewired** (not "future work"), with six
  default relays configured in Satchel.
- Satchel's navigation is the eight-screen Public/Private layout (not the older
  four-item nav).

| File | Topic |
|------|-------|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and trust boundaries |
| [`TRADING_ROADMAP.md`](TRADING_ROADMAP.md) | Product roadmap and phases |
| [`V2_ADAPTOR_SWAPS.md`](V2_ADAPTOR_SWAPS.md) | v2 Taproot/MuSig2 adaptor design |
| [`NOSTR_TRANSPORT.md`](NOSTR_TRANSPORT.md) | Nostr transport design |
| [`PRIVATE_OFFERS.md`](PRIVATE_OFFERS.md) | Private (off-market) offers design |
| [`SATCHEL.md`](SATCHEL.md), [`SATCHEL_BACKEND.md`](SATCHEL_BACKEND.md), [`SATCHEL_UI.md`](SATCHEL_UI.md) | Satchel app, backend surface, and UI design |

The normative protocol specification and test vectors live in [`../spec/`](../spec/).
