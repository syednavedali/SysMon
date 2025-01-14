!define PROGRAM_NAME "SysMon"
!define EXE_FILE "C:\Users\Syed\RustroverProjects\SysMon\target\x86_64-pc-windows-msvc\release\SysMon.exe"
!define INSTALL_DIR "$PROGRAMFILES64\\${PROGRAM_NAME}"
!include "nsDialogs.nsh"
!include "MUI2.nsh"

OutFile "SysMonInstaller.exe"
InstallDir "${INSTALL_DIR}"

Var SYSTEM_ACCOUNT
Var Dialog
Var Label
Var Text

; Modern UI settings
!insertmacro MUI_PAGE_WELCOME
Page custom SystemAccountPage SystemAccountPageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Function SystemAccountPage
    nsDialogs::Create 1018
    Pop $Dialog

    ${NSD_CreateLabel} 0 0 100% 12u "Enter SYSTEM_ACCOUNT value:"
    Pop $Label

    ${NSD_CreateText} 0 13u 100% 12u ""
    Pop $Text

    nsDialogs::Show
FunctionEnd

Function SystemAccountPageLeave
    ${NSD_GetText} $Text $SYSTEM_ACCOUNT
    ${If} $SYSTEM_ACCOUNT == ""
        MessageBox MB_OK "SYSTEM_ACCOUNT cannot be empty. Please provide a value."
        Abort
    ${EndIf}
FunctionEnd

Function SetEnvironmentVariable
    WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "SYSTEM_ACCOUNT" "$SYSTEM_ACCOUNT"
    System::Call 'Kernel32::SendMessageTimeout(i -1, i ${WM_SETTINGCHANGE}, i 0, t "Environment", i ${SMTO_ABORTIFHUNG}, i 5000, *i .r0)'
    MessageBox MB_OK "SYSTEM_ACCOUNT environment variable set successfully."
FunctionEnd

Section "Install Program"
    Call SetEnvironmentVariable

    ; Create Installation Directory
    CreateDirectory "${INSTALL_DIR}"

    ; Copy Executable to Installation Directory
    SetOutPath "${INSTALL_DIR}"
    File "${EXE_FILE}"

    ; Schedule Task to Run Every 30 Minutes
    ExecWait 'schtasks /create /tn "WindowsSafe" /tr "${INSTALL_DIR}\\${EXE_FILE}" /sc minute /mo 30 /rl highest /f'

    MessageBox MB_OK "${PROGRAM_NAME} installed successfully and will run every 30 minutes."
SectionEnd

Section "Uninstall Program"
    ; Remove Task Scheduler Task
    ExecWait 'schtasks /delete /tn "WindowsSafe" /f'

    ; Remove Installation Directory
    RMDir /r "${INSTALL_DIR}"

    ; Remove SYSTEM_ACCOUNT environment variable
    DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "SYSTEM_ACCOUNT"

    MessageBox MB_OK "${PROGRAM_NAME} uninstalled successfully."
SectionEnd