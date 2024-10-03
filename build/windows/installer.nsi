; Make sure to save this file as UTF-8 with BOM
; if it contains non-ascii characters. 

; The documentation for NSIS can be found here: <https://nsis.sourceforge.io/Docs/>
; Documentation for the modern UI can be found here: <https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html>

; This installer is written to do a per user installation as per Microsoft's recommendation:
; > In Windows 7 and later, we strongly recommend you 
; > install applications per user rather than per machine.
; (excerpt from <https://learn.microsoft.com/en-us/windows/win32/shell/app-registration#using-the-app-paths-subkey>)

; I don't usually write NSIS installers, so
; the comments here try to be extra chatty and helpful.

; Let's define some constants so we don't have to repeat
; ourselves later
!define MANUFACTURER "Tau Gärtli"
!define PRODUCT "Portal"
!define ICON_SRC "portal.ico"
!define EXECUTABLE "portal.exe"
!define EXECUTABLE_SRC "..\..\target\release\portal.exe"
!define EXECUTABLE_ALIAS "${EXECUTABLE}"
!define VERSION "0.3.0"
!define HOMEPAGE "https://github.com/bash/portal"
OutFile "portal-installer.exe"

!define UNIQUE_PRODUCT "${MANUFACTURER} (${PRODUCT})"

; The registry key where we store information for our uninstaller
; (like the name of the start menu folder and the path to our installation)
!define APP_KEY "Software\${MANUFACTURER}\${PRODUCT}"

; The registry key where we store information for Windows' Application List
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNIQUE_PRODUCT}"

Unicode true
ManifestDPIAware true ; This ensures that the installer looks nice on high-dpi displays
RequestExecutionLevel user ; We install per-user, so no need for admin privileges
SetCompressor lzma
Name "${PRODUCT}"
BrandingText "© ${MANUFACTURER}"
!define MUI_ICON "${ICON_SRC}"
!define MUI_UNICON "${ICON_SRC}"

; This includes the "Modern UI" macros that we'll use for our
; installer GUI
!include "MUI2.nsh"

!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "AutoUninstall.nsh"

Function .onInit
    ; We initalize the INSTDIR variable (the path to our installation)
    ; with a default value.
    ReadRegStr $0 SHCTX "${UNINSTALL_KEY}" "InstallLocation"
    ${If} $0 != ""
    	StrCpy $INSTDIR $0
    ${Else}
    	StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCT}"
    ${EndIf}
FunctionEnd

; Installer Pages
    !insertmacro MUI_PAGE_WELCOME

    ; A page that let's the user select an installation directory (Updates $INSTDIR).
    !insertmacro MUI_PAGE_DIRECTORY

    ; A page that lets the user configure the start menu shortcut
    Var StartMenuFolder
    !define MUI_STARTMENUPAGE_REGISTRY_ROOT "SHCTX"
    !define MUI_STARTMENUPAGE_REGISTRY_KEY "${APP_KEY}"
    !define MUI_STARTMENUPAGE_REGISTRY_VALUENAME "Start Menu Folder"
    !insertmacro MUI_PAGE_STARTMENU Application $StartMenuFolder

    ; Installation progress
    !insertmacro MUI_PAGE_INSTFILES

    !insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
    !insertmacro MUI_UNPAGE_WELCOME
    !insertmacro MUI_UNPAGE_CONFIRM
    !insertmacro MUI_UNPAGE_INSTFILES
    !insertmacro MUI_UNPAGE_FINISH

; Languages
    !insertmacro MUI_LANGUAGE "English"

; Here's where our actual installation happens.
Section "" ; The section name is empty because we don't let the user select individual components.
    SetOutPath $INSTDIR

    ; Let's uninstall the previous version (if it exists)
    ReadRegStr $0 SHCTX "${UNINSTALL_KEY}" "UninstallString"
    ${If} $0 != ""
		!insertmacro UninstallExisting $0 $0
		${If} $0 <> 0
			MessageBox MB_YESNO|MB_ICONSTOP "Failed to uninstall previous version, continue anyway?" /SD IDYES IDYES +2
				Abort
		${EndIf}
	${EndIf}

    ; Copies our executable to the installation dir.
    ; `oname` is the output path.
    File /oname=${EXECUTABLE} "${EXECUTABLE_SRC}"
    WriteUninstaller uninstall.exe

    ; Write Install Directory to registry
    WriteRegStr SHCTX "${APP_KEY}" "" "$INSTDIR"

    Call startMenuShortcut
    Call appPath
    Call uninstaller
    Call uriScheme
    Call registeredApplication
SectionEnd

; Here's where the uninstallation happens.
Section "Uninstall"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$INSTDIR\${EXECUTABLE}"
    RMDir "$INSTDIR"

    Call un.startMenuShortcut

    ; Cleanup Registry
    DeleteRegKey SHCTX "${APP_KEY}"
    DeleteRegKey /ifempty SHCTX "Software\${MANUFACTURER}"
    Call un.appPath
    Call un.uninstaller
    Call un.registeredApplication
