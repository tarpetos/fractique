#!/bin/sh
# Builds Fractique and starts it in the iOS Simulator: scripts/ios/run-simulator.sh
# Needs a Mac with Xcode. No Apple account and no signing.
set -eu
cd "$(dirname "$0")/../.."

app="$(./scripts/ios/bundle.sh aarch64-apple-ios-sim)"
codesign --force --sign - "$app"

device="$(xcrun simctl list devices booted | sed -nE 's/.*\(([0-9A-F-]{36})\) \(Booted\).*/\1/p' | head -n1)"
if [ -z "$device" ]; then
    device="$(xcrun simctl list devices available | sed -nE 's/^ +iPhone[^(]*\(([0-9A-F-]{36})\).*/\1/p' | head -n1)"
    if [ -z "$device" ]; then
        echo "[ios] No iPhone simulator is installed: add one in Xcode, Settings, Platforms." >&2
        exit 1
    fi
    xcrun simctl boot "$device"
fi
open -a Simulator
xcrun simctl bootstatus "$device" -b >/dev/null
xcrun simctl install "$device" "$app"
echo "[ios] Fractique is starting in the simulator. What it prints follows; Ctrl+C stops watching."
xcrun simctl launch --console-pty "$device" "${FRACTIQUE_BUNDLE_ID:-app.fractique}"
