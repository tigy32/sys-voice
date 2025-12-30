#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TARGET=${1:-simulator}

echo "=== Building sys-voice for iOS ==="
echo ""

if [ "$TARGET" = "device" ]; then
    echo "Target: iOS Device (aarch64-apple-ios)"
    RUST_TARGET="aarch64-apple-ios"
    XCODE_SDK="iphoneos"
elif [ "$TARGET" = "simulator" ] || [ "$TARGET" = "sim" ]; then
    echo "Target: iOS Simulator (aarch64-apple-ios-sim)"
    RUST_TARGET="aarch64-apple-ios-sim"
    XCODE_SDK="iphonesimulator"
    SIMULATOR_NAME="iPhone 16"
else
    echo "Usage: ./build.sh [device|simulator]"
    exit 1
fi

# Step 1: Build Rust library
echo ""
echo "=== Step 1: Building Rust library ==="
cd "$PROJECT_ROOT"
rustup target add "$RUST_TARGET" 2>/dev/null || true
cargo build --release --target "$RUST_TARGET" --manifest-path "$SCRIPT_DIR/Cargo.toml"

LIB_PATH="$SCRIPT_DIR/target/$RUST_TARGET/release/libsys_voice_ffi.a"
if [ ! -f "$LIB_PATH" ]; then
    echo "ERROR: Library not found at $LIB_PATH"
    exit 1
fi
echo "✓ Library built: $LIB_PATH"

# Step 2: Build Xcode project
echo ""
echo "=== Step 2: Building Xcode project ==="
cd "$SCRIPT_DIR/VoiceTest"

# Run xcodebuild and capture result
set -o pipefail
if xcodebuild \
    -project VoiceTest.xcodeproj \
    -scheme VoiceTest \
    -sdk "$XCODE_SDK" \
    -configuration Debug \
    LIBRARY_SEARCH_PATHS="$SCRIPT_DIR/target/$RUST_TARGET/release" \
    build 2>&1 | tee /tmp/xcodebuild.log | grep -E '(error:|warning:|note:|Build Succeeded|BUILD FAILED)'; then
    echo "✓ Xcode build succeeded"
else
    echo ""
    echo "Build failed. Full output:"
    cat /tmp/xcodebuild.log | tail -80
    exit 1
fi

if [ "$TARGET" = "simulator" ] || [ "$TARGET" = "sim" ]; then
    echo ""
    echo "=== Step 3: Launching in Simulator ==="
    
    # Get build directory
    BUILD_DIR=$(xcodebuild -project VoiceTest.xcodeproj -scheme VoiceTest -showBuildSettings -sdk iphonesimulator 2>/dev/null | grep -m1 'BUILT_PRODUCTS_DIR' | awk '{print $3}')
    APP_PATH="$BUILD_DIR/VoiceTest.app"
    
    if [ ! -d "$APP_PATH" ]; then
        echo "ERROR: App not found at $APP_PATH"
        exit 1
    fi
    echo "App: $APP_PATH"
    
    # Boot simulator
    echo "Booting simulator: $SIMULATOR_NAME"
    xcrun simctl boot "$SIMULATOR_NAME" 2>/dev/null || true
    open -a Simulator
    sleep 3
    
    # Install and launch
    echo "Installing app..."
    xcrun simctl install booted "$APP_PATH"
    
    echo "Launching app (console output below)..."
    echo "========================================"
    xcrun simctl launch --console booted com.example.VoiceTest
else
    echo ""
    echo "=== Device build complete ==="
    echo "Open Xcode to deploy: open VoiceTest.xcodeproj"
fi
