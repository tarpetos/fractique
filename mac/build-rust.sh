#!/bin/sh
# Xcode compiles stub.c only so that it has an executable to sign; the Rust binary replaces it. cargo started by
# Xcode cannot compile the C of a dependency, so scripts/mac/appstore.sh builds the binary before Xcode runs.
set -eu
built="$SRCROOT/../target/mac-app-store/fractique-gui"
if [ ! -f "$built" ]; then
    echo "error: $built is missing: run scripts/mac/appstore.sh, which builds it." >&2
    exit 1
fi
cp "$built" "$TARGET_BUILD_DIR/$EXECUTABLE_PATH"
