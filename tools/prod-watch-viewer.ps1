<#
.SYNOPSIS
  A second, ISOLATED mainnet Satchel viewer for debugging the board against your
  REAL prod session -- same Nostr relays, SAME KEY (you import it). Unlike
  playground-viewer.ps1 this dir PERSISTS, so your imported seed survives restarts.

.DESCRIPTION
  Runs a second Satchel entirely beside your production install, so it can never
  touch it:
    * Config dir: %LOCALAPPDATA%\org.pocx.satchel-viewer  (its own SATCHEL_DATA_DIR;
      NOT your real %LOCALAPPDATA%\org.pocx.satchel). PERSISTS between runs.
    * Managed pactd on :9747 -- NOT the mainnet default :9737 -- so it can never
      collide with or adopt your prod pactd.
    * NO coins and NO nodes: the board is pure Nostr, so this viewer only
      subscribes to the six default relays (the same set your prod uses). It
      does NOT connect to your pocx/btc nodes at all.

  You set it up once (import your prod mnemonic = same identity), and it stays
  set up. A zero-coin session browses the whole board out of the box, so you can
  drive your PROD session (post / revoke) and watch what this viewer shows.

  Blocks on the Satchel window; close it to tear the viewer down. -Down
  force-tears a stale run.

.PARAMETER Down
  Force-tear a stale run and exit (no setup, no run).

.PARAMETER Reset
  Wipe the viewer's config dir (its seed + setup) and start fresh. Off by
  default so your import persists.

.NOTES
  SAFETY: teardown is PID/PORT-ONLY, scoped to THIS viewer's ports (:9747 pactd,
  :5173 Vite). It NEVER touches :9737 (your prod pactd), never Stop-Process by
  name, and never your live MAINNET pocx-bitcoind. Only -Reset removes the dir.

  With no coins connected, this viewer can't post/take/fund (each is gated on
  having the pair's two coins set up). BUT it holds your PROD key, so it shares
  your identity: on close its exit gate may list your prod board offers and offer
  to withdraw them. Choose "Keep running" or Cancel there -- never "Withdraw &
  exit" -- or you'd revoke your REAL offers from this debug window.
#>
param([switch]$Down, [switch]$Reset)

$ErrorActionPreference = "Stop"
$Repo     = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
# PERSISTENT, isolated config dir -- a sibling of (never) the real install dir.
$ViewerDir = Join-Path $env:LOCALAPPDATA "org.pocx.satchel-viewer"
$LogDir    = Join-Path $Repo ".playground"
$PidFile   = Join-Path $LogDir "viewer-pids.txt"

# ONLY this viewer's ports. :9747 = managed pactd (off the mainnet default :9737);
# :5173 = Vite (cargo tauri dev).
$Ports = 9747, 5173

function Kill-Tree([int]$procId) {
    if ($procId -gt 0) { & cmd /c "taskkill /T /F /PID $procId >nul 2>nul" }
}

function Stop-Viewer {
    if (Test-Path $PidFile) {
        foreach ($line in Get-Content $PidFile) {
            $procId = 0; if ([int]::TryParse($line.Trim(), [ref]$procId)) { Kill-Tree $procId }
        }
        Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
    }
    foreach ($port in $Ports) {
        $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
        foreach ($procId in ($conns.OwningProcess | Sort-Object -Unique)) { Kill-Tree ([int]$procId) }
    }
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline) {
        $busy = $false
        foreach ($port in $Ports) {
            if (Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue) { $busy = $true; break }
        }
        if (-not $busy) { break }
        Start-Sleep -Milliseconds 500
    }
}

if ($Down) {
    Write-Host "[viewer] tearing down (PID + port only) ..."
    Stop-Viewer
    Write-Host "[viewer] down."
    exit 0
}

Write-Host "[viewer] stopping any prior viewer run (PID + port only) ..."
Stop-Viewer

if ($Reset -and (Test-Path $ViewerDir)) {
    Write-Host "[viewer] -Reset: wiping $ViewerDir (seed + setup) ..."
    Remove-Item -Recurse -Force $ViewerDir
}

New-Item -ItemType Directory -Force -Path $ViewerDir | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir    | Out-Null

# Build the managed pactd + pact-cli sidecars this viewer runs.
Write-Host "[viewer] building pactd + pact-cli (debug) ..."
& cargo build --manifest-path (Join-Path $Repo "pact\Cargo.toml") -p pactd -p pact-cli
if ($LASTEXITCODE -ne 0) { throw "cargo build (pactd/pact-cli) failed" }

