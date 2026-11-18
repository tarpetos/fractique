#!/bin/sh
# Xcode compiles stub.c only so that it has an executable to sign; the Rust binary replaces it.
set -eu
cd "$SRCROOT/.."
case "$PLATFORM_NAME" in
    iphonesimulator) target=aarch64-apple-ios-sim ;;
    *) target=aarch64-apple-ios ;;
esac
PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
unset SDKROOT
export IPHONEOS_DEPLOYMENT_TARGET=15.0
rustup target add "$target" >/dev/null
cargo build --release --locked -p fractique-app --bin fractique-gui --features store --target "$target"
cp "target/$target/release/fractique-gui" "$TARGET_BUILD_DIR/$EXECUTABLE_PATH"
