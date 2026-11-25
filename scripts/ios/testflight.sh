#!/bin/sh
# Builds Fractique, signs it for the App Store and uploads it to TestFlight:
#   scripts/ios/testflight.sh            build, sign, upload; only in a clean checkout of the version's tag
#   scripts/ios/testflight.sh --export   build and sign into ios/build/export, upload nothing
# Needs xcodegen (brew install xcodegen), an App Store Connect app record for the bundle identifier, and either
# Xcode signed in to the team's Apple account or an App Store Connect API key in ASC_KEY_PATH (the .p8 file),
# ASC_KEY_ID and ASC_ISSUER_ID. FRACTIQUE_TEAM_ID picks the team.
set -eu
cd "$(dirname "$0")/../../ios"
mode="${1:-upload}"

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' ../Cargo.toml | head -n1)"
ios="$(sed -n 's/^ *MARKETING_VERSION: *"\(.*\)"/\1/p' project.yml | head -n1)"
if [ "$version" != "$ios" ]; then
    echo "[ios] Cargo.toml says $version, ios/project.yml says $ios: make them equal." >&2
    exit 1
fi

# An upload from a Mac is made from a clean checkout of the tag, as the Release workflow's is (docs/releasing.md).
if [ "$mode" = "upload" ]; then
    if [ "$(git describe --exact-match --tags HEAD 2>/dev/null || true)" != "v$version" ] || [ -n "$(git status --porcelain)" ]; then
        echo "[ios] An upload is made from a clean checkout of the tag v$version; --export builds and signs without uploading." >&2
        exit 1
    fi
fi

team="${FRACTIQUE_TEAM_ID:-73L32W9C88}"
# In UTC, so that a build from GitHub and one from a Mac in another time zone keep counting upwards.
build="$(date -u +%y%m%d%H%M)"
set -- -allowProvisioningUpdates
if [ -n "${ASC_KEY_PATH:-}" ]; then
    set -- "$@" -authenticationKeyPath "$ASC_KEY_PATH" -authenticationKeyID "$ASC_KEY_ID" \
        -authenticationKeyIssuerID "$ASC_ISSUER_ID"
fi
rm -rf build
mkdir build
options=ExportOptions.plist
if [ "$mode" = "--export" ]; then
    options=build/ExportOptions.plist
    cp ExportOptions.plist "$options"
    plutil -replace destination -string export "$options"
fi
xcodegen generate --quiet
# FRACTIQUE_SIGN_IDENTITY: the Release workflow signs the archive with the team's distribution certificate.
xcodebuild -quiet -project Fractique.xcodeproj -scheme Fractique -configuration Release \
    -destination generic/platform=iOS -archivePath build/Fractique.xcarchive "$@" \
    DEVELOPMENT_TEAM="$team" CURRENT_PROJECT_VERSION="$build" \
    ${FRACTIQUE_SIGN_IDENTITY:+"CODE_SIGN_IDENTITY=$FRACTIQUE_SIGN_IDENTITY"} archive
xcodebuild -exportArchive -archivePath build/Fractique.xcarchive -exportOptionsPlist "$options" \
    -exportPath build/export "$@" DEVELOPMENT_TEAM="$team"
if [ "$mode" = "--export" ]; then
    echo "[ios] Build $build is in ios/build/export, not uploaded."
else
    echo "[ios] Uploaded build $build. TestFlight shows it after Apple processes it (5-30 minutes)."
fi
