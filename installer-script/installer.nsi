!define PROGRAM_NAME "SysMon"
!define EXE_FILE "C:\Users\Syed\RustroverProjects\SysMon\target\x86_64-pc-windows-msvc\release\SysMon.exe"
!define INSTALL_DIR "C:\Windows\System32\winsysmon"
!define CONFIG_FILE "orgdt.cng"

!include "nsDialogs.nsh"
!include "MUI2.nsh"

OutFile "SysMonInstaller.exe"
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
Page custom GetOrgEmpCodePage GetOrgEmpCodePageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

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
    MessageBox MB_OK "Configuration file created successfully."
FunctionEnd

Function CreateScheduledTask
    ; Create a scheduled task that runs on startup for all users
    ; Runs the executable every 120 minutes
    ExecWait 'schtasks /create /tn "WindowsSysMon" /tr "${INSTALL_DIR}\${PROGRAM_NAME}.exe" /sc minute /mo 120 /ru "SYSTEM" /rl highest /f /sc onstart'
FunctionEnd

Section "Install Program"
    ; Create Installation Directory
    CreateDirectory "${INSTALL_DIR}"

    ; Set output path to installation directory
    SetOutPath "${INSTALL_DIR}"

    ; Create config file with ORG_CODE and EMP_CODE
    Call CreateConfigFile

    ; Copy Executable to Installation Directory
    File "${EXE_FILE}"

    ; Create Scheduled Task
    Call CreateScheduledTask

    MessageBox MB_OK "${PROGRAM_NAME} installed successfully and will run on startup every 120 minutes."
SectionEnd

Section "Uninstall Program"
    ; Remove Task Scheduler Task
    ExecWait 'schtasks /delete /tn "WindowsSysMon" /f'

    ; Remove Installation Directory (this will also remove the config file)
    RMDir /r "${INSTALL_DIR}"

    MessageBox MB_OK "${PROGRAM_NAME} uninstalled successfully."
SectionEnd