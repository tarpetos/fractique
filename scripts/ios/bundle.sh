#!/bin/sh
# Builds Fractique.app for iOS: scripts/ios/bundle.sh [aarch64-apple-ios-sim | aarch64-apple-ios]
set -eu
cd "$(dirname "$0")/../.."

target="${1:-aarch64-apple-ios-sim}"
case "$target" in
    *-sim) platform=iPhoneSimulator ;;
    *) platform=iPhoneOS ;;
esac
version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)"

rustup target add "$target"
IPHONEOS_DEPLOYMENT_TARGET=15.0 cargo build --release --locked -p fractique-app --bin fractique-gui --target "$target"

app="target/ios/$target/Fractique.app"
rm -rf "$app"
mkdir -p "$app"
cp "target/$target/release/fractique-gui" "$app/Fractique"
cat > "$app/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>Fractique</string>
    <key>CFBundleIdentifier</key><string>app.fractique</string>
    <key>CFBundleName</key><string>Fractique</string>
    <key>CFBundleDisplayName</key><string>Fractique</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleSupportedPlatforms</key><array><string>$platform</string></array>
    <key>MinimumOSVersion</key><string>15.0</string>
    <key>LSRequiresIPhoneOS</key><true/>
    <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
    <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string><string>metal</string></array>
    <key>UILaunchScreen</key><dict/>
    <key>UIRequiresFullScreen</key><false/>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
EOF
echo "$app"
