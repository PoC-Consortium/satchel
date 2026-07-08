<#
  playground-multimachine.ps1 - multi-machine safety (issue #122).

  Runs TWO pactd on the SAME seed in SEPARATE data dirs ("machine A" + "machine
  B") and verifies the seed-scoped partition - the automatable half of the #122
  test matrix (docs/MULTI_MACHINE_122.md, Testing section), needing NO nodes,
  board or Electrum: it only exercises pactd's offline derivation + the data-dir
  lock.

  Automated checks:
    1. Section 0 data-dir lock - a 2nd pactd on machine A's data dir is REFUSED.
    2. Section 1 machine label - A and B derive DISTINCT machine labels
       (distinct machine.json scopes).
    3. Section 1 partition - the SAME offer built on A and B yields DISTINCT
       swap_id + distinct nonzero derive_scope (so preimage, keys and relay
       coordinates never collide).

  The full FAILOVER flow (A drives a live swap, B follows read-only, kill A, take
  over on B, B drives to completion) is on-chain + human-confirm driven, so it is
  a MANUAL walkthrough (printed at the end) you run on top of a normal playground
  stack - not scripted here.

  Usage:
    tools\playground-multimachine.ps1            # build, run the checks, print the walkthrough
    tools\playground-multimachine.ps1 -NoBuild   # skip cargo build (use the existing debug pactd)
    tools\playground-multimachine.ps1 -Down      # tear down (stop the two test pactd, wipe temp dirs)

  Safety: only ever touches the two test ports (19801/19802/19803) and its own
  temp data dirs. It NEVER kills pactd by name (your mainnet/testnet daemons are
  safe) - teardown is by these ports only.
#>
param([switch]$Down, [switch]$NoBuild)

$ErrorActionPreference = "Stop"
$Repo = Split-Path -Parent $PSScriptRoot
$Pactd = Join-Path $Repo "pact\target\debug\pactd.exe"
$Root = Join-Path $env:TEMP "pact-mm-playground"
$DirA = Join-Path $Root "machineA"
$DirB = Join-Path $Root "machineB"
$PortA = 19801
$PortB = 19802
$PortLock = 19803   # the refused 2nd-pactd-on-A attempt

# The standard BIP39 test mnemonic - shared by both "machines".
$Mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

# --- helpers ---------------------------------------------------------------

# Kill only the process bound to one of OUR test ports (never by name).
function Stop-OnPort([int]$Port) {
    try {
        $conns = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
        foreach ($c in $conns) {
            Stop-Process -Id $c.OwningProcess -Force -ErrorAction SilentlyContinue
        }
    } catch {}
}

function Invoke-Teardown {
    foreach ($p in @($PortA, $PortB, $PortLock)) { Stop-OnPort $p }
    Start-Sleep -Milliseconds 300
    if (Test-Path $Root) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }
}

# One JSON-RPC call to a pactd, authing with its .cookie (Basic auth).
function Invoke-Pactd([int]$Port, [string]$DataDir, [string]$Method, [object[]]$Params = @()) {
    $cookie = (Get-Content -Raw (Join-Path $DataDir ".cookie")).Trim()  # "__cookie__:<hex>"
    $b64 = [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes($cookie))
    $body = @{ jsonrpc = "2.0"; id = 1; method = $Method; params = $Params } | ConvertTo-Json -Compress -Depth 8
    $resp = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/" -Method Post `
        -Headers @{ Authorization = "Basic $b64" } -Body $body -ContentType "application/json"
    if ($resp.error) { throw "pactd $Method error: $($resp.error | ConvertTo-Json -Compress)" }
    return $resp.result
}

# Launch a pactd on a data dir + port; return the Process. Logs to <dir>\out.log.
function Start-Pactd([string]$DataDir, [int]$Port) {
    New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
    $log = Join-Path $DataDir "out.log"
    return Start-Process -FilePath $Pactd `
        -ArgumentList "--data-dir", $DataDir, "--listen", "127.0.0.1:$Port", "--network", "regtest" `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err" -PassThru -WindowStyle Hidden
}

# Poll getinfo until the daemon answers (or time out).
function Wait-Pactd([int]$Port, [string]$DataDir, [int]$TimeoutSec = 20) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try { Invoke-Pactd $Port $DataDir "getinfo" | Out-Null; return } catch { Start-Sleep -Milliseconds 400 }
    }
    throw "pactd on :$Port ($DataDir) did not come up within ${TimeoutSec}s"
}

$script:Fail = 0
function Test-Check([string]$Name, [bool]$Ok, [string]$Detail = "") {
    if ($Ok) { Write-Host "  [PASS] $Name" -ForegroundColor Green }
    else { Write-Host "  [FAIL] $Name  $Detail" -ForegroundColor Red; $script:Fail++ }
}

# --- teardown mode ---------------------------------------------------------

if ($Down) { Write-Host "Tearing down multi-machine playground..."; Invoke-Teardown; Write-Host "Done."; return }

# --- run -------------------------------------------------------------------

if (-not $NoBuild) {
    Write-Host "Building pactd (debug)..." -ForegroundColor Cyan
    Push-Location (Join-Path $Repo "pact")
    try { cargo build -p pactd; if ($LASTEXITCODE -ne 0) { throw "cargo build failed" } } finally { Pop-Location }
}
if (-not (Test-Path $Pactd)) { throw "pactd.exe not found at $Pactd (run without -NoBuild, or build it first)" }

