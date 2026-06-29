# Satchel — User Handbook — build script
#
# Combines metadata.yaml + chapters/*.md into satchel-handbook.pdf.
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
    "chapters/ch02-what-is-satchel.md",
    "chapters/ch03-installing.md",
    "chapters/ch04-first-launch.md",
    "chapters/ch05-setting-up-coins.md",
    "chapters/part2.md",
    "chapters/ch06-tour.md",
    "chapters/ch07-corkboard.md",
    "chapters/ch08-making-an-offer.md",
    "chapters/ch09-taking-an-offer.md",
    "chapters/ch10-tracking-swaps.md",
    "chapters/ch11-wallets.md",
    "chapters/ch12-contacts.md",
    "chapters/ch13-private-offers.md",
    "chapters/part3.md",
    "chapters/ch14-transports.md",
    "chapters/ch15-settings.md",
    "chapters/part4.md",
    "chapters/ch16-safety.md",
    "chapters/ch17-understanding-swaps.md",
    "chapters/ch18-troubleshooting.md",
    "chapters/ch19-faq.md",
    "chapters/ch20-glossary.md",
    "chapters/ch21-help.md"
)

$missing = $inputs | Where-Object { -not (Test-Path $_) }
if ($missing) {
    Write-Error "Missing input file(s): $($missing -join ', ')"
}

if (-not (Get-Command pandoc -ErrorAction SilentlyContinue)) {
    Write-Error "pandoc was not found in PATH. Install it from https://pandoc.org/installing.html"
}

& pandoc $inputs `
    --output ../satchel-handbook.pdf `
    --pdf-engine=xelatex `
    --toc `
    --toc-depth=2 `
    --number-sections `
    --top-level-division=chapter `
    --resource-path=.

if ($LASTEXITCODE -ne 0) {
    Write-Error "pandoc exited with code $LASTEXITCODE"
}

Write-Host "Built ../satchel-handbook.pdf"
