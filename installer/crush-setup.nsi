; crush-setup.nsi — unified Crush installer (CLI + GUI, component-selectable)
;
; One installer that can install the Crush CLI, the Crush desktop GUI, or both —
; chosen on the Components page. A Tauri app compiles to a single self-contained
; .exe, so both binaries ship in one installer. Built by scripts/build-installer.ps1.
;
; PATH editing uses the EnVar plugin (https://nsis.sourceforge.io/EnVar_plug-in) —
; the build script drops EnVar.dll into a local plugin dir and passes !addplugindir.
;
; Overridable defines (passed by the build script via makensis /D...):
;   VERSION   product version                         (default 0.0.0)
;   CLI_EXE   path to built crush-cli.exe             (CLI section omitted if unset)
;   GUI_EXE   path to built crush-gui.exe             (GUI section omitted if unset)
;   WV2_BOOT  path to MicrosoftEdgeWebview2Setup.exe  (optional; installs WebView2)
;   OUTFILE   output installer path                   (default Crush-Setup-<VERSION>.exe)

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef OUTFILE
  !define OUTFILE "Crush-Setup-${VERSION}.exe"
!endif

Unicode true
Name "Crush ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\Crush"
InstallDirRegKey HKLM "Software\Crush" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "LogicLib.nsh"

!define MUI_ICON   "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"
!define MUI_ABORTWARNING
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Crush"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!ifdef GUI_EXE
  !define MUI_FINISHPAGE_RUN "$INSTDIR\Crush.exe"
  !define MUI_FINISHPAGE_RUN_TEXT "Launch Crush desktop"
!endif
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; ── Components ────────────────────────────────────────────────────────────────
!ifdef CLI_EXE
Section "Crush CLI (command line)" SecCLI
  SetOutPath "$INSTDIR"
  File "/oname=crush.exe" "${CLI_EXE}"
  ; Add install dir to the system PATH (EnVar dedupes + broadcasts WM_SETTINGCHANGE)
  EnVar::SetHKLM
  EnVar::AddValue "Path" "$INSTDIR"
  Pop $0
  DetailPrint "PATH update: $0"
SectionEnd
!endif

!ifdef GUI_EXE
Section "Crush Desktop (GUI)" SecGUI
  SetOutPath "$INSTDIR"
  File "/oname=Crush.exe" "${GUI_EXE}"

  ; Ensure the Edge WebView2 runtime is present (the GUI needs it).
  !ifdef WV2_BOOT
  ClearErrors
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${EndIf}
  ${If} $0 == ""
    DetailPrint "Installing Microsoft Edge WebView2 runtime..."
    SetOutPath "$TEMP"
    File "/oname=wv2setup.exe" "${WV2_BOOT}"
    ExecWait '"$TEMP\wv2setup.exe" /silent /install'
    Delete "$TEMP\wv2setup.exe"
    SetOutPath "$INSTDIR"
  ${EndIf}
  !endif

  CreateDirectory "$SMPROGRAMS\Crush"
  CreateShortCut "$SMPROGRAMS\Crush\Crush.lnk" "$INSTDIR\Crush.exe"
  CreateShortCut "$DESKTOP\Crush.lnk" "$INSTDIR\Crush.exe"
SectionEnd
!endif

; Always-on: register uninstaller + Add/Remove Programs entry.
Section "-post"
  WriteRegStr HKLM "Software\Crush" "InstallDir" "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayName"     "Crush"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayVersion"  "${VERSION}"
  WriteRegStr   HKLM "${UNINST_KEY}" "Publisher"       "Crush"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayIcon"     "$INSTDIR\uninstall.exe"
  WriteRegStr   HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
SectionEnd

; Component descriptions
!ifdef CLI_EXE
  LangString DESC_CLI ${LANG_ENGLISH} "The `crush` command-line tool, added to your system PATH."
!endif
!ifdef GUI_EXE
  LangString DESC_GUI ${LANG_ENGLISH} "The Crush desktop application (Start Menu + Desktop shortcuts)."
!endif
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !ifdef CLI_EXE
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCLI} $(DESC_CLI)
  !endif
  !ifdef GUI_EXE
    !insertmacro MUI_DESCRIPTION_TEXT ${SecGUI} $(DESC_GUI)
  !endif
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ── Uninstaller ───────────────────────────────────────────────────────────────
Section "Uninstall"
  EnVar::SetHKLM
  EnVar::DeleteValue "Path" "$INSTDIR"
  Pop $0

  Delete "$INSTDIR\crush.exe"
  Delete "$INSTDIR\Crush.exe"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\Crush\Crush.lnk"
  RMDir  "$SMPROGRAMS\Crush"
  Delete "$DESKTOP\Crush.lnk"
  RMDir  "$INSTDIR"

  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\Crush"
SectionEnd
