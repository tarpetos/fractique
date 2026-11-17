#!/bin/sh
# Builds the macOS app and disk image into dist/: scripts/build/build-mac.sh [--formats app,dmg]
# With CARGO_PACKAGER_SIGN_PRIVATE_KEY (and _PASSWORD) set, Fractique.app.tar.gz gets the .sig the app's updater
# checks. The app is not signed by Apple: copied from the .dmg it needs
#   xattr -dr com.apple.quarantine /Applications/Fractique.app
set -eu
cd "$(dirname "$0")/../.."

formats="app,dmg"
if [ "${1:-}" = "--formats" ]; then
    formats="$2"
fi

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)"
packaged="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' crates/fractique-app/Packager.toml | head -n1)"
if [ "$version" != "$packaged" ]; then
    echo "[build] Cargo.toml says $version, Packager.toml says $packaged: make them equal." >&2
    exit 1
fi

command -v cargo-packager >/dev/null 2>&1 || cargo install cargo-packager --locked
FRACTIQUE_RELEASE=1 cargo build --release --locked -p fractique-app -p fractique-cli
cargo packager -c crates/fractique-app/Packager.toml --formats "$formats"

if [ -d dist/Fractique.app ] && [ ! -f dist/Fractique.app.tar.gz ]; then
    tar -czf dist/Fractique.app.tar.gz -C dist Fractique.app
fi
if [ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ] && [ -f dist/Fractique.app.tar.gz ] && [ ! -f dist/Fractique.app.tar.gz.sig ]; then
    cargo packager signer sign dist/Fractique.app.tar.gz
fi
ls -l dist
