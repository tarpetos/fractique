#!/bin/sh
# Builds Fractique.app for iOS: scripts/ios/bundle.sh [aarch64-apple-ios-sim | aarch64-apple-ios]
set -eu
cd "$(dirname "$0")/../.."

target="${1:-aarch64-apple-ios-sim}"
case "$target" in
    *-sim) platform=iPhoneSimulator ;;
    *) platform=iPhoneOS ;;
esac
bundle_id="${FRACTIQUE_BUNDLE_ID:-app.fractique}"
version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)"

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "[ios] Installing Rust (rustup)..." >&2
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none >&2
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi
rustup target add "$target" >&2
IPHONEOS_DEPLOYMENT_TARGET=15.0 cargo build --release --locked -p fractique-app --bin fractique-gui --target "$target" >&2

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
    <key>CFBundleIdentifier</key><string>$bundle_id</string>
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
    <key>NSPhotoLibraryAddUsageDescription</key><string>Fractique saves the pictures you export to your photo library.</string>
    <key>UIRequiresFullScreen</key><false/>
    <key>UIApplicationSceneManifest</key>
    <dict>
        <key>UIApplicationSupportsMultipleScenes</key><false/>
        <key>UISceneConfigurations</key>
        <dict>
            <key>UIWindowSceneSessionRoleApplication</key>
            <array><dict><key>UISceneConfigurationName</key><string>Default</string></dict></array>
        </dict>
    </dict>
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
