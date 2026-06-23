!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif

Name "Firmium"
OutFile "firmium-setup-${VERSION}.exe"
InstallDir "$PROGRAMFILES64\Firmium"
InstallDirRegKey HKLM "Software\Firmium" "Install_Dir"
RequestExecutionLevel admin

!define MUI_ICON "..\assets\app-icons\icon.ico"
!define MUI_UNICON "..\assets\app-icons\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\firmium.exe"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Firmium"
  SetOutPath "$INSTDIR"
  File "..\target\release\firmium.exe"
  File "..\assets\app-icons\icon.ico"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Firmium"
  CreateShortcut "$SMPROGRAMS\Firmium\Firmium.lnk" "$INSTDIR\firmium.exe" "" "$INSTDIR\icon.ico"
  CreateShortcut "$DESKTOP\Firmium.lnk" "$INSTDIR\firmium.exe" "" "$INSTDIR\icon.ico"

  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "DisplayName" "Firmium"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "Publisher" "fossisawesome"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium" \
    "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\firmium.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\Firmium\Firmium.lnk"
  RMDir "$SMPROGRAMS\Firmium"
  Delete "$DESKTOP\Firmium.lnk"

  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Firmium"
  DeleteRegKey HKLM "Software\Firmium"
SectionEnd
