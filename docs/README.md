# Documentation

This folder holds the project's documentation. It comes in three forms, all
kept in sync and checked against the code.

> **Status** — these docs were verified against commit `424834b`. The docs track
> the code by **commit hash** rather than a release version; when the code moves,
> the hash (in each handbook's front-matter and the wiki footer) is bumped and
> the affected pages are updated.

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

## 3. Roadmap & specification

The earlier design/architecture docs (`ARCHITECTURE.md`, `V2_ADAPTOR_SWAPS.md`,
`NOSTR_TRANSPORT.md`, `PRIVATE_OFFERS.md`, `SATCHEL.md`, `SATCHEL_BACKEND.md`,
`SATCHEL_UI.md`) have been **removed** — they are fully superseded by the
handbooks and the wiki, which are verified against the code. Their content lives
on in:

- the **Pact handbook** (architecture, the v1/v2 protocols, transports, private
  offers, and the full RPC/CLI surface), and
- the **Satchel handbook** + wiki (the app and its screens).

What remains here, because the handbooks deliberately do not cover it:

| File | Topic |
|------|-------|
| [`TRADING_ROADMAP.md`](TRADING_ROADMAP.md) | Product strategy and regulatory (MiCA) positioning — not a tech doc the handbooks replace. |

The normative protocol specification and deterministic test vectors live in
[`../spec/`](../spec/) and remain authoritative; the handbooks cite them.
