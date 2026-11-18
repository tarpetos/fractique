#!/bin/sh
# Builds Fractique, signs it for the App Store and uploads it to TestFlight: scripts/ios/testflight.sh
# Needs Xcode signed in to the team's Apple account, xcodegen (brew install xcodegen) and an App Store Connect app
# record for the bundle identifier. FRACTIQUE_TEAM_ID picks the team.
set -eu
cd "$(dirname "$0")/../../ios"

team="${FRACTIQUE_TEAM_ID:-73L32W9C88}"
build="$(date +%y%m%d%H%M)"
rm -rf build
xcodegen generate --quiet
xcodebuild -quiet -project Fractique.xcodeproj -scheme Fractique -configuration Release \
    -destination generic/platform=iOS -archivePath build/Fractique.xcarchive -allowProvisioningUpdates \
    DEVELOPMENT_TEAM="$team" CURRENT_PROJECT_VERSION="$build" archive
xcodebuild -exportArchive -archivePath build/Fractique.xcarchive -exportOptionsPlist ExportOptions.plist \
    -exportPath build/export -allowProvisioningUpdates DEVELOPMENT_TEAM="$team"
echo "[ios] Uploaded build $build. TestFlight shows it after Apple processes it (5-30 minutes)."
