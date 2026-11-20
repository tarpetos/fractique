#!/bin/sh
# Builds the Linux packages and the command line tool and the two servers on their own into dist/:
# scripts/build/build-linux.sh [--formats appimage,deb]
# With CARGO_PACKAGER_SIGN_PRIVATE_KEY (and _PASSWORD) set, the AppImage gets the .sig the app's updater checks,
# and each program on its own the .sig its "update" command checks.
set -eu
cd "$(dirname "$0")/../.."

formats="appimage,deb"
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
FRACTIQUE_RELEASE=1 cargo build --release --locked -p fractique-app -p fractique-cli -p fractique-server

# linuxdeploy's own strip fails on current toolchains, and its AppImage needs no FUSE this way.
NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 cargo packager -c crates/fractique-app/Packager.toml --formats "$formats"

# The command line tool and the two servers on their own, for a machine that wants no app.
arch="$(uname -m)-linux"
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
ls -l dist
