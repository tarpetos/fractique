#!/bin/sh
# Builds Fractique for the Mac App Store, sandboxed and with the purchase, and uploads it to App Store Connect:
#   scripts/mac/appstore.sh          build, sign for the App Store, upload
#   scripts/mac/appstore.sh --run    build, sign for this Mac and open the app, to try it in its sandbox
# Needs xcodegen (brew install xcodegen), the Mac platform added to the app record of the bundle identifier, and
# either Xcode signed in to the team's Apple account or an App Store Connect API key in ASC_KEY_PATH (the .p8
# file), ASC_KEY_ID and ASC_ISSUER_ID. FRACTIQUE_TEAM_ID picks the team.
set -eu
cd "$(dirname "$0")/../.."
mode="${1:-upload}"

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)"
mac="$(sed -n 's/^ *MARKETING_VERSION: *"\(.*\)"/\1/p' mac/project.yml | head -n1)"
if [ "$version" != "$mac" ]; then
    echo "[mac] Cargo.toml says $version, mac/project.yml says $mac: make them equal." >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
    export PATH
fi
# One file with the code for Apple Silicon and for Intel. Without FRACTIQUE_RELEASE the app does not update
# itself: the App Store does that.
parts=""
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$target" >/dev/null
    MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --release --locked -p fractique-app --bin fractique-gui \
        --features store --target "$target"
    parts="$parts target/$target/release/fractique-gui"
done
mkdir -p target/mac-app-store
# shellcheck disable=SC2086
lipo -create -output target/mac-app-store/fractique-gui $parts

cd mac
team="${FRACTIQUE_TEAM_ID:-73L32W9C88}"
set -- -allowProvisioningUpdates
if [ -n "${ASC_KEY_PATH:-}" ]; then
    set -- "$@" -authenticationKeyPath "$ASC_KEY_PATH" -authenticationKeyID "$ASC_KEY_ID" \
        -authenticationKeyIssuerID "$ASC_ISSUER_ID"
fi
rm -rf build
xcodegen generate --quiet

if [ "$mode" = "--run" ]; then
    xcodebuild -quiet -project Fractique.xcodeproj -scheme Fractique -configuration Debug \
        -derivedDataPath build/run "$@" DEVELOPMENT_TEAM="$team" build
    open build/run/Build/Products/Debug/Fractique.app
    exit 0
fi

# In UTC, so that a build from GitHub and one from a Mac in another time zone keep counting upwards.
build="$(date -u +%y%m%d%H%M)"
xcodebuild -quiet -project Fractique.xcodeproj -scheme Fractique -configuration Release \
    -destination generic/platform=macOS -archivePath build/Fractique.xcarchive "$@" \
    DEVELOPMENT_TEAM="$team" CURRENT_PROJECT_VERSION="$build" archive
xcodebuild -exportArchive -archivePath build/Fractique.xcarchive -exportOptionsPlist ExportOptions.plist \
    -exportPath build/export "$@" DEVELOPMENT_TEAM="$team"
echo "[mac] Uploaded build $build. App Store Connect shows it after Apple processes it (5-30 minutes)."
