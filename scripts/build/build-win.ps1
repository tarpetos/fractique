# Builds the Windows installer into dist\: .\scripts\build\build-win.ps1
# With $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY (and _PASSWORD) set, the setup .exe gets the .sig the app's updater checks.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

$version = (Select-String -Path Cargo.toml -Pattern '^version *= *"(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
$packaged = (Select-String -Path crates\fractique-app\Packager.toml -Pattern '^version *= *"([^"]*)"' | Select-Object -First 1).Matches.Groups[1].Value
if ($version -ne $packaged) {
    throw "Cargo.toml says $version, Packager.toml says $packaged: make them equal."
}

if (-not (Get-Command cargo-packager -ErrorAction SilentlyContinue)) {
    cargo install cargo-packager --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo-packager could not be installed.' }
}
$env:FRACTIQUE_RELEASE = '1'
cargo build --release --locked -p fractique-app -p fractique-cli
if ($LASTEXITCODE -ne 0) { throw 'The build failed.' }
cargo packager -c crates\fractique-app\Packager.toml --formats nsis
if ($LASTEXITCODE -ne 0) { throw 'Packaging failed.' }
Get-ChildItem dist
