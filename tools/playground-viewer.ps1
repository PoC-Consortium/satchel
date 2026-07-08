<#
.SYNOPSIS
  One-shot Satchel BOARD-VIEWER playground: a mainnet viewer on the default Nostr
  board, in an ISOLATED throwaway config dir. cleanup -> setup -> run.

.DESCRIPTION
  A zero-coin session browses the WHOLE live board out of the box (trading —
  post/take/fund — stays gated per-action until you connect two coins), so this
  is a read-only viewer of the live book. To watch a REAL, populated book it must
  run on mainnet against the default Nostr relays (offers are network-filtered
  client-side, so a regtest viewer on the public relays would see an empty board).

  To keep your real mainnet Satchel untouched, this launches Satchel against an
  isolated config dir via SATCHEL_DATA_DIR:

    * config dir: %LOCALAPPDATA%\org.pocx.satchel-watchpg  (throwaway, wiped on
      setup) -- NOT your real %LOCALAPPDATA%\org.pocx.satchel.
    * satchel.json ships NO coins and OMITS nostr_relays, so Satchel falls back
      to the six RECOMMENDED_NOSTR_RELAYS (spec/protocol.md §8.8) -- the real
      default board.
    * managed pactd on :9747 (NOT the mainnet default :9737), so it can't
      collide with a real mainnet Satchel you may have running.

  No coins, no funds, no bitcoind: this viewer only subscribes to the public
  relays read-only. The Corkboard shows the WHOLE live mainnet book; Post/Take
  are gated (they nudge you to set up two coins) until you connect them.

  The script BLOCKS on the Satchel window: close it and the playground tears
  itself down. -Down force-tears a stale run.

.PARAMETER Down
  Force-tear a stale run and exit (no setup, no run).

