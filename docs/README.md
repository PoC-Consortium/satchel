# Documentation

This folder holds the project's documentation. It comes in three layers, all
kept in sync and checked against the code.

> **Status** — these docs were verified against commit `8dc8318`. The docs track
> the code by **commit hash** rather than a release version; when the code moves,
> the hash (in each handbook's front-matter and the wiki footer) is bumped and
> the affected pages are updated.

## The rule

**The handbooks are canonical.** If a handbook chapter covers a topic, no other
doc duplicates it. A design doc exists under [`design/`](design/) only when it
carries something the handbooks deliberately do not: internal invariants,
section anchors cited from code comments, or decision rationale. Every doc that
stays in this tree is kept true at the stamped commit — point-in-time notes,
status scratchpads, and completed plans are deleted (git history is the
archive).

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

## 3. Design docs of record — [`design/`](design/)

Internal designs the handbooks delegate to. Their section numbers (§) are cited
from code comments — do not renumber sections.

| File | Topic |
|------|-------|
| [`design/MULTI_MACHINE_122.md`](design/MULTI_MACHINE_122.md) | One seed on several machines: derive-scope partitioning, follow/takeover, the broadcast belt. |
| [`design/STATE_RECONSTRUCTION.md`](design/STATE_RECONSTRUCTION.md) | Chain-truth reconstruction: leg classification, backend tiers, reconcile-before-drive. |
| [`design/NODELESS_WALLET.md`](design/NODELESS_WALLET.md) | The bdk/Electrum wallet: BIP-86 derivation, wallet exclusivity, degradation tiers. |
| [`design/TEST_FRAMEWORK_PLAN.md`](design/TEST_FRAMEWORK_PLAN.md) | The e2e harness design of record (`pact/harness/` delegates here); mainnet-safe port registry. |

## 4. Roadmap & specification

| File | Topic |
|------|-------|
| [`TRADING_ROADMAP.md`](TRADING_ROADMAP.md) | Product strategy and regulatory (MiCA) positioning — not a tech doc the handbooks replace. |

The normative protocol specification and deterministic test vectors live in
[`../spec/`](../spec/) and remain authoritative; the handbooks cite them.
`spec/README.md` also defines the protocol **naming & versioning** (family names
Standard/HTLC and Private/Taproot, wire-id strings, and the per-family wire
epochs the app displays).
