; Satchel NSIS installer hooks (wired via tauri.conf.json bundle.windows.nsis.installerHooks).
;
; Goal: make pact-cli / pactd runnable from any terminal WITHOUT requiring admin.
; We keep the default per-user install (into %LOCALAPPDATA%) and append the
; install dir to the *user* PATH (HKCU\Environment) — the user owns that key, so
; no UAC elevation is needed. The system PATH (HKLM) is never touched.
;
; Pure NSIS: StrFunc.nsh + WinMessages.nsh are standard headers shipped with
; NSIS, so there is no external-plugin dependency.

!include "StrFunc.nsh"
!include "WinMessages.nsh"

; Instantiate the StrFunc helpers we use (installer: StrStr; uninstaller: UnStrRep).
${StrStr}
${UnStrRep}

!macro NSIS_HOOK_POSTINSTALL
  Push $0
  Push $1
  ReadRegStr $0 HKCU "Environment" "Path"
  ; Skip if our dir is already on PATH (avoid duplicates on reinstall/upgrade).
  ${StrStr} $1 "$0" "$INSTDIR"
  StrCmp $1 "" 0 satchel_path_done
    StrCmp $0 "" satchel_path_empty satchel_path_append
    satchel_path_empty:
      WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR"
      Goto satchel_path_notify
    satchel_path_append:
      WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR"
    satchel_path_notify:
      ; Tell already-running processes (Explorer, etc.) the environment changed,
      ; so newly-spawned shells pick up the new PATH without a reboot.
      SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  satchel_path_done:
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Push $0
  ReadRegStr $0 HKCU "Environment" "Path"
  ; Best-effort removal of our entry, in all three positional forms.
  ${UnStrRep} $0 "$0" ";$INSTDIR" ""
  ${UnStrRep} $0 "$0" "$INSTDIR;" ""
  ${UnStrRep} $0 "$0" "$INSTDIR" ""
  WriteRegExpandStr HKCU "Environment" "Path" "$0"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  Pop $0
!macroend
