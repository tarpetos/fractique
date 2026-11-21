#!/bin/sh
# Builds the macOS app and disk image, and the command line tool and the two servers on their own, into dist/:
# scripts/build/build-mac.sh [--formats app,dmg]
# Every program is universal: one file with the code for Apple Silicon and for Intel, so one download runs on
# every Mac.
# With CARGO_PACKAGER_SIGN_PRIVATE_KEY (and _PASSWORD) set, the archive of the app gets the .sig the app's updater
# checks, and each program on its own the .sig its "update" command checks.
# The app is not signed by Apple: copied from the .dmg it needs
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
programs="fractique-gui fractique fractique-mcp fractique-server"
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$target"
    FRACTIQUE_RELEASE=1 cargo build --release --locked --target "$target" \
        -p fractique-app -p fractique-cli -p fractique-server
done
# cargo-packager takes the programs from target/release.
mkdir -p target/release
for program in $programs; do
    lipo -create -output "target/release/$program" \
        "target/aarch64-apple-darwin/release/$program" "target/x86_64-apple-darwin/release/$program"
    lipo -archs "target/release/$program"
done
cargo packager -c crates/fractique-app/Packager.toml --formats "$formats"
# cargo-packager names the disk image after the machine that builds it.
for image in dist/*.dmg; do
    [ -e "$image" ] || continue
    mv "$image" "dist/fractique-gui_${version}_universal.dmg"
done

# The command line tool and the two servers on their own, for a machine that wants no app.
arch=universal-macos
for program in fractique fractique-mcp fractique-server; do
    name="$program"
    if [ "$program" = fractique ]; then
        name=fractique-cli
    fi
    tar -czf "dist/${name}_${version}_${arch}.tar.gz" -C target/release "$program"
    if [ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ]; then
        cargo packager signer sign "dist/${name}_${version}_${arch}.tar.gz"
    fi
done

if [ -d dist/Fractique.app ] && [ ! -f dist/Fractique.app.tar.gz ]; then
    tar -czf dist/Fractique.app.tar.gz -C dist Fractique.app
fi
if [ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ] && [ -f dist/Fractique.app.tar.gz ] && [ ! -f dist/Fractique.app.tar.gz.sig ]; then
    cargo packager signer sign dist/Fractique.app.tar.gz
fi
for made in dist/Fractique.app.tar.gz dist/Fractique.app.tar.gz.sig; do
    [ -e "$made" ] || continue
    mv "$made" "dist/Fractique_${version}_universal${made#dist/Fractique}"
done
ls -l dist
