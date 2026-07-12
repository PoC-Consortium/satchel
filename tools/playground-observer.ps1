<#
.SYNOPSIS
  Observer playground: a MAIN Satchel drives make/take swaps and a second
  OBSERVER Satchel on the SAME seed follows them read-only. Watch the two docks
  track in lockstep (narrate story + progress line), then the followed row
  self-purges when the swap settles.

.DESCRIPTION
  Launches, over one local Nostr relay, NODELESS (both coins Electrum so every
  leg is history-classifiable and the observer resolves whichever side redeems):

    * regtest PoCX + BTC nodes, two electrs, a local Nostr relay, and two
      node-backed counterparties (Bob/Carol) that post a book AND auto-take
      Alice's own offers (so you can MAKE, not just take).  [observer_playground.py]
    * MAIN Satchel "Alice"      - managed pactd :9739, default config dir.
    * OBSERVER Satchel           - managed pactd :9740, ISOLATED config dir via
      SATCHEL_DATA_DIR (%LOCALAPPDATA%\org.pocx.satchel-observer). Same seed
      (imported in the wizard) -> sees Alice's snapshots; own data dir -> own
      machine.json scope -> Alice's swaps read source=foreign and it FOLLOWS.

  Both windows run the BUILT satchel.exe (frontendDist = ui/dist), so there is
  no vite dev-server and the two GUIs don't collide. Each Satchel spawns its OWN
  managed pactd from its satchel.json; you seed both in the wizard with one fixed
  mnemonic and the driver then funds Alice's (seed-derived, shared) wallets.

  Close the MAIN window to tear the whole stack (both Satchels + nodes + relay
  + electrs) down. -Down force-tears a stale run.

.NOTES
  SAFETY: teardown is PID/PORT-ONLY - never Stop-Process by name, so a live
  MAINNET pocx-bitcoind / Satchel is untouched. None of these ports is mainnet.
#>
param([switch]$Down, [switch]$NoBuild)

$ErrorActionPreference = "Stop"
$Repo    = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$AppBase = Join-Path $env:LOCALAPPDATA "org.pocx.satchel"           # MAIN (Alice)
$ObsBase = Join-Path $env:LOCALAPPDATA "org.pocx.satchel-observer"  # OBSERVER (isolated)
$AliceNet = Join-Path $AppBase "regtest"
$ObsNet   = Join-Path $ObsBase "regtest"
$LogDir  = Join-Path $Repo ".playground"
$PidFile = Join-Path $LogDir "observer-pids.txt"

# Alice pactd :9739, observer pactd :9740, PoCX/BTC REST nodes :18443/:18332,
# electrs :19750/:19751 (btcx) + :19760/:19761 (btc), relay :19788, plus the
# default regtest RPC ports and vite (in case a prior run left one).
# NEVER 9737/9738 - those are the user's mainnet/testnet pactd.
$Ports = 9739, 9740, 18443, 18332, 19443, 19543, 19750, 19751, 19760, 19761, 19788, 5173

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
    # Orphan driver, matched by SCRIPT PATH (never a generic name): its Harness
    # cleanup would otherwise `stop` the fresh nodes we're about to start.
    Get-CimInstance Win32_Process -Filter "Name = 'python.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -and $_.CommandLine -match 'observer_playground\.py' } |
        ForEach-Object { Kill-Tree ([int]$_.ProcessId) }
    foreach ($port in $Ports) {
        $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
        foreach ($procId in ($conns.OwningProcess | Sort-Object -Unique)) { Kill-Tree ([int]$procId) }
    }
    $deadline = (Get-Date).AddSeconds(20)
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
    Write-Host "[obs-pg] tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[obs-pg] down."
    exit 0
}

Write-Host "[obs-pg] cleaning up any prior run ..."
Stop-Playground

# --- build: pactd + a STANDALONE satchel.exe -------------------------------
# The webview frontend must be EMBEDDED (frontendDist = ui/dist), NOT the vite
# dev URL: a plain `cargo build -p satchel` yields a dev binary that fails with
# ERR_CONNECTION_REFUSED (no vite server). `cargo tauri build` embeds ui/dist
# (running `npm run build` for us via beforeBuildCommand); --debug keeps it in
# target/debug, --no-bundle skips the installer. Two such binaries run side by
# side with no vite dev server.
if (-not $NoBuild) {
    Write-Host "[obs-pg] building pactd (debug) ..." -ForegroundColor Cyan
    Push-Location $Repo
    try {
        cargo build -p pactd --manifest-path pact\Cargo.toml
        if ($LASTEXITCODE -ne 0) { throw "pactd build failed" }
    } finally { Pop-Location }
    Write-Host "[obs-pg] building satchel.exe (cargo tauri build --debug --no-bundle) ..." -ForegroundColor Cyan
    Push-Location (Join-Path $Repo "satchel")
    try {
        cargo tauri build --debug --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "satchel (tauri build) failed" }
    } finally { Pop-Location }
}
$SatchelExe = Join-Path $Repo "satchel\target\debug\satchel.exe"
$PactdExe   = Join-Path $Repo "pact\target\debug\pactd.exe"
if (-not (Test-Path $SatchelExe)) { throw "satchel.exe not found at $SatchelExe (run without -NoBuild)" }
if (-not (Test-Path $PactdExe))   { throw "pactd.exe not found at $PactdExe" }