Invoke-Teardown  # clean slate
New-Item -ItemType Directory -Force -Path $Root, $DirA, $DirB | Out-Null

Write-Host ""
Write-Host "Starting machine A (:$PortA) and machine B (:$PortB) on ONE seed..." -ForegroundColor Cyan
$procA = Start-Pactd $DirA $PortA
Wait-Pactd $PortA $DirA
Invoke-Pactd $PortA $DirA "importseed" @($Mnemonic) | Out-Null

$procB = Start-Pactd $DirB $PortB
Wait-Pactd $PortB $DirB
Invoke-Pactd $PortB $DirB "importseed" @($Mnemonic) | Out-Null

Write-Host ""
Write-Host "Checks:" -ForegroundColor Cyan

# 1. Section 0 data-dir lock - a 2nd pactd on machine A's data dir must be refused.
$lockLog = Join-Path $DirA "lock-attempt.log"
$lockProc = Start-Process -FilePath $Pactd `
    -ArgumentList "--data-dir", $DirA, "--listen", "127.0.0.1:$PortLock", "--network", "regtest" `
    -RedirectStandardOutput $lockLog -RedirectStandardError "$lockLog.err" -PassThru -WindowStyle Hidden
$lockProc.WaitForExit(8000) | Out-Null
$refused = $lockProc.HasExited -and $lockProc.ExitCode -ne 0
if (-not $lockProc.HasExited) { Stop-Process -Id $lockProc.Id -Force -ErrorAction SilentlyContinue }
$lockErr = (Get-Content "$lockLog.err" -Raw -ErrorAction SilentlyContinue)
Test-Check "data-dir lock refuses a 2nd pactd on machine A's dir" $refused "(exit=$($lockProc.ExitCode)); stderr: $lockErr"

# 2. Section 1 machine label - A and B have DISTINCT machine labels (distinct scopes).
$labelA = (Invoke-Pactd $PortA $DirA "getinfo").machine_label
$labelB = (Invoke-Pactd $PortB $DirB "getinfo").machine_label
Test-Check "machines A/B derive distinct labels" ($labelA -and $labelB -and ($labelA -ne $labelB)) "A=$labelA B=$labelB"

# 3. Section 1 partition - the SAME offer on A and B => distinct swap_id + distinct nonzero scope.
$offA = (Invoke-Pactd $PortA $DirA "offer" @("btcx:100", "btc:100", 1700000002, 1700000001)).record
$offB = (Invoke-Pactd $PortB $DirB "offer" @("btcx:100", "btc:100", 1700000002, 1700000001)).record
Test-Check "same offer => distinct swap_id per machine" ($offA.swap_id -ne $offB.swap_id) "A=$($offA.swap_id) B=$($offB.swap_id)"
Test-Check "same offer => distinct nonzero derive_scope per machine" `
    (($offA.derive_scope -ne $offB.derive_scope) -and ($offA.derive_scope -ne 0) -and ($offB.derive_scope -ne 0)) `
    "A=$($offA.derive_scope) B=$($offB.derive_scope)"

# --- result + manual walkthrough ------------------------------------------

Write-Host ""
if ($script:Fail -eq 0) { Write-Host "All automated checks PASSED." -ForegroundColor Green }
else { Write-Host "$($script:Fail) check(s) FAILED - see above." -ForegroundColor Red }

Write-Host ""
Write-Host "Machine A: http://127.0.0.1:$PortA  (data dir $DirA, label $labelA)"
Write-Host "Machine B: http://127.0.0.1:$PortB  (data dir $DirB, label $labelB)"
Write-Host "Both are running. Tear down with:  tools\playground-multimachine.ps1 -Down"
Write-Host ""
Write-Host "--- MANUAL failover walkthrough (needs a live swap; run on a full playground) ---" -ForegroundColor Gray
Write-Host "The on-chain failover cannot be scripted deterministically (it needs a real" -ForegroundColor Gray
Write-Host "swap, a machine kill, and a human 'that machine is stopped' confirm). To do it:" -ForegroundColor Gray
Write-Host " 1. Bring up a normal stack (nodes + board + a COUNTERPARTY trader), e.g." -ForegroundColor Gray
Write-Host "    tools\playground-nodeless.ps1, and point BOTH A and B at the SAME coins" -ForegroundColor Gray
Write-Host "    (--coin ...), same board/relay. A and B share this seed's wallet." -ForegroundColor Gray
Write-Host " 2. On machine A, post an offer and have the counterparty take it: listswaps" -ForegroundColor Gray
Write-Host "    on A shows source=local." -ForegroundColor Gray
Write-Host " 3. On machine B, run listswaps: the SAME swap shows source=foreign and" -ForegroundColor Gray
Write-Host "    machine_label=$labelA (Satchel: the read-only 'Another machine' group)." -ForegroundColor Gray
Write-Host "    Confirm B never broadcasts for it (its scheduler skips it; belt refuses)." -ForegroundColor Gray
Write-Host " 4. Withdraw (getnewaddress/sendtoaddress) on BOTH A and B - always allowed." -ForegroundColor Gray
Write-Host " 5. Kill machine A (stop its pactd). Confirm it is really stopped." -ForegroundColor Gray
Write-Host " 6. On machine B: call takeover <swap_id> (Satchel: the group's 'Take over'" -ForegroundColor Gray
Write-Host "    button). listswaps on B now shows source=local; B drives it to the end." -ForegroundColor Gray
