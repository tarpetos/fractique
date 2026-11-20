#!/bin/sh
# Builds Fractique, signs it for the App Store and uploads it to TestFlight: scripts/ios/testflight.sh
# Needs xcodegen (brew install xcodegen), an App Store Connect app record for the bundle identifier, and either
# Xcode signed in to the team's Apple account or an App Store Connect API key in ASC_KEY_PATH (the .p8 file),
# ASC_KEY_ID and ASC_ISSUER_ID. FRACTIQUE_TEAM_ID picks the team.
set -eu
cd "$(dirname "$0")/../../ios"

team="${FRACTIQUE_TEAM_ID:-73L32W9C88}"
# In UTC, so that a build from GitHub and one from a Mac in another time zone keep counting upwards.
build="$(date -u +%y%m%d%H%M)"
set -- -allowProvisioningUpdates
if [ -n "${ASC_KEY_PATH:-}" ]; then
    set -- "$@" -authenticationKeyPath "$ASC_KEY_PATH" -authenticationKeyID "$ASC_KEY_ID" \
        -authenticationKeyIssuerID "$ASC_ISSUER_ID"
fi
rm -rf build
xcodegen generate --quiet
xcodebuild -quiet -project Fractique.xcodeproj -scheme Fractique -configuration Release \
    -destination generic/platform=iOS -archivePath build/Fractique.xcarchive "$@" \
    DEVELOPMENT_TEAM="$team" CURRENT_PROJECT_VERSION="$build" archive
xcodebuild -exportArchive -archivePath build/Fractique.xcarchive -exportOptionsPlist ExportOptions.plist \
    -exportPath build/export "$@" DEVELOPMENT_TEAM="$team"
echo "[ios] Uploaded build $build. TestFlight shows it after Apple processes it (5-30 minutes)."
