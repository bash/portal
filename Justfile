set windows-shell := ["powershell"]

default:
    just --list

build-windows-installer:
    dotnet tool install --global wix --version 5.0.1
    wix extension add WixToolset.UI.wixext/5.0.1 --global
    wix build \
        build/windows/installer/Package.wxs \
        build/windows/installer/WixUI_InstallDir.wxs \
        build/windows/installer/Package.en-us.wxl \
        -ext WixToolset.UI.wixext \
        -o target/windows-installer/portal-installer \
        -bindpath build/windows/installer
