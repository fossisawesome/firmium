!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef ARCH
  !define ARCH "x86_64-pc-windows-msvc"
!endif

Name "Termium"
OutFile "..\termium-setup-${VERSION}-${ARCH}.exe"
InstallDir "$PROGRAMFILES64\Termium"
InstallDirRegKey HKLM "Software\Termium" "Install_Dir"
RequestExecutionLevel admin

!define MUI_ICON "..\assets\app-icons\icon.ico"
!define MUI_UNICON "..\assets\app-icons\icon.ico"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Termium"
  SetOutPath "$INSTDIR"
  File "..\target\${ARCH}\release\termium.exe"
  File "..\assets\app-icons\icon.ico"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\Termium" "Install_Dir" "$INSTDIR"

  ; termium.exe is a console app, so the shortcut opens its own terminal window.
  CreateDirectory "$SMPROGRAMS\Termium"
  CreateShortcut "$SMPROGRAMS\Termium\Termium.lnk" "$INSTDIR\termium.exe" "" "$INSTDIR\icon.ico"

  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "DisplayName" "Termium"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "Publisher" "fossisawesome"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium" \
    "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\termium.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\Termium\Termium.lnk"
  RMDir "$SMPROGRAMS\Termium"

  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Termium"
  DeleteRegKey HKLM "Software\Termium"
SectionEnd
