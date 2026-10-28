; Upgrading from "Claude Accounts" (this app before 4.3.0, same data folders):
; that name is a separate install to NSIS, so remove it - silently, which also
; closes it and drops its shortcuts, but keeps its app data - and carry its
; start-at-login entry over to the new name.
!include LogicLib.nsh

!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claude Accounts" "InstallLocation"
  ${If} $R0 != ""
    ReadRegStr $R1 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Claude Accounts"
    nsis_tauri_utils::StrReplace "$R0" '"' ""
    Pop $R0
    ExecWait '"$R0\uninstall.exe" /S _?=$R0'
    Delete "$R0\uninstall.exe"
    Delete "$R0\debug.log"
    RMDir "$R0"
    ${If} $R1 != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --minimized'
    ${EndIf}
  ${EndIf}
!macroend
