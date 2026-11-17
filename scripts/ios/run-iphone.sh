#!/bin/sh
# Builds Fractique, signs it and starts it on the iPhone connected by cable: scripts/ios/run-iphone.sh
# With --ipa it writes target/ios/Fractique.ipa instead; with --unsigned-ipa that file is not signed.
# Needs a Mac with Xcode 15 or newer, an Apple Development certificate in the keychain, and a provisioning profile
# that covers this app and this iPhone. docs/ios.md says how to get them.
set -eu
cd "$(dirname "$0")/../.."

bundle_id="${FRACTIQUE_BUNDLE_ID:-app.fractique}"
work="target/ios/signing"
mkdir -p "$work"
mode="${1:-run}"

pack() {
    rm -rf target/ios/Payload target/ios/Fractique.ipa
    mkdir -p target/ios/Payload
    cp -R "$1" target/ios/Payload/
    (cd target/ios && zip -qry Fractique.ipa Payload)
    rm -rf target/ios/Payload
    echo "[ios] Wrote target/ios/Fractique.ipa"
}

if [ "$mode" = "--unsigned-ipa" ]; then
    pack "$(./scripts/ios/bundle.sh aarch64-apple-ios)"
    exit 0
fi

identity="${FRACTIQUE_SIGN_IDENTITY:-$(security find-identity -v -p codesigning | sed -nE 's/.*"(Apple Development: [^"]+)".*/\1/p' | head -n1)}"
if [ -z "$identity" ]; then
    echo "[ios] No 'Apple Development' certificate in the keychain. See docs/ios.md, 'Signing'." >&2
    exit 1
fi

profile=""
for candidate in "$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"/*.mobileprovision "$HOME/Library/MobileDevice/Provisioning Profiles"/*.mobileprovision; do
    [ -f "$candidate" ] || continue
    security cms -D -i "$candidate" > "$work/candidate.plist" 2>/dev/null || continue
    app_id="$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:application-identifier' "$work/candidate.plist" 2>/dev/null || true)"
    expires="$(/usr/libexec/PlistBuddy -c 'Print :ExpirationDate' "$work/candidate.plist" 2>/dev/null || true)"
    if [ -n "$expires" ] && [ "$(date -j -f '%a %b %d %T %Z %Y' "$expires" +%s 2>/dev/null || echo 9999999999)" -lt "$(date +%s)" ]; then
        continue
    fi
    case "$app_id" in
        *".$bundle_id" | *".*")
            profile="$candidate"
            cp "$work/candidate.plist" "$work/profile.plist"
            case "$app_id" in *".$bundle_id") break ;; esac
            ;;
    esac
done
if [ -z "$profile" ]; then
    echo "[ios] No provisioning profile covers $bundle_id. See docs/ios.md, 'Signing'." >&2
    exit 1
fi
echo "[ios] Signing as: $identity"
echo "[ios] Profile:    $(/usr/libexec/PlistBuddy -c 'Print :Name' "$work/profile.plist")"

app="$(./scripts/ios/bundle.sh aarch64-apple-ios)"
cp "$profile" "$app/embedded.mobileprovision"
/usr/libexec/PlistBuddy -x -c 'Print :Entitlements' "$work/profile.plist" > "$work/entitlements.plist"
team="$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:com.apple.developer.team-identifier' "$work/profile.plist")"
/usr/libexec/PlistBuddy -c "Set :application-identifier $team.$bundle_id" "$work/entitlements.plist"
codesign --force --sign "$identity" --entitlements "$work/entitlements.plist" "$app"

if [ "$mode" = "--ipa" ]; then
    pack "$app"
    echo "[ios] It installs only on the iPhones listed in the profile above."
    exit 0
fi

device="${FRACTIQUE_DEVICE:-$(xcrun devicectl list devices 2>/dev/null | sed -nE 's/.* ([0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}) +(available|connected).*/\1/p' | head -n1)}"
if [ -z "$device" ]; then
    echo "[ios] No iPhone found. Connect it by cable, unlock it, tap Trust, and turn on Developer Mode" >&2
    echo "[ios] (Settings, Privacy & Security, Developer Mode). 'xcrun devicectl list devices' shows what the Mac sees." >&2
    exit 1
fi
xcrun devicectl device install app --device "$device" "$app"
echo "[ios] Fractique is starting on the iPhone. What it prints follows; Ctrl+C stops watching."
xcrun devicectl device process launch --console --device "$device" "$bundle_id"