SectionEnd

Function startMenuShortcut
    ; Start menu shortcuts
    ; The begin/end macro takes care of checking if the user actually wants a shortcut
    ; as configured in the start menu page.
    !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
        CreateDirectory "$SMPROGRAMS\$StartMenuFolder"
        CreateShortcut "$SMPROGRAMS\$StartMenuFolder\${PRODUCT}.lnk" "$INSTDIR\${EXECUTABLE}"
    !insertmacro MUI_STARTMENU_WRITE_END
FunctionEnd

Function un.startMenuShortcut
	!insertmacro MUI_STARTMENU_GETFOLDER Application $StartMenuFolder
    Delete "$SMPROGRAMS\$StartMenuFolder\${PRODUCT}.lnk"
    RMDir "$SMPROGRAMS\$StartMenuFolder"
FunctionEnd

Function uninstaller
     ; Uninstaller Information (for "Add and Remove Programs" in the Settings)
    ; See https://nsis.sourceforge.io/Add_uninstall_information_to_Add/Remove_Programs
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayName" "$(^Name)"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXECUTABLE}"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "Publisher" "${MANUFACTURER}"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
    WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "NoModify" 1
    WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "NoRepair" 1
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "URLInfoAbout" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "URLUpdateInfo" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "HelpLink" "${HOMEPAGE}"
    ; Computes the size of our installed files
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "EstimatedSize" "$0"
FunctionEnd

Function un.uninstaller
    DeleteRegKey SHCTX "${UNINSTALL_KEY}"
FunctionEnd

Function appPath
    ; Register our executable with App Paths, so that
    ; users can run it from anywhere without changing their PATH env var.
    ; See: https://learn.microsoft.com/en-us/windows/win32/shell/app-registration#registering-applications
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXECUTABLE_ALIAS}" "" "$INSTDIR\${EXECUTABLE}"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXECUTABLE_ALIAS}" "Path" "$INSTDIR"
FunctionEnd

Function un.appPath
    DeleteRegValue SHCTX "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXECUTABLE_ALIAS}" ""
    DeleteRegValue SHCTX "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXECUTABLE_ALIAS}" "Path"
    DeleteRegKey /ifempty SHCTX "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXECUTABLE_ALIAS}"
FunctionEnd

Function uriScheme
	; Registers the wormhole-transfer URI protocol
	; See <https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/aa767914(v=vs.85)?redirectedfrom=MSDN>.
	; We intentionally don't uninstall it as another application might stil use it.
	WriteRegStr SHCTX "Software\Classes\wormhole-transfer" "" "URL:Wormhole Transfer"
	WriteRegStr SHCTX "Software\Classes\wormhole-transfer" "URL Protocol" ""
FunctionEnd

Function registeredApplication
	; This registers our application so that we can be 
	; chosen as the default handler for the wormhole-transfer protocol
	; See: <https://learn.microsoft.com/en-gb/windows/win32/shell/default-programs#registering-an-application-for-use-with-default-programs>
	WriteRegStr SHCTX "${APP_KEY}\Capabilities" "ApplicationName" "${PRODUCT}"
	WriteRegStr SHCTX "${APP_KEY}\Capabilities" "ApplicationDescription" "${PRODUCT}"
	WriteRegStr SHCTX "${APP_KEY}\Capabilities" "ApplicationIcon" "$INSTDIR\${EXECUTABLE},1"
	WriteRegStr SHCTX "${APP_KEY}\Capabilities\UrlAssociations" "wormhole-transfer" "${UNIQUE_PRODUCT}.wormhole-transfer" ; This points to our "protocol handler"
	WriteRegStr SHCTX "Software\RegisteredApplications" "${UNIQUE_PRODUCT}" "${APP_KEY}\Capabilities"

	; protcol handler
	WriteRegStr SHCTX "Software\Classes\${UNIQUE_PRODUCT}.wormhole-transfer" "" "URL:Wormhole Transfer"
	WriteRegStr SHCTX "Software\Classes\${UNIQUE_PRODUCT}.wormhole-transfer" "URL Protocol" ""
	WriteRegStr SHCTX "Software\Classes\${UNIQUE_PRODUCT}.wormhole-transfer" "DefaultIcon" "$INSTDIR\${EXECUTABLE},1"
	WriteRegStr SHCTX "Software\Classes\${UNIQUE_PRODUCT}.wormhole-transfer\shell\Open\command" "" "$\"$INSTDIR\${EXECUTABLE}$\" -- $\"%1$\""
FunctionEnd

Function un.registeredApplication
	DeleteRegKey SHCTX "Software\Classes\${UNIQUE_PRODUCT}.wormhole-transfer"
	DeleteRegValue SHCTX "Software\RegisteredApplications" "${UNIQUE_PRODUCT}"
FunctionEnd