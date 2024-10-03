set windows-shell := ["powershell"]

default:
    just --list

build-windows-installer:
    &"C:\Program Files (x86)\NSIS\makensis.exe" ".\build\windows\installer.nsi"