$pactdPath = (Join-Path $Repo "pact\target\debug\pactd.exe") -replace '\\', '/'

# Write satchel.json ONLY if absent, so your setup persists across runs.
#   * coins [] + no nodes  -> board-only viewer over Nostr (the same six default
#     relays your prod uses; omitting nostr_relays falls back to them).
#   * listen :9747         -> never the prod :9737.
$cfgPath = Join-Path $ViewerDir "satchel.json"
if (-not (Test-Path $cfgPath)) {
    $satchelJson = @"
{
  "pactd_path": "$pactdPath",
  "coins": [],
  "board_urls": [],
  "listen": "127.0.0.1:9747",
  "tick_secs": 30,
  "ui": { "theme": "system", "language": "en", "nav_open": true }
}
"@
    # UTF-8 WITHOUT BOM (a BOM breaks serde_json parsing in pactd).
    [System.IO.File]::WriteAllText($cfgPath, $satchelJson, (New-Object System.Text.UTF8Encoding $false))
    Write-Host "[viewer] wrote fresh satchel.json (:9747, no coins)."
} else {
    # Keep your persisted config, but make sure pactd_path tracks this checkout.
    $cfg = Get-Content $cfgPath -Raw | ConvertFrom-Json
    $cfg.pactd_path = $pactdPath
    [System.IO.File]::WriteAllText($cfgPath, ($cfg | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding $false))
    Write-Host "[viewer] reusing existing satchel.json (your setup persists)."
}

# Stage the Tauri sidecars for the host triple (cargo tauri dev needs them).
$triple = ((rustc -vV) -split "`n" | Where-Object { $_ -like "host:*" }) -replace "host:\s*", ""
$bin = Join-Path $Repo "satchel\binaries"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item (Join-Path $Repo "pact\target\debug\pactd.exe")    (Join-Path $bin "pactd-$triple.exe")    -Force
Copy-Item (Join-Path $Repo "pact\target\debug\pact-cli.exe") (Join-Path $bin "pact-cli-$triple.exe") -Force

$env:SATCHEL_NETWORK  = "mainnet"
$env:SATCHEL_DATA_DIR = $ViewerDir

Write-Host "[viewer] launching Satchel (mainnet, isolated, :9747) ..."
$sat = Start-Process -FilePath "cargo" -ArgumentList "tauri", "dev" `
    -WorkingDirectory (Join-Path $Repo "satchel") `
    -RedirectStandardOutput (Join-Path $LogDir "viewer-satchel.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "viewer-satchel.err.log") `
    -PassThru -WindowStyle Hidden
Set-Content -Path $PidFile -Value $sat.Id

Write-Host ""
Write-Host "======================================================================"
Write-Host "  SATCHEL VIEWER IS UP  (mainnet, isolated, board viewer)"
Write-Host ""
Write-Host "  Config dir: $ViewerDir   (PERSISTS; your prod install is untouched)"
Write-Host "  pactd :9747  |  no nodes, no coins  |  six default Nostr relays"
Write-Host ""
Write-Host "  FIRST RUN -- set it up once:"
Write-Host "    1. Wizard -> Create a merchant (any name)."
Write-Host "    2. Seed -> *** IMPORT *** -> paste your PROD mnemonic (= same key)."
Write-Host "    3. You land straight in the app on the Corkboard, browsing the"
Write-Host "       live board. With no coins, Post/Take nudge you to set coins up."
Write-Host "  After that the seed persists -- rerun this script to reopen the viewer."
Write-Host ""
Write-Host "  TEST: in your PROD Satchel, post then revoke an offer; watch whether"
Write-Host "        it clears here. (Re-tag the script -Reset to start clean.)"
Write-Host ""
Write-Host "  CLOSE THE SATCHEL WINDOW to tear the viewer down (PID/port only)."
Write-Host "  On close, if asked about YOUR offers, choose Keep running / Cancel"
Write-Host "  -- never Withdraw (this shares your prod identity)."
Write-Host "  Logs: $LogDir"
Write-Host "======================================================================"

try {
    $sat.WaitForExit()
} finally {
    Write-Host ""
    Write-Host "[viewer] Satchel closed -- tearing down (PID + port only) ..."
    Stop-Viewer
    Write-Host "[viewer] down."
}
exit 0
