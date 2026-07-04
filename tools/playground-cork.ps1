<#
.SYNOPSIS
  One-shot Satchel v2 playground: cleanup -> setup -> run.

.DESCRIPTION
  Brings up the full regtest stack for click-testing v2 (Taproot/MuSig2 adaptor)
  swaps in Satchel:

    * regtest PoCX + BTC nodes, a Corkboard, and two headless counterparties
      (Bob = buy side, Carol = sell side) posting a two-sided book. On a
      PoCX<->BTC pair off-mainnet every board offer defaults to pact-htlc-v2,
      so the whole book is v2.
    * Satchel launched as managed "Alice" (owns its own regtest pactd on :9739,
      NOT the mainnet :9737 — kept distinct so teardown never hits a live daemon),
      funded on BOTH coins, with a factory-new data dir so a stale relay
      cursor never makes Alice miss the fresh board's offers.

  You click; the swaps auto-complete and show on the Swaps tab badged
  "Private (Taproot)". Logs land in <repo>\.playground\ — check those if
  anything misbehaves.

  The script then BLOCKS on the Satchel window (like the bundled demo runner):
  close the window and the whole stack tears itself down automatically — no
  separate cleanup step. -Down is only needed to force-tear a stale/orphaned run.

.PARAMETER Down
  Force-tear a stale run and exit (no setup, no run). Not needed in the normal
  flow — closing the Satchel window already tears everything down.

.PARAMETER FirstRun
  Ship Satchel with NO coins pre-wired, so the first-run onboarding + coin-setup
  (the >=2-live-coins gate) runs and you configure the coins yourself. The
  startup banner prints the playground nodes' connection details to enter in the
  form (user/pass auth, NOT cookie). Mirrors playground-nostr.ps1 -FirstRun.

.NOTES
  SAFETY: teardown is PID/PORT-ONLY. We kill the process trees we started and,
  as a backstop, whatever still listens on the playground ports. We NEVER
  Stop-Process by name — the user runs a live MAINNET pocx-bitcoind and a
  name-based kill would take it down. The mainnet node is not on any of these
  ports.
#>
param([switch]$Down, [switch]$FirstRun)

$ErrorActionPreference = "Stop"
$Repo    = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path   # repo root (script now lives in tools/)
$AppData = Join-Path $env:LOCALAPPDATA "org.pocx.satchel"
# Satchel nests test networks in a per-network subdir (Bitcoin-Core style); this
# is a regtest playground, so all its state lives under <config>/regtest and we
# launch Satchel with SATCHEL_NETWORK=regtest.
$NetData = Join-Path $AppData "regtest"
$LogDir  = Join-Path $Repo ".playground"
$PidFile = Join-Path $LogDir "pids.txt"

# Managed pactd (:9739 — Satchel offsets the listen port per network: regtest
# 9739, testnet 9738, MAINNET 9737. We launch with SATCHEL_NETWORK=regtest, so
# OUR pactd is 9739. NEVER list 9737/9738 here — those are the user's mainnet /
# testnet pactd, and the port teardown would kill their live daemon.)
# Bob/Carol pactd (:19737/8) + adaptor spares (:19739/40),
# PoCX/BTC/LTC regtest RPC (:19443/:19543/:19643), Corkboard (:19790), Vite (:5173).
# (18443/19750/19751 belong to the playground-nodeless variant; swept here
# too so a stale electrum run can't poison a cork run.)
$Ports = 9739, 19737, 19738, 19739, 19740, 19443, 18443, 19543, 19643, 19750, 19751, 19790, 5173

# Force-kill a process tree by PID. Routes through `cmd /c ... 2>nul` so the
# native stderr ("process not found" for an already-dead PID) is swallowed by
# cmd and never reaches PowerShell -- under $ErrorActionPreference='Stop' a
# native stderr line is otherwise promoted to a terminating error.
function Kill-Tree([int]$procId) {
    if ($procId -gt 0) { & cmd /c "taskkill /T /F /PID $procId >nul 2>nul" }
}

