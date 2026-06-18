<#
.SYNOPSIS
  One-shot Satchel NOSTR (relays-only) playground: cleanup -> setup -> run.

.DESCRIPTION
  Like playground-cork.ps1, but with NO corkboard -- Bob, Carol and the managed
  Satchel "Alice" trade over a single LOCAL Nostr relay only. This proves offers
  flow over Nostr alone (the demo's target config, corkboard server dropped); a
  broken relay shows an empty board rather than a false pass.

    * regtest PoCX + BTC nodes, a local Nostr relay (ephemeral, wiped on
      teardown), and two headless counterparties posting a two-sided book over
      the relay with a short (~5 min) offer TTL.
    * Satchel launched as managed "Alice" with a RELAYS-ONLY satchel.json
      (nostr_relays set, board_urls empty), factory-new data dir.

  The script then BLOCKS on the Satchel window (like the bundled demo runner):
  close the window and the whole stack — including the ephemeral relay — tears
  itself down automatically. -Down is only needed to force-tear a stale run.

.PARAMETER RelayCmd
  Launch command for the local Nostr relay, with {port}/{dir} substituted.
  Defaults to the bundled nostr-rs-relay in pact\harness\bin. Examples:
    -RelayCmd "mini-relay --listen 127.0.0.1:{port}"
  May also be supplied via the PACT_NOSTR_RELAY_CMD environment variable.

.PARAMETER Down
  Force-tear a stale run and exit (no setup, no run). Not needed in the normal
  flow — closing the Satchel window already tears everything down.

.PARAMETER FirstRun
  Ship Satchel with NO coins pre-wired, so the first-run onboarding + coin-setup
  (the >=2-live-coins gate) runs and you configure the coins yourself. The
  startup banner prints the playground nodes' connection details to enter in the
  form (user/pass auth, NOT cookie).

.NOTES
  SAFETY: teardown is PID/PORT-ONLY (see playground-cork.ps1). We NEVER Stop-Process
  by name -- the user runs a live MAINNET pocx-bitcoind. None of these ports is
  the mainnet node.
#>
param([string]$RelayCmd, [switch]$Down, [switch]$FirstRun)

$ErrorActionPreference = "Stop"
$Repo    = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path   # repo root (script now lives in tools/)
$AppData = Join-Path $env:APPDATA "org.pocx.satchel"
# Satchel nests test networks in a per-network subdir (Bitcoin-Core style); this
# is a regtest playground, so all its state lives under <config>/regtest and we
# launch Satchel with SATCHEL_NETWORK=regtest.
$NetData = Join-Path $AppData "regtest"
$LogDir  = Join-Path $Repo ".playground"
$PidFile = Join-Path $LogDir "pids.txt"

# Managed pactd (:9737), Bob/Carol pactd (:19737/8) + spares (:19739/40),
# PoCX/BTC/LTC regtest RPC (:19443/:19543/:19643), local Nostr relay (:19788),
# stale corkboard (:19790, in case the other playground left one), Vite (:5173).
$Ports = 9737, 19737, 19738, 19739, 19740, 19443, 19543, 19643, 19788, 19790, 5173

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
    # Orphan harness orchestrator, matched by SCRIPT PATH (never a generic name):
    # on its way out its Harness cleanup sends `stop` to :19443/:19543, which
    # would hit the FRESH nodes we are about to start.
    Get-CimInstance Win32_Process -Filter "Name = 'python.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -and $_.CommandLine -match 'satchel_playground_nostr\.py' } |
        ForEach-Object { Kill-Tree ([int]$_.ProcessId) }
    foreach ($port in $Ports) {
        $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
        foreach ($procId in ($conns.OwningProcess | Sort-Object -Unique)) {
            Kill-Tree ([int]$procId)
        }
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
    Write-Host "[nostr-pg] tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[nostr-pg] down."
    exit 0
}

# Relay: defaults to the bundled nostr-rs-relay (pact\harness\bin), launched with
# a generated config by the harness. -RelayCmd overrides with a full launch
# command ({port}/{dir} substituted).
if ($RelayCmd) { $env:PACT_NOSTR_RELAY_CMD = $RelayCmd }
$bundledRelay = Join-Path $Repo "pact\harness\bin\nostr-rs-relay.exe"
if (-not $env:PACT_NOSTR_RELAY_CMD -and -not (Test-Path $bundledRelay)) {
    Write-Host "[nostr-pg] No relay available: bundled nostr-rs-relay missing at" -ForegroundColor Yellow
    Write-Host "           $bundledRelay"
    Write-Host "           Build/copy it, or pass -RelayCmd '<cmd with {port}/{dir}>'."
    exit 1
}

# --- cleanup -------------------------------------------------------------
Write-Host "[nostr-pg] cleaning up any prior run ..."
Stop-Playground

# --- setup ---------------------------------------------------------------
# Factory-new Alice + a RELAYS-ONLY satchel.json (board_urls empty so the book
# can only come from Nostr). Everything lives under the regtest subdir ($NetData).
New-Item -ItemType Directory -Force -Path $NetData | Out-Null
$pactdState = Join-Path $NetData "pactd"
if (Test-Path $pactdState) { Remove-Item -Recurse -Force $pactdState }

$pactdPath = (Join-Path $Repo "pact\target\debug\pactd.exe") -replace '\\', '/'
# -FirstRun ships NO coins so Satchel's first-run coin-setup (the >=2-live-coins
# gate) runs and you wire btcx/btc/ltc yourself; otherwise all three are
# pre-wired so Alice is ready to trade immediately. (Single-quoted JSON line:
# the `@` in the RPC URLs must stay literal.)
$coinsJson = if ($FirstRun) { '[]' } else {
  '[{ "coin_id": "btcx", "chain_data": "http://pactharness:pactharness@127.0.0.1:19443/wallet/alice_pocx", "funding_wallet": "core-rpc" }, { "coin_id": "btc", "chain_data": "http://pactharness:pactharness@127.0.0.1:19543/wallet/alice_btc", "funding_wallet": "core-rpc" }, { "coin_id": "ltc", "chain_data": "http://pactharness:pactharness@127.0.0.1:19643/wallet/alice_ltc", "funding_wallet": "core-rpc" }]'
}
$satchelJson = @"
{
  "pactd_path": "$pactdPath",
  "coins": $coinsJson,
  "board_urls": [],
  "nostr_relays": ["ws://127.0.0.1:19788"],
  "listen": "127.0.0.1:9737",
  "auto_fund": true,
  "tick_secs": 2,
  "ui": { "theme": "system", "language": "en", "nav_open": true }
}
"@
# UTF-8 WITHOUT BOM -- a BOM would break serde_json parsing in pactd.
[System.IO.File]::WriteAllText(
    (Join-Path $NetData "satchel.json"), $satchelJson,
    (New-Object System.Text.UTF8Encoding $false))

# LTC is a file-added coin: ship coins.toml (consensus params) + its icon next
# to satchel.json so both Satchel and its managed pactd resolve the `ltc`
# template (and the Coins/Wallet cards show the Litecoin badge). Harmless in
# -FirstRun too -- it just makes `ltc` available in the setup picker.
Copy-Item (Join-Path $Repo "satchel\coins.toml") (Join-Path $NetData "coins.toml") -Force
Copy-Item (Join-Path $Repo "satchel\ltc.svg")    (Join-Path $NetData "ltc.svg")    -Force

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

$env:POCX_BITCOIND = Join-Path $Repo "pact\harness\bin\pocx-bitcoind.exe"
$env:BTC_BITCOIND  = Join-Path $Repo "pact\harness\bin\btc-bitcoind.exe"
$env:LITECOIND     = Join-Path $Repo "pact\harness\bin\litecoind.exe"
# Bots' pactd logs at debug (stderr -> pact-<name>/pactd.log) so we can see the
# Nostr publish/fetch path while testing.
if (-not $env:RUST_LOG) { $env:RUST_LOG = "pactd=debug,libswap=debug" }

# --- run -----------------------------------------------------------------
Write-Host "[nostr-pg] starting regtest stack + local relay + Bob/Carol (building if needed) ..."
$pg = Start-Process -FilePath "python" -ArgumentList "satchel_playground_nostr.py" `
    -WorkingDirectory (Join-Path $Repo "pact\harness") `
    -RedirectStandardOutput (Join-Path $LogDir "nostr-playground.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "nostr-playground.err.log") `
    -PassThru -WindowStyle Hidden
Set-Content -Path $PidFile -Value $pg.Id

$pgOut = Join-Path $LogDir "nostr-playground.out.log"
$deadline = (Get-Date).AddMinutes(5)
$up = $false
while ((Get-Date) -lt $deadline) {
    if ($pg.HasExited) {
        throw "nostr playground exited early (code $($pg.ExitCode)). See $LogDir\nostr-playground.err.log"
    }
    if ((Test-Path $pgOut) -and (Select-String -Path $pgOut -Pattern "PLAYGROUND IS UP" -Quiet -ErrorAction SilentlyContinue)) {
        $up = $true; break
    }
    Start-Sleep -Seconds 1
}
if (-not $up) { throw "nostr playground did not come up within 5 min. See $pgOut" }
Write-Host "[nostr-pg] stack up; launching Satchel ..."

# Stage the Tauri sidecars (same as playground-cork.ps1 -- cargo tauri dev needs a
# binary for the host triple to exist).
$triple = ((rustc -vV) -split "`n" | Where-Object { $_ -like "host:*" }) -replace "host:\s*", ""
$bin = Join-Path $Repo "satchel\binaries"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item (Join-Path $Repo "pact\target\debug\pactd.exe")    (Join-Path $bin "pactd-$triple.exe")    -Force
Copy-Item (Join-Path $Repo "pact\target\debug\pact-cli.exe") (Join-Path $bin "pact-cli-$triple.exe") -Force

# SATCHEL_NETWORK selects regtest (nests config under <config>/regtest); forwarding
# a -regtest flag through `cargo tauri dev` is awkward, so the env var is cleaner.
$env:SATCHEL_NETWORK = "regtest"
$sat = Start-Process -FilePath "cargo" -ArgumentList "tauri", "dev" `
    -WorkingDirectory (Join-Path $Repo "satchel") `
    -RedirectStandardOutput (Join-Path $LogDir "satchel.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "satchel.err.log") `
    -PassThru -WindowStyle Hidden
Add-Content -Path $PidFile -Value $sat.Id

Write-Host ""
Write-Host "======================================================================"
Write-Host "  SATCHEL NOSTR (RELAYS-ONLY) PLAYGROUND IS UP"
Write-Host ""
Write-Host "  No corkboard - the book flows over one local Nostr relay (:19788)."
Write-Host "  Default offer TTL; the relay is wiped on teardown (ephemeral)."
Write-Host ""
if ($FirstRun) {
    Write-Host "  FIRST-RUN: no coins pre-wired -> step through onboarding + coin"
    Write-Host "  setup. Configure the coins against the playground nodes:"
    Write-Host "    BTCX : 127.0.0.1:19443  user/pass  pactharness / pactharness  wallet alice_pocx"
    Write-Host "    BTC  : 127.0.0.1:19543  user/pass  pactharness / pactharness  wallet alice_btc"
    Write-Host "    LTC  : 127.0.0.1:19643  user/pass  pactharness / pactharness  wallet alice_ltc"
    Write-Host "    (auth = user/pass, NOT cookie; confirmations blank = regtest default 1)"
} else {
    Write-Host "  In the window: wizard -> create merchant; Coins tab shows all"
    Write-Host "  three connected; Corkboard -> offers come from Nostr; take any"
    Write-Host "  side (incl. LTC)."
}
Write-Host ""
Write-Host "  CLOSE THE SATCHEL WINDOW to tear the whole stack down."
Write-Host "  Logs:  $LogDir"
Write-Host "======================================================================"

# Block on Satchel like the demo runner does: when the window closes (cargo
# tauri dev exits with it), tear the whole stack down -- including the ephemeral
# relay. The finally{} also fires on Ctrl-C, so there is no separate cleanup
# step. Teardown stays PID/port-only.
try {
    $sat.WaitForExit()
} finally {
    Write-Host ""
    Write-Host "[nostr-pg] Satchel closed -- tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[nostr-pg] down."
}
exit 0
