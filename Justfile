PluginName       := "TweakShader"
BundleIdentifier := "com.adobe.AfterEffects.{{PluginName}}"
BinaryName       := lowercase(PluginName)
CrateName        := "tweak_shader_ae_rs" 

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

TargetDir := env_var_or_default("CARGO_TARGET_DIR", "target")
export AESDK_ROOT := if env("AESDK_ROOT", "") == "" { justfile_directory() / "../../sdk/AfterEffectsSDK" } else { env_var("AESDK_ROOT") }
export PRSDK_ROOT := if env("PRSDK_ROOT", "") == "" { justfile_directory() / "../../sdk/Premiere Pro 22.0 C++ SDK" } else { env_var("PRSDK_ROOT") }


[windows]
build:
    cargo build
    Start-Process PowerShell -Verb runAs -ArgumentList "-command Copy-Item -Force '{{TargetDir}}\debug\{{CrateName}}.dll' 'C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\{{PluginName}}.aex'"

[windows]
release:
    cargo build --release
    Copy-Item -Force '{{TargetDir}}\release\{{CrateName}}.dll' '{{TargetDir}}\release\{{PluginName}}.aex'

[macos]
build:
    just -f {{justfile()}} create_bundle debug {{TargetDir}} 'Apple Development' ''

[macos]
release:
    just -f {{justfile()}} create_bundle release {{TargetDir}} 'Developer ID Application' --release

[macos]
create_bundle BuildType TargetDir CertType BuildFlags:
    echo "Creating universal plugin bundle"
    rm -Rf {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin
    mkdir -p {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/Resources
    mkdir -p {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/MacOS

    rustup target add aarch64-apple-darwin
    rustup target add x86_64-apple-darwin

    cargo build {{BuildFlags}} --target x86_64-apple-darwin
    cargo build {{BuildFlags}} --target aarch64-apple-darwin

    echo "eFKTFXTC" >> {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/PkgInfo
    /usr/libexec/PlistBuddy -c 'add CFBundlePackageType string eFKT' {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/Info.plist
    /usr/libexec/PlistBuddy -c 'add CFBundleSignature string FXTC' {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/Info.plist
    /usr/libexec/PlistBuddy -c 'add CFBundleIdentifier string {{BundleIdentifier}}' {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/Info.plist

    cp {{TargetDir}}/{{BuildType}}/{{CrateName}}.rsrc {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/Resources/{{PluginName}}.rsrc

    lipo {{TargetDir}}/{x86_64,aarch64}-apple-darwin/{{BuildType}}/lib{{CrateName}}.dylib -create -output {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/MacOS/{{BinaryName}}.dylib
    mv {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/MacOS/{{BinaryName}}.dylib {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin/Contents/MacOS/{{PluginName}}

    codesign --options runtime --timestamp -strict  --sign $( security find-identity -v -p codesigning | grep -m 1 "{{CertType}}" | awk -F ' ' '{print $2}' ) {{TargetDir}}/{{BuildType}}/{{PluginName}}.plugin