.NOTES
  SAFETY: teardown is PID/PORT-ONLY and scoped to THIS playground's ports
  (:9747 managed pactd, :5173 Vite). It NEVER touches :9737 (a real mainnet
  Satchel's pactd), never Stop-Process by name, and never the user's live
  MAINNET pocx-bitcoind. Only the throwaway config dir is wiped.
#>
param([switch]$Down)

$ErrorActionPreference = "Stop"
$Repo    = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path   # repo root (script lives in tools/)
# Isolated, throwaway config dir -- a sibling of (never) the real install dir.
# SATCHEL_DATA_DIR points Satchel (and its managed pactd) here; on mainnet there
# is no per-network subdir, so satchel.json + pactd live directly under it.
$WatchDir = Join-Path $env:LOCALAPPDATA "org.pocx.satchel-watchpg"
$LogDir   = Join-Path $Repo ".playground"
$PidFile  = Join-Path $LogDir "watchpg-pids.txt"

# ONLY this playground's ports. :9747 = our managed pactd (offset off the
# mainnet default :9737 so a real mainnet Satchel is never hit); :5173 = Vite.
$Ports = 9747, 5173

function Kill-Tree([int]$procId) {
    if ($procId -gt 0) { & cmd /c "taskkill /T /F /PID $procId >nul 2>nul" }
}

function Stop-Playground {
    if (Test-Path $PidFile) {
        foreach ($line in Get-Content $PidFile) {
            $procId = 0; if ([int]::TryParse($line.Trim(), [ref]$procId)) { Kill-Tree $procId }
        }
        Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
    }
    foreach ($port in $Ports) {
        $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
        foreach ($procId in ($conns.OwningProcess | Sort-Object -Unique)) {
            Kill-Tree ([int]$procId)
        }
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
    Write-Host "[watch-pg] tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[watch-pg] down."
    exit 0
}

# --- cleanup (this playground only) --------------------------------------
Write-Host "[watch-pg] cleaning up any prior viewer run ..."
Stop-Playground
# Factory-new the throwaway dir so we always start fresh on the default relays.
if (Test-Path $WatchDir) { Remove-Item -Recurse -Force $WatchDir }

# --- setup ---------------------------------------------------------------
New-Item -ItemType Directory -Force -Path $WatchDir | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir   | Out-Null

# Build the managed pactd + pact-cli (sidecars) the viewer will run.
Write-Host "[watch-pg] building pactd + pact-cli (debug) ..."
& cargo build --manifest-path (Join-Path $Repo "pact\Cargo.toml") -p pactd -p pact-cli
if ($LASTEXITCODE -ne 0) { throw "cargo build (pactd/pact-cli) failed" }

$pactdPath = (Join-Path $Repo "pact\target\debug\pactd.exe") -replace '\\', '/'
# NO coins (a pure board viewer) and NO nostr_relays key: the container-level
# #[serde(default)] fills the omitted field from Config::default -> the six
# RECOMMENDED_NOSTR_RELAYS. board_urls empty (corkboard off; Nostr only).
# listen :9747 keeps us off a real mainnet Satchel's :9737.
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
# UTF-8 WITHOUT BOM -- a BOM would break serde_json parsing in pactd.
[System.IO.File]::WriteAllText(
    (Join-Path $WatchDir "satchel.json"), $satchelJson,
    (New-Object System.Text.UTF8Encoding $false))

# Stage the Tauri sidecars for the host triple (cargo tauri dev needs them).
$triple = ((rustc -vV) -split "`n" | Where-Object { $_ -like "host:*" }) -replace "host:\s*", ""
$bin = Join-Path $Repo "satchel\binaries"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item (Join-Path $Repo "pact\target\debug\pactd.exe")    (Join-Path $bin "pactd-$triple.exe")    -Force
Copy-Item (Join-Path $Repo "pact\target\debug\pact-cli.exe") (Join-Path $bin "pact-cli-$triple.exe") -Force

# --- run -----------------------------------------------------------------
# Mainnet (no SATCHEL_NETWORK override needed, but explicit is clearer) +
# the isolated config dir. Satchel's managed pactd inherits both.
$env:SATCHEL_NETWORK  = "mainnet"
$env:SATCHEL_DATA_DIR = $WatchDir

Write-Host "[watch-pg] launching Satchel (mainnet board viewer, isolated config) ..."
$sat = Start-Process -FilePath "cargo" -ArgumentList "tauri", "dev" `
    -WorkingDirectory (Join-Path $Repo "satchel") `
    -RedirectStandardOutput (Join-Path $LogDir "watch-satchel.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "watch-satchel.err.log") `
    -PassThru -WindowStyle Hidden
Set-Content -Path $PidFile -Value $sat.Id

Write-Host ""
Write-Host "======================================================================"
Write-Host "  SATCHEL BOARD-VIEWER PLAYGROUND IS UP  (mainnet, default Nostr board)"
Write-Host ""
Write-Host "  Isolated config: $WatchDir"
Write-Host "  Your real mainnet Satchel config is UNTOUCHED. pactd on :9747."
Write-Host "  Board: the six default Nostr relays (no coins, no funds)."
Write-Host ""
Write-Host "  In the window:"
Write-Host "    1. Wizard -> Create a merchant (any name)."
Write-Host "    2. Seed -> 'Create new' -> ack -> confirm the 3 words (mainnet"
Write-Host "       does NOT skip verify) -> 'No passphrase' -> Done."
Write-Host "       (Throwaway seed: with no coins it never holds funds.)"
Write-Host "    3. You land straight in the app on the Corkboard, browsing the"
Write-Host "       WHOLE live public board; the header relay dot shows x/6 connected."
Write-Host "    4. With no coins set up, Post/Take nudge you to connect two coins."
Write-Host ""
Write-Host "  CLOSE THE SATCHEL WINDOW to tear the playground down."
Write-Host "  Logs:  $LogDir"
Write-Host "======================================================================"

try {
    $sat.WaitForExit()
} finally {
    Write-Host ""
    Write-Host "[watch-pg] Satchel closed -- tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[watch-pg] down."
}
exit 0
