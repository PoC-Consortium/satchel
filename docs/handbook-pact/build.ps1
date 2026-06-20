# Pact — Developer & Integrator Handbook — build script
#
# Combines metadata.yaml + chapters/*.md into pact-handbook.pdf.
# Reorder, add, or remove chapters by editing the $inputs list below.
#
# Requirements:
#   - Pandoc        https://pandoc.org/installing.html
#   - xelatex       MiKTeX (Windows), MacTeX (macOS), or TeX Live (Linux)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# Files are passed to Pandoc in this exact order.
$inputs = @(
    "metadata.yaml",
    "chapters/front-copyright.md",
    "chapters/part1.md",
    "chapters/ch01-about.md",
    "chapters/ch02-what-is-pact.md",
    "chapters/ch03-architecture.md",
    "chapters/part2.md",
    "chapters/ch04-building.md",
    "chapters/ch05-running-pactd.md",
    "chapters/ch06-configuring-coins.md",
    "chapters/ch07-seed-and-wallet.md",
    "chapters/ch08-pact-cli.md",
    "chapters/part3.md",
    "chapters/ch09-rpc-conventions.md",
    "chapters/ch10-api-node-wallet-coins.md",
    "chapters/ch11-api-swaps-v1.md",
    "chapters/ch12-api-swaps-v2.md",
    "chapters/ch13-api-board-offers-fees.md",
    "chapters/part4.md",
    "chapters/ch14-swap-lifecycle.md",
    "chapters/ch15-htlc-v1.md",
    "chapters/ch16-adaptor-v2.md",
    "chapters/ch17-timelocks-margins.md",
    "chapters/ch18-fees-refunds.md",
    "chapters/ch19-safety-gating.md",
    "chapters/part5.md",
    "chapters/ch20-noticeboard.md",
    "chapters/ch21-wire-format.md",
    "chapters/ch22-nostr.md",
    "chapters/ch23-corkboard.md",
    "chapters/ch24-private-offers.md",
    "chapters/part6.md",
    "chapters/ch25-integrating.md",
    "chapters/ch26-testing-harness.md",
    "chapters/ch27-reference.md"
)

$missing = $inputs | Where-Object { -not (Test-Path $_) }
if ($missing) {
    Write-Error "Missing input file(s): $($missing -join ', ')"
}

if (-not (Get-Command pandoc -ErrorAction SilentlyContinue)) {
    Write-Error "pandoc was not found in PATH. Install it from https://pandoc.org/installing.html"
}

& pandoc $inputs `
    --output ../pact-handbook.pdf `
    --pdf-engine=xelatex `
    --toc `
    --toc-depth=2 `
    --number-sections `
    --top-level-division=chapter `
    --resource-path=.

if ($LASTEXITCODE -ne 0) {
    Write-Error "pandoc exited with code $LASTEXITCODE"
}

Write-Host "Built ../pact-handbook.pdf"