# Stage the pactd sidecar for the host triple (Satchel resolves the managed
# pactd via pactd_path below, but the sidecar keeps the resource lookup happy).
$triple = ((rustc -vV) -split "`n" | Where-Object { $_ -like "host:*" }) -replace "host:\s*", ""
$bin = Join-Path $Repo "satchel\binaries"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item $PactdExe (Join-Path $bin "pactd-$triple.exe") -Force
Copy-Item (Join-Path $Repo "pact\target\debug\pact-cli.exe") (Join-Path $bin "pact-cli-$triple.exe") -Force

# --- fresh config dirs + satchel.json for BOTH windows ---------------------
foreach ($d in @($AliceNet, $ObsNet)) {
    if (Test-Path (Join-Path $d "pactd")) { Remove-Item -Recurse -Force (Join-Path $d "pactd") }
    New-Item -ItemType Directory -Force -Path $d | Out-Null
}
$pactdPath = $PactdExe -replace '\\', '/'
# NODELESS coins: both legs Electrum (pact-seed wallet), so the observer can
# history-classify whichever side redeems. Mainnet-like confs (match the bots).
$coinsJson = '[{ "coin_id": "btcx", "chain_data": "tcp://127.0.0.1:19750", "funding_wallet": "pact-seed", "confirmations": 6 }, { "coin_id": "btc", "chain_data": "tcp://127.0.0.1:19760", "funding_wallet": "pact-seed", "confirmations": 4 }]'

function Write-SatchelJson([string]$netDir, [int]$listenPort) {
    $json = @"
{
  "pactd_path": "$pactdPath",
  "coins": $coinsJson,
  "board_urls": [],
  "nostr_relays": ["ws://127.0.0.1:19788"],
  "listen": "127.0.0.1:$listenPort",
  "auto_fund": true,
  "tick_secs": 5,
  "ui": { "theme": "system", "language": "en", "nav_open": true, "onboarded": true }
}
"@
    [System.IO.File]::WriteAllText((Join-Path $netDir "satchel.json"), $json,
        (New-Object System.Text.UTF8Encoding $false))
}
Write-SatchelJson $AliceNet 9739
Write-SatchelJson $ObsNet   9740

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$env:POCX_BITCOIND = Join-Path $Repo "pact\harness\bin\pocx-bitcoind.exe"
$env:BTC_BITCOIND  = Join-Path $Repo "pact\harness\bin\btc-bitcoind.exe"
if (-not $env:RUST_LOG) { $env:RUST_LOG = "pactd=info,libswap=info" }