function Stop-Playground {
    # 1. Kill the process trees we started (by recorded PID, never by name).
    #    /T takes the children too: python -> nodes/board/pactd,
    #    cargo -> vite + satchel -> managed pactd.
    if (Test-Path $PidFile) {
        foreach ($line in Get-Content $PidFile) {
            $procId = 0; if ([int]::TryParse($line.Trim(), [ref]$procId)) { Kill-Tree $procId }
        }
        Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
    }
    # 2. Kill any orphan harness orchestrator, matched by its SCRIPT PATH (not a
    #    generic process name). It has no listening port of its own, so the port
    #    backstop below can't see it -- yet on its way out its Harness cleanup
    #    sends a `stop` RPC to :19443/:19543, which would land on the FRESH nodes
    #    we are about to start. Matching satchel_playground.py is safe: it can
    #    only ever be our driver, never the mainnet node.
    Get-CimInstance Win32_Process -Filter "Name = 'python.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -and $_.CommandLine -match 'satchel_playground\.py' } |
        ForEach-Object { Kill-Tree ([int]$_.ProcessId) }
    # 3. Backstop: kill anything still LISTENing on a playground port (by PID).
    #    Covers orphans from a crashed prior run. None of these ports is the
    #    user's mainnet node, so this is safe.
    foreach ($port in $Ports) {
        $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
        foreach ($procId in ($conns.OwningProcess | Sort-Object -Unique)) {
            Kill-Tree ([int]$procId)
        }
    }
    # 4. Wait for the ports to actually release (a force-killed node holds its
    #    socket briefly). Starting a fresh node before :19543 is free races the
    #    bind + lets a dying orphan's `stop` hit the new node.
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
    Write-Host "[playground] tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[playground] down."
    exit 0
}

# --- cleanup -------------------------------------------------------------
Write-Host "[playground] cleaning up any prior run ..."
Stop-Playground

# --- setup ---------------------------------------------------------------
# Factory-new Alice: wipe the managed pactd state (seed/db/relay cursor) and
# (re)write a known-good satchel.json so the run is reproducible from scratch.
# Everything lives under the regtest network subdir ($NetData).
New-Item -ItemType Directory -Force -Path $NetData | Out-Null
$pactdState = Join-Path $NetData "pactd"
if (Test-Path $pactdState) { Remove-Item -Recurse -Force $pactdState }

$pactdPath = (Join-Path $Repo "pact\target\debug\pactd.exe") -replace '\\', '/'
# -FirstRun ships NO coins so Satchel's first-run coin-setup (the >=2-live-coins
# gate) runs and you wire btcx/btc/ltc yourself; otherwise all three are
# pre-wired so Alice is ready to trade immediately. (Single-quoted JSON line:
# the `@` in the RPC URLs must stay literal.)
$coinsJson = if ($FirstRun) { '[]' } else {
  '[{ "coin_id": "btcx", "chain_data": "http://pactharness:pactharness@127.0.0.1:19443/wallet/alice_pocx", "funding_wallet": "core-rpc", "confirmations": 10 }, { "coin_id": "btc", "chain_data": "http://pactharness:pactharness@127.0.0.1:19543/wallet/alice_btc", "funding_wallet": "core-rpc", "confirmations": 6 }, { "coin_id": "ltc", "chain_data": "http://pactharness:pactharness@127.0.0.1:19643/wallet/alice_ltc", "funding_wallet": "core-rpc", "confirmations": 6 }]'
}
$satchelJson = @"
{
  "pactd_path": "$pactdPath",
  "coins": $coinsJson,
  "board_urls": ["http://127.0.0.1:19790"],
  "nostr_relays": [],
  "listen": "127.0.0.1:9739",
  "auto_fund": true,
  "tick_secs": 5,
  "ui": { "theme": "system", "language": "en", "nav_open": true }
}
"@
# UTF-8 WITHOUT BOM — a BOM would break serde_json parsing in pactd.
[System.IO.File]::WriteAllText(
    (Join-Path $NetData "satchel.json"), $satchelJson,
    (New-Object System.Text.UTF8Encoding $false))

