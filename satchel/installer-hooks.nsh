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
