; Satchel NSIS installer hooks (wired via tauri.conf.json bundle.windows.nsis.installerHooks).
;
; Goal: make pact-cli / pactd runnable from any terminal WITHOUT requiring admin.
; We keep the default per-user install (into %LOCALAPPDATA%) and append the
; install dir to the *user* PATH (HKCU\Environment) — the user owns that key, so
; no UAC elevation is needed. The system PATH (HKLM) is never touched.
;
; WHY POWERSHELL AND NOT ReadRegStr/WriteRegStr (the bug this replaces):
; NSIS strings are capped at NSIS_MAX_STRLEN (1024 in the standard build Tauri
; uses). `ReadRegStr $0 HKCU "Environment" "Path"` therefore SILENTLY TRUNCATES a
; PATH longer than 1024 chars, and writing $0 back DELETES everything past the cut
; — wiping user PATH entries (e.g. cargo, npm-global tools). The .NET registry API
; reads/writes the full value with no length limit AND preserves REG_EXPAND_SZ.
; nsExec ships with NSIS (no external plugin), and powershell.exe is always on the
; system PATH (System32), so a broken user PATH can't stop the repair.
;
; PowerShell quoting note: this file uses backtick-delimited NSIS strings so inner
; ' and " are literal; `$$` emits a literal `$` (a PowerShell variable), while
; `$INSTDIR` is expanded by NSIS to the install directory.

!include "WinMessages.nsh"

; Stop any pactd / pact-cli running FROM THIS INSTALL DIR before files are
; (over)written. Tauri's NSIS template only handles the main app (satchel.exe,
; via CheckIfAppIsRunning right after this hook); sidecars are invisible to it,
; and a deliberately-detached pactd (C6 keep-running-on-close) survives
; Satchel's exit BY DESIGN — which is correct in operation but wrong during an
; upgrade: it holds a lock on pactd.exe (the overwrite fails or silently keeps
; the old engine) and the next Satchel would re-adopt the OLD binary.
; Path-scoped so a dev build or playground pactd running from elsewhere is
; never touched. A hard stop is tolerated by design (state is persisted around
; every broadcast; chain-watch resumes the swap when the new pactd comes up).
;
; WHY CIM AND NOT `Get-Process | Where Path -like ...` (the bug this replaces,
; issue #63): NSIS installers are 32-bit, so nsExec's `powershell` resolves to
; the 32-bit PowerShell in SysWOW64 — and there `(Get-Process).Path` (backed by
; Process.MainModule) is EMPTY for 64-bit targets like pactd, so the path
; filter silently matched nothing and nothing was ever stopped. WMI's
; Win32_Process.ExecutablePath is bitness-agnostic. Kill is by PID, and the
; loop re-queries until the processes are actually GONE (Stop-Process returns
; before the OS releases the exe lock), bounded by a 15 s deadline so a
; zombie can never hang the installer.
!macro _STOP_INSTDIR_DAEMONS
  Push $0
  nsExec::Exec `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$dir='$INSTDIR'; $$names=@('pactd.exe','pact-cli.exe'); $$deadline=(Get-Date).AddSeconds(15); do { $$procs=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $$names -contains $$_.Name -and $$_.ExecutablePath -and $$_.ExecutablePath.StartsWith($$dir + '\', [System.StringComparison]::OrdinalIgnoreCase) }); if (-not $$procs) { break }; $$procs | ForEach-Object { Stop-Process -Id $$_.ProcessId -Force -ErrorAction SilentlyContinue }; Start-Sleep -Milliseconds 250 } while ((Get-Date) -lt $$deadline)"`
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro _STOP_INSTDIR_DAEMONS
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro _STOP_INSTDIR_DAEMONS
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $0
  ; Append $INSTDIR to the USER Path if absent — full read-modify-write in .NET
  ; (no 1024-char truncation), written back as REG_EXPAND_SZ.
  nsExec::Exec `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$c=[string]$$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); if(($$c -split ';') -notcontains '$INSTDIR'){ $$n=($$c.TrimEnd(';') + ';$INSTDIR').TrimStart(';'); $$k.SetValue('Path',$$n,[Microsoft.Win32.RegistryValueKind]::ExpandString) }; $$k.Close()"`
  Pop $0    ; nsExec exit code — best-effort, ignored
  ; Tell already-running processes the environment changed, so newly-spawned
  ; shells pick up the new PATH without a reboot.
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  Pop $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Push $0
  ; Remove ONLY our entry from the USER Path, preserving every other entry
  ; (again full-width via .NET, never a truncating NSIS round-trip).
  nsExec::Exec `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$c=[string]$$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); $$p=@($$c -split ';' | Where-Object { $$_ -ne '' -and $$_ -ne '$INSTDIR' }); $$k.SetValue('Path',($$p -join ';'),[Microsoft.Win32.RegistryValueKind]::ExpandString); $$k.Close()"`
  Pop $0
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  Pop $0
!macroend
