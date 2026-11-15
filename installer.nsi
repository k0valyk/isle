; NSIS installer for Isle 1.0
!include "MUI2.nsh"
Unicode true

Name "Isle"
OutFile "Isle-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Isle"
InstallDirRegKey HKCU "Software\Isle" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "1.6.0.0"
VIAddVersionKey "ProductName" "Isle"
VIAddVersionKey "FileDescription" "Isle - a live island for Windows"
VIAddVersionKey "FileVersion" "1.6.0"
VIAddVersionKey "ProductVersion" "1.6.0"
VIAddVersionKey "LegalCopyright" "MIT License"

!define MUI_ABORTWARNING
!define MUI_ICON "src-tauri\icons\icon.ico"
!define MUI_UNICON "src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Ukrainian"

Section "Isle" SEC01
  SetOutPath "$INSTDIR"
  File "src-tauri\target\release\isle.exe"
  WriteRegStr HKCU "Software\Isle" "InstallDir" "$INSTDIR"

  CreateDirectory "$SMPROGRAMS\Isle"
  CreateShortcut "$SMPROGRAMS\Isle\Isle.lnk" "$INSTDIR\isle.exe"
  CreateShortcut "$SMPROGRAMS\Isle\Uninstall Isle.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Isle.lnk" "$INSTDIR\isle.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "DisplayName" "Isle"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "DisplayVersion" "1.6.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "DisplayIcon" "$INSTDIR\isle.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "Publisher" "Isle contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\isle.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\Isle\Isle.lnk"
  Delete "$SMPROGRAMS\Isle\Uninstall Isle.lnk"
  RMDir "$SMPROGRAMS\Isle"
  Delete "$DESKTOP\Isle.lnk"

  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Isle"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Isle"
  DeleteRegKey HKCU "Software\Isle"

  ; Settings, clipboard history and notes stay in %APPDATA%\Isle by design.
SectionEnd
