!define PROGRAM_NAME "WinSysHlp"
!define DISPLAY_NAME "Windows System Helper"
!define PUBLISHER "MicroWin"
!define EXE_FILE "C:\Users\Syed\RustroverProjects\SysMon\target\x86_64-pc-windows-msvc\release\SysMon.exe"
!define INSTALL_DIR "$PROGRAMFILES64\${PUBLISHER}\${PROGRAM_NAME}"
!define CONFIG_FILE "orgdt.cng"
!define PS_SCRIPT "script.ps1"

!include "nsDialogs.nsh"
!include "MUI2.nsh"
!include "FileFunc.nsh"

; Request application privileges for Windows Vista/7/8/10
RequestExecutionLevel admin

; Add version information
VIProductVersion "1.0.0.0"
VIAddVersionKey "ProductName" "${DISPLAY_NAME}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "© ${PUBLISHER}"
VIAddVersionKey "FileDescription" "System Monitoring Application"
VIAddVersionKey "FileVersion" "1.0.0.0"
VIAddVersionKey "ProductVersion" "1.0.0"

OutFile "WinSysMonSetup.exe"
InstallDir "${INSTALL_DIR}"

Var Dialog
Var OrgCodeLabel
Var OrgCodeText
Var EmpCodeLabel
Var EmpCodeText
Var ORG_CODE
Var EMP_CODE

; Modern UI settings
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "license.txt" ; You should create a license file
Page custom GetOrgEmpCodePage GetOrgEmpCodePageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Function .onInit
    SetShellVarContext all    ; Install for all users

    ; Check if running with admin privileges
    UserInfo::GetAccountType
    Pop $0
    ${If} $0 != "admin"
        MessageBox MB_OK|MB_ICONSTOP "Administrator rights required!"
        Quit
    ${EndIf}
FunctionEnd

Function GetOrgEmpCodePage
    nsDialogs::Create 1018
    Pop $Dialog

    ${NSD_CreateLabel} 0 0 100% 12u "Enter Organization Code:"
    Pop $OrgCodeLabel

    ${NSD_CreateText} 0 13u 100% 12u ""
    Pop $OrgCodeText

    ${NSD_CreateLabel} 0 30u 100% 12u "Enter Employee Code:"
    Pop $EmpCodeLabel

    ${NSD_CreateText} 0 43u 100% 12u ""
    Pop $EmpCodeText

    nsDialogs::Show
FunctionEnd

Function GetOrgEmpCodePageLeave
    ${NSD_GetText} $OrgCodeText $ORG_CODE
    ${NSD_GetText} $EmpCodeText $EMP_CODE

    ${If} $ORG_CODE == ""
        MessageBox MB_OK "Organization Code cannot be empty. Please provide a value."
        Abort
    ${EndIf}

    ${If} $EMP_CODE == ""
        MessageBox MB_OK "Employee Code cannot be empty. Please provide a value."
        Abort
    ${EndIf}
FunctionEnd

Function CreateConfigFile
    ; Create and write to config file
    FileOpen $0 "$INSTDIR\${CONFIG_FILE}" w
    FileWrite $0 "$ORG_CODE$\r$\n"
    FileWrite $0 "$EMP_CODE"
    FileClose $0

    ; Remove restrictive NTFS permissions -  Allow everyone full control
    ExecWait 'icacls "$INSTDIR\${CONFIG_FILE}" /inheritance:r /grant:f "Everyone":(F)' ; Everyone has Full Control
FunctionEnd

Section "Install Program"
    SetOutPath "$INSTDIR"

    ; Create required directories
    CreateDirectory "$INSTDIR"

    ; Remove restrictive NTFS permissions from the main directory - Allow everyone full control
    ExecWait 'icacls "$INSTDIR" /inheritance:r /grant:f "Everyone":(OI)(CI)(F)'


    ; Install main executable
    File "${EXE_FILE}"
    Rename "$OUTDIR\SysMon.exe" "$OUTDIR\${PROGRAM_NAME}.exe"

    ; Remove restrictive NTFS permissions from the executable - Allow everyone full control
    ExecWait 'icacls "$OUTDIR\${PROGRAM_NAME}.exe" /inheritance:r /grant:f "Everyone":(F)'


    ; Install PowerShell script
    File "${PS_SCRIPT}"
    ExecWait 'icacls "$OUTDIR\${PS_SCRIPT}" /inheritance:r /grant:f "Everyone":(F)'

    ; Create config file
    Call CreateConfigFile

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Add uninstaller information to Add/Remove Programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}" \
        "DisplayName" "${DISPLAY_NAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}" \
        "DisplayIcon" "$INSTDIR\${PROGRAM_NAME}.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}" \
        "Publisher" "${PUBLISHER}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}" \
        "DisplayVersion" "1.0.0"

    MessageBox MB_OK "${DISPLAY_NAME} installed successfully."
SectionEnd

Section "Uninstall"
    ; Remove installation directory
    RMDir /r "$INSTDIR"

    ; Remove registry entries
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PROGRAM_NAME}"

    MessageBox MB_OK "${DISPLAY_NAME} uninstalled successfully."
SectionEnd