; UARTRecon Installer (NSIS)
; Multi-versi: FULL (CLI+TUI+GUI), GUI-only, CLI-only
;
; Build: makensis -DVERSION=0.1.0 -DEDITION=full installer\uartrecon.nsi

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; ---- Konfigurasi (dapat di-override via command line) ----
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef EDITION
  !define EDITION "full"
!endif
!ifndef SRCDIR
  !define SRCDIR "..\target\release"
!endif
!ifndef OUTDIR
  !define OUTDIR "..\dist"
!endif

!define APPNAME "UARTRecon"
!define PUBLISHER "UARTRecon Contributors"
!define WEBSITE "https://github.com/USER/uartrecon"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\UARTRecon"

Name "${APPNAME} ${VERSION}"
OutFile "${OUTDIR}\uartrecon-${EDITION}-${VERSION}-setup.exe"
Unicode True

InstallDir "$PROGRAMFILES64\UARTRecon"
InstallDirRegKey HKLM "Software\UARTRecon" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

; ---- Metadata versi ----
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APPNAME}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "FileDescription" "${APPNAME} Setup (${EDITION})"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT License"

; ---- UI ----
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\uartrecon.ico"
!define MUI_UNICON "..\assets\uartrecon.ico"
!define MUI_HEADERIMAGE
!define MUI_WELCOMEPAGE_TITLE "${APPNAME} ${VERSION}"
!define MUI_WELCOMEPAGE_TEXT "Installer UARTRecon (${EDITION}).$\r$\n$\r$\nToolkit reconnaissance UART - read-only first.$\r$\n$\r$\nKlik Next untuk melanjutkan."

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Indonesian"

; ---- Install ----
Section "UARTRecon" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"

  ; Binary sesuai edisi.
  !if "${EDITION}" == "full"
    File "${SRCDIR}\uartrecon.exe"
    File "${SRCDIR}\uartrecon-tui.exe"
    File "${SRCDIR}\uartrecon-gui.exe"
  !else if "${EDITION}" == "gui"
    File "${SRCDIR}\uartrecon-gui.exe"
  !else if "${EDITION}" == "cli"
    File "${SRCDIR}\uartrecon.exe"
    File "${SRCDIR}\uartrecon-tui.exe"
  !endif

  File "..\README.md"
  File "..\LICENSE"

  ; Shortcut Start Menu.
  !if "${EDITION}" == "gui"
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\uartrecon-gui.exe"
  !else if "${EDITION}" == "cli"
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME} (CLI).lnk" "$INSTDIR\uartrecon.exe"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME} (TUI).lnk" "$INSTDIR\uartrecon-tui.exe"
  !else
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME} (GUI).lnk" "$INSTDIR\uartrecon-gui.exe"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME} (CLI).lnk" "$INSTDIR\uartrecon.exe"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME} (TUI).lnk" "$INSTDIR\uartrecon-tui.exe"
    CreateShortcut "$SMPROGRAMS\${APPNAME}\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  !endif

  ; Desktop shortcut (GUI atau full).
  !if "${EDITION}" != "cli"
    CreateShortcut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\uartrecon-gui.exe"
  !endif

  ; Registry.
  WriteRegStr HKLM "Software\UARTRecon" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\UARTRecon" "Version" "${VERSION}"

  ; Uninstaller.
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Add/Remove Programs.
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "${WEBSITE}"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

  ; Estimasi ukuran.
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

; ---- Uninstall ----
Section "Uninstall"
  Delete "$INSTDIR\uartrecon.exe"
  Delete "$INSTDIR\uartrecon-tui.exe"
  Delete "$INSTDIR\uartrecon-gui.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APPNAME}\*.lnk"
  RMDir "$SMPROGRAMS\${APPNAME}"
  Delete "$DESKTOP\${APPNAME}.lnk"

  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\UARTRecon"
SectionEnd
