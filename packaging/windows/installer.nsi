Unicode True
SetCompressor /SOLID lzma

!ifndef APP_VERSION
  !define APP_VERSION "1.0.0"
!endif
!ifndef APP_EXE
  !define APP_EXE "usedns.exe"
!endif
!ifndef OUT_FILE
  !define OUT_FILE "UseDNS-Setup.exe"
!endif
!ifndef APP_ICON
  !define APP_ICON "UseDNS.ico"
!endif

Name "UseDNS"
OutFile "${OUT_FILE}"
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
InstallDir "$PROGRAMFILES64\UseDNS"
InstallDirRegKey HKLM "Software\UseDNS" "InstallDir"
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "${APP_EXE}"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\UseDNS" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "DisplayName" "UseDNS"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "Publisher" "UseDNS"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "DisplayIcon" "$INSTDIR\usedns.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\UseDNS"
  CreateShortCut "$SMPROGRAMS\UseDNS\UseDNS.lnk" "$INSTDIR\usedns.exe" "" "$INSTDIR\usedns.exe" 0
  CreateShortCut "$DESKTOP\UseDNS.lnk" "$INSTDIR\usedns.exe" "" "$INSTDIR\usedns.exe" 0
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\usedns.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\UseDNS\UseDNS.lnk"
  RMDir "$SMPROGRAMS\UseDNS"
  Delete "$DESKTOP\UseDNS.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\UseDNS"
  DeleteRegKey HKLM "Software\UseDNS"
SectionEnd