# Refresh the coin templates next to satchel.json so BOTH Satchel and its
# managed pactd resolve the `ltc` consensus params (a stale prior copy would
# lack ltc and the LTC leg would fail genesis verification). The icon rides
# along so the Coins/Wallet cards show the Litecoin badge.
Copy-Item (Join-Path $Repo "satchel\coins.toml") (Join-Path $NetData "coins.toml") -Force
Copy-Item (Join-Path $Repo "satchel\ltc.svg")    (Join-Path $NetData "ltc.svg")    -Force

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# Pin the regtest node binaries (the harness also auto-resolves harness\bin,
# but absolute paths work regardless of cwd).
$env:POCX_BITCOIND = Join-Path $Repo "pact\harness\bin\pocx-bitcoind.exe"
$env:BTC_BITCOIND  = Join-Path $Repo "pact\harness\bin\btc-bitcoind.exe"
$env:LITECOIND     = Join-Path $Repo "pact\harness\bin\litecoind.exe"

# --- run -----------------------------------------------------------------
# Infra + Bob + Carol. build_workspace() inside it builds pactd.exe, so by the
# time the banner prints, the binary Satchel spawns exists.
Write-Host "[playground] starting regtest stack + Bob/Carol (building if needed) ..."
$pg = Start-Process -FilePath "python" -ArgumentList "satchel_playground.py" `
    -WorkingDirectory (Join-Path $Repo "pact\harness") `
    -RedirectStandardOutput (Join-Path $LogDir "playground.out.log") `
    -RedirectStandardError  (Join-Path $LogDir "playground.err.log") `
    -PassThru -WindowStyle Hidden
Set-Content -Path $PidFile -Value $pg.Id

# Wait for the infra to be fully up before launching Satchel, so the managed
# pactd connects to live nodes + a populated board on first probe.
$pgOut = Join-Path $LogDir "playground.out.log"
$deadline = (Get-Date).AddMinutes(5)
$up = $false
while ((Get-Date) -lt $deadline) {
    if ($pg.HasExited) {
        throw "playground exited early (code $($pg.ExitCode)). See $LogDir\playground.err.log"
    }
    if ((Test-Path $pgOut) -and (Select-String -Path $pgOut -Pattern "PLAYGROUND IS UP" -Quiet -ErrorAction SilentlyContinue)) {
        $up = $true; break
    }
    Start-Sleep -Seconds 1
}
if (-not $up) { throw "playground did not come up within 5 min. See $pgOut" }
Write-Host "[playground] regtest stack up; launching Satchel ..."

# Stage the Tauri sidecars: tauri.conf now declares pactd + pact-cli as
# externalBin, so `cargo tauri dev` refuses to start unless a binary exists for
# the host target triple. The debug binaries were just built by build_workspace;
# copy them to satchel/binaries/<name>-<triple>.exe. (At runtime the playground's
# satchel.json still points pactd_path at the absolute debug pactd, so this copy
# only satisfies the build — CI stages the release binaries the same way.)
$triple = ((rustc -vV) -split "`n" | Where-Object { $_ -like "host:*" }) -replace "host:\s*", ""
$bin = Join-Path $Repo "satchel\binaries"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item (Join-Path $Repo "pact\target\debug\pactd.exe")    (Join-Path $bin "pactd-$triple.exe")    -Force
Copy-Item (Join-Path $Repo "pact\target\debug\pact-cli.exe") (Join-Path $bin "pact-cli-$triple.exe") -Force

# Satchel (cargo tauri dev: Vite from source + the Tauri window + managed pactd).
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
Write-Host "  SATCHEL v2 PLAYGROUND IS UP"
Write-Host ""
Write-Host "  Corkboard book mixes HTLC (v1) + Private/Taproot (v2) offers."
Write-Host "  The Satchel window will open shortly (first build can take a bit)."
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
    Write-Host "  three connected; Corkboard -> take any side (incl. LTC); Swaps"
    Write-Host "  tab walks it to 'completed', badged 'Private (Taproot)'."
}
Write-Host ""
Write-Host "  CLOSE THE SATCHEL WINDOW to tear the whole stack down."
Write-Host "  Logs:  $LogDir"
Write-Host "======================================================================"

# Block on Satchel like the demo runner does: when the window closes (cargo
# tauri dev exits with it), tear the whole stack down. The finally{} also fires
# on Ctrl-C, so there is no separate cleanup step. Teardown stays PID/port-only.
try {
    $sat.WaitForExit()
} finally {
    Write-Host ""
    Write-Host "[playground] Satchel closed -- tearing down (PID + port only) ..."
    Stop-Playground
    Write-Host "[playground] down."
}
exit 0
