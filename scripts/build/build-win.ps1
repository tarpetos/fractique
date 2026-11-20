# Builds the Windows programs into dist\: .\scripts\build\build-win.ps1
# Windows gets no installer: the app is one file that replaces itself when it updates.
# With $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY (and _PASSWORD) set, the app gets the .sig its updater checks.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

$version = (Select-String -Path Cargo.toml -Pattern '^version *= *"(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
$packaged = (Select-String -Path crates\fractique-app\Packager.toml -Pattern '^version *= *"([^"]*)"' | Select-Object -First 1).Matches.Groups[1].Value
if ($version -ne $packaged) {
    throw "Cargo.toml says ${version}, Packager.toml says ${packaged}: make them equal."
}

$env:FRACTIQUE_RELEASE = '1'
cargo build --release --locked -p fractique-app -p fractique-cli -p fractique-server
if ($LASTEXITCODE -ne 0) { throw 'The build failed.' }

New-Item -ItemType Directory -Force dist | Out-Null
$app = "dist\Fractique_${version}_x64.exe"
Copy-Item target\release\fractique-gui.exe $app -Force
Copy-Item target\release\fractique.exe "dist\fractique-cli_${version}_x64.exe" -Force
Copy-Item target\release\fractique-mcp.exe "dist\fractique-mcp_${version}_x64.exe" -Force
Copy-Item target\release\fractique-server.exe "dist\fractique-server_${version}_x64.exe" -Force

if ($env:CARGO_PACKAGER_SIGN_PRIVATE_KEY) {
    if (-not (Get-Command cargo-packager -ErrorAction SilentlyContinue)) {
        cargo install cargo-packager --locked
        if ($LASTEXITCODE -ne 0) { throw 'cargo-packager could not be installed.' }
    }
    cargo packager signer sign $app
    if ($LASTEXITCODE -ne 0) { throw 'Signing failed.' }
}
Get-ChildItem dist