# --- run the stack driver (nodes + electrs + relay + bots + faucet) --------
Write-Host "[obs-pg] starting regtest stack + electrs + relay + Bob/Carol ..." -ForegroundColor Cyan
$pg = Start-Process -FilePath "python" -ArgumentList "observer_playground.py" `
    -WorkingDirectory (Join-Path $Repo "pact\harness") `
    -RedirectStandardOutput (Join-Path $LogDir "observer-playground.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "observer-playground.err.log") `
    -PassThru -WindowStyle Hidden
Set-Content -Path $PidFile -Value $pg.Id

$pgOut = Join-Path $LogDir "observer-playground.out.log"
$deadline = (Get-Date).AddMinutes(5)
$up = $false
while ((Get-Date) -lt $deadline) {
    if ($pg.HasExited) { throw "driver exited early (code $($pg.ExitCode)). See $LogDir\observer-playground.err.log" }
    if ((Test-Path $pgOut) -and (Select-String -Path $pgOut -Pattern "STACK IS UP" -Quiet -ErrorAction SilentlyContinue)) { $up = $true; break }
    Start-Sleep -Seconds 1
}
if (-not $up) { throw "driver did not come up within 5 min. See $pgOut" }
Write-Host "[obs-pg] stack up." -ForegroundColor Green

# --- launch BOTH Satchel windows (built binary, no vite) -------------------
# Each Satchel SPAWNS ITS OWN managed pactd on its configured `listen` port (the
# normal app path: nothing on the port -> spawn from satchel.json). We launch
# them STAGGERED and wait for each backend's /health before the next, so the two
# never race and we fail loudly if a backend doesn't come up. (Do NOT pre-spawn
# pactd here: Satchel only ADOPTS a pactd it detached itself via running-pactd.json
# and REFUSES a foreign one on its port -- "already serving a different engine".)
#
# MAIN "Alice": default config dir. OBSERVER: isolated dir via SATCHEL_DATA_DIR.
# Each also gets its OWN WebView2 user-data folder -- two instances of the same
# exe share one WebView2 dir by default and the SECOND window fails to create its
# webview (folder locked) and renders blank/stuck. Both are playground-local so
# neither touches your real Satchel's WebView2 state. PS 5.1's Start-Process has
# no -Environment, so set env in THIS session just before each launch, then clear.
function Wait-Health([int]$port, [int]$sec = 45) {
    $deadline = (Get-Date).AddSeconds($sec)
    while ((Get-Date) -lt $deadline) {
        try {
            if ((Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -TimeoutSec 2 -UseBasicParsing).StatusCode -eq 200) { return $true }
        } catch {}
        Start-Sleep -Milliseconds 500
    }
    return $false
}

$env:SATCHEL_NETWORK = "regtest"

# MAIN "Alice" - default config dir, own WebView2 folder.
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $AliceNet "webview2"
try {
    $alice = Start-Process -FilePath $SatchelExe -PassThru `
        -RedirectStandardOutput (Join-Path $LogDir "satchel-alice.out.log") `
        -RedirectStandardError  (Join-Path $LogDir "satchel-alice.err.log")
} finally {
    Remove-Item Env:\WEBVIEW2_USER_DATA_FOLDER -ErrorAction SilentlyContinue
}
Add-Content -Path $PidFile -Value $alice.Id
Write-Host "[obs-pg] Alice launched; waiting for its managed pactd on :9739 ..." -ForegroundColor Cyan
if (-not (Wait-Health 9739)) { throw "Alice pactd did not come up on :9739 (see $LogDir\satchel-alice.err.log)" }
Write-Host "[obs-pg] Alice backend up." -ForegroundColor Green

# OBSERVER - isolated config dir (SATCHEL_DATA_DIR) + own WebView2 folder.
$env:SATCHEL_DATA_DIR = $ObsBase
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $ObsBase "webview2"
try {
    $obs = Start-Process -FilePath $SatchelExe -PassThru `
        -RedirectStandardOutput (Join-Path $LogDir "satchel-observer.out.log") `
        -RedirectStandardError  (Join-Path $LogDir "satchel-observer.err.log")
} finally {
    Remove-Item Env:\SATCHEL_DATA_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\WEBVIEW2_USER_DATA_FOLDER -ErrorAction SilentlyContinue
}
Add-Content -Path $PidFile -Value $obs.Id
Write-Host "[obs-pg] Observer launched; waiting for its managed pactd on :9740 ..." -ForegroundColor Cyan
if (-not (Wait-Health 9740)) { throw "Observer pactd did not come up on :9740 (see $LogDir\satchel-observer.err.log)" }
Write-Host "[obs-pg] Observer backend up." -ForegroundColor Green

Write-Host ""
Write-Host "======================================================================"
Write-Host "  OBSERVER PLAYGROUND IS UP"
Write-Host ""
Write-Host "  Two Satchel windows, SAME seed, over one local relay:"
Write-Host "    MAIN 'Alice'  (pactd :9739) - you drive make/take here."
Write-Host "    OBSERVER      (pactd :9740) - follows Alice read-only."
Write-Host ""
Write-Host "  SEED BOTH WINDOWS with the SAME phrase (the wizard runs its own"
Write-Host "  onboarding, so create a merchant then IMPORT - do not generate - in"
Write-Host "  EACH window):"
Write-Host ""
Write-Host "       legal winner thank year wave sausage worth useful legal winner thank yellow" -ForegroundColor Yellow
Write-Host ""
Write-Host "    The driver then auto-funds Alice (watch the log for 'faucet')."
Write-Host "  Same phrase -> same identity -> the observer follows Alice; each"
Write-Host "  window's own data dir gives it a distinct machine scope (foreign)."
Write-Host ""
Write-Host "  TEST (watch BOTH docks track, then the observer's row self-purge):"
Write-Host "    TAKE : on Alice, Corkboard -> take a Bob/Carol offer (v1 AND v2 are"
Write-Host "           posted). Alice is the taker (participant)."
Write-Host "    MAKE : on Alice, post an offer -> a bot auto-takes it. Alice is the"
Write-Host "           maker (initiator)."
Write-Host "  On the OBSERVER window, the same swap appears under 'Another machine'"
Write-Host "  and its narrate + progress should mirror Alice's, step for step."
Write-Host "  You can also 'Take over' on the observer after closing Alice."
Write-Host ""
Write-Host "  Pace: btcx blocks ~8s / btc ~12s, confs 6/4 - states linger to watch."
Write-Host "  CLOSE THE MAIN (Alice) WINDOW to tear the whole stack down."
Write-Host "  Logs:  $LogDir  (driver, satchel-alice, satchel-observer)"
Write-Host "======================================================================"

try {
    $alice.WaitForExit()
} finally {
    Write-Host ""
    Write-Host "[obs-pg] main window closed - tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[obs-pg] down."
}
