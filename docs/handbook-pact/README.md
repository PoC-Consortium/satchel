# Pact — Developer & Integrator Handbook

Source files for the **Pact Developer & Integrator Handbook**. The handbook is
written in Markdown (one file per chapter) and built into a single PDF with
Pandoc + xelatex. It documents the Pact swap engine for the people who run it,
build against it, or implement the protocol independently: `pactd` (the local
JSON-RPC daemon), `pact-cli`, `libswap`, the wire format, and the v1/v2 swap
protocols.

For the end-user desktop app that embeds Pact, see the **Satchel User Handbook**
in `../handbook-satchel/`. The two handbooks are kept in sync.

## Prerequisites

- **Pandoc 3.0 or newer** — <https://pandoc.org/installing.html>
- **A LaTeX distribution** that provides `xelatex`:
    - Windows: [MiKTeX](https://miktex.org/) (auto-installs missing packages on first build)
    - macOS: [MacTeX](https://www.tug.org/mactex/)
    - Linux: `texlive-xetex texlive-fonts-recommended texlive-latex-extra`

## Building the PDF

From this directory:

    .\build.ps1

The output is written to `../pact-handbook.pdf`. The first build under MiKTeX
takes longer because it downloads several LaTeX packages on demand.

## Project layout

    handbook-pact/
    ├── chapters/          one Markdown file per chapter or part divider
    ├── images/processed/  diagrams referenced by the chapters
    ├── metadata.yaml      title, version, fonts, page setup
    ├── style.tex          LaTeX header includes (page numbers, header bar)
    ├── build.ps1          PowerShell build script
    └── README.md          this file

## Editing conventions

- One file per chapter under `chapters/`. Filenames use the prefix `chNN-…` so
  they sort naturally.
- Part dividers (`part1.md`, `part2.md`, …) each contain a single LaTeX
  `\part{…}` command.
- The build script lists every input file explicitly. To add a chapter, append
  it to the `$inputs` array in `build.ps1`.

### Keeping it in sync with the code

This handbook documents an external contract (RPC methods, CLI flags, on-disk
formats, protocol steps). Every such claim should be traceable to source. When
the code changes, update the corresponding chapter. The JSON-RPC API chapters
mirror the daemon's method dispatch; the protocol chapters mirror `spec/` and
`libswap/`.

### Callouts

Callouts are blockquotes with a leading bold tag:

    > **Tip** — A helpful suggestion.

    > **Note** — Useful background information.

    > **Warning** — An action that can cause loss of funds, or a sharp edge.

### Text styles

- **Bold** for component names and emphasis.
- `Monospace` for commands, method names, flags, file paths, and field names.
- *Italics* for new terms when first introduced.
