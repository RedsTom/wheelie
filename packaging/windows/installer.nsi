; NSIS installer script for Wheelie.
;
; Invoked by the release workflow from the repository root:
;   makensis /DAPP_VERSION=<ver> /DOUTFILE=<setup.exe> packaging\windows\installer.nsi

Unicode true
!include "MUI2.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "0.0.0"
!endif
; Paths are resolved relative to this script unless overridden by the
; release workflow with absolute defines.
!ifndef BINARY
  !define BINARY "..\..\target\release\wheelie.exe"
!endif
!ifndef ICON
  !define ICON "..\..\assets\icons\app.ico"
!endif
!ifndef OUTFILE
  !define OUTFILE "wheelie-setup.exe"
!endif
!ifndef LICENSE
  !define LICENSE "..\..\LICENSE"
!endif

!define APP_NAME "Wheelie"
!define APP_EXE "wheelie.exe"
!define APP_PUBLISHER "RedsTom"
!define APP_URL "https://github.com/RedsTom/wheelie"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel admin
BrandingText "${APP_PUBLISHER}"
ShowInstDetails "nevershow"

!define MUI_ABORTWARNING
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_WELCOMEPAGE_TEXT "This wizard installs ${APP_NAME} ${APP_VERSION}.$\r$\n$\r$\n${APP_NAME} turns a keyboard knob into a configurable radial wheel.$\r$\n$\r$\nIt is recommended to close any running Wheelie instance before continuing."
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "French"

Section "${APP_NAME}" SEC01
  SetOutPath "$INSTDIR"
  File "${BINARY}"
  File "/oname=LICENSE.txt" "${LICENSE}"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\uninstall.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "URLInfoAbout" "${APP_URL}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
SectionEnd
