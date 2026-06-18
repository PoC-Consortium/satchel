<#
.SYNOPSIS
  Tear down the Satchel playground (cleanup path only).

.DESCRIPTION
  Thin wrapper over `playground-cork.ps1 -Down` so there is a single source of
  truth for the (PID/PORT-ONLY, never by name) teardown. Both playground
  variants share the same PID file + port set, so one -Down tears down whichever
  is running. Use this when you just want everything stopped without bringing
  anything back up.
#>
& (Join-Path $PSScriptRoot "playground-cork.ps1") -Down
exit $LASTEXITCODE
