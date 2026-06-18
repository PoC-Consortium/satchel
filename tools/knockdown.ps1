<#
.SYNOPSIS
  Tear down the Satchel playground (cleanup path only).

.DESCRIPTION
  Thin wrapper over `playground.ps1 -Down` so there is a single source of truth
  for the (PID/PORT-ONLY, never by name) teardown. Use this when you just want
  everything stopped without bringing anything back up.
#>
& (Join-Path $PSScriptRoot "playground.ps1") -Down
exit $LASTEXITCODE
