#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Building sys-voice for Android ==="

# Check for cargo-ndk
if ! command -v cargo-ndk &> /dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Add Android targets if needed
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android 2>/dev/null || true

# Check for NDK
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Try common locations
    if [ -d "$HOME/Library/Android/sdk/ndk" ]; then
        export ANDROID_NDK_HOME=$(ls -d "$HOME/Library/Android/sdk/ndk"/*/ 2>/dev/null | head -1)
    elif [ -d "$HOME/Android/Sdk/ndk" ]; then
        export ANDROID_NDK_HOME=$(ls -d "$HOME/Android/Sdk/ndk"/*/ 2>/dev/null | head -1)
    fi
fi

if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "ERROR: ANDROID_NDK_HOME not set and NDK not found in common locations."
    echo ""
    echo "Install Android NDK via Android Studio or:"
    echo "  sdkmanager 'ndk;26.1.10909125'"
    echo ""
    echo "Then set: export ANDROID_NDK_HOME=/path/to/ndk"
    exit 1
fi

echo "Using NDK: $ANDROID_NDK_HOME"

# Build for all Android architectures
echo ""
echo "=== Building for arm64-v8a ==="
cargo ndk -t arm64-v8a --manifest-path "$SCRIPT_DIR/Cargo.toml" build --release

echo ""
echo "=== Building for armeabi-v7a ==="
cargo ndk -t armeabi-v7a --manifest-path "$SCRIPT_DIR/Cargo.toml" build --release

echo ""
echo "=== Building for x86_64 (emulator) ==="
cargo ndk -t x86_64 --manifest-path "$SCRIPT_DIR/Cargo.toml" build --release

# Copy libraries to jniLibs
JNILIBS="$SCRIPT_DIR/app/src/main/jniLibs"
mkdir -p "$JNILIBS/arm64-v8a" "$JNILIBS/armeabi-v7a" "$JNILIBS/x86_64"

cp "$SCRIPT_DIR/target/aarch64-linux-android/release/libsys_voice_android_ffi.so" "$JNILIBS/arm64-v8a/"
cp "$SCRIPT_DIR/target/armv7-linux-androideabi/release/libsys_voice_android_ffi.so" "$JNILIBS/armeabi-v7a/"
cp "$SCRIPT_DIR/target/x86_64-linux-android/release/libsys_voice_android_ffi.so" "$JNILIBS/x86_64/"

# Copy libc++_shared.so from NDK (required for Oboe C++ runtime)
HOST_TAG="darwin-x86_64"  # macOS
if [ "$(uname)" = "Linux" ]; then
    HOST_TAG="linux-x86_64"
fi
NDK_STL="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG/sysroot/usr/lib"

cp "$NDK_STL/aarch64-linux-android/libc++_shared.so" "$JNILIBS/arm64-v8a/"
cp "$NDK_STL/arm-linux-androideabi/libc++_shared.so" "$JNILIBS/armeabi-v7a/"
cp "$NDK_STL/x86_64-linux-android/libc++_shared.so" "$JNILIBS/x86_64/"

echo ""
echo "=== Libraries built and copied to jniLibs ==="
ls -la "$JNILIBS"/*/*.so

echo ""
echo "=== Step 2: Building APK ==="
cd "$SCRIPT_DIR"

# Find gradle - prefer gradlew, fall back to system gradle or Android Studio's bundled gradle
if [ -x "./gradlew" ]; then
    GRADLE="./gradlew"
elif command -v gradle &> /dev/null; then
    GRADLE="gradle"
elif [ -d "/Applications/Android Studio.app" ]; then
    # Use Android Studio's bundled gradle
    GRADLE_DIR=$(ls -d "/Applications/Android Studio.app/Contents/gradle/gradle-"* 2>/dev/null | head -1)
    if [ -n "$GRADLE_DIR" ]; then
        GRADLE="$GRADLE_DIR/bin/gradle"
    fi
fi

if [ -z "$GRADLE" ]; then
    echo "No gradle found. Opening in Android Studio..."
    open -a "Android Studio" "$SCRIPT_DIR"
    echo ""
    echo "Android Studio will download gradle and build the project."
    echo "After it syncs, click Run (green play button) to deploy."
    exit 0
fi

echo "Using: $GRADLE"
$GRADLE assembleDebug --quiet

if [ $? -ne 0 ]; then
    echo "Gradle build failed. Try opening in Android Studio:"
    echo "  open -a 'Android Studio' $SCRIPT_DIR"
    exit 1
fi

APK_PATH="$SCRIPT_DIR/app/build/outputs/apk/debug/app-debug.apk"
echo "✓ APK built: $APK_PATH"

echo ""
echo "=== Step 3: Launching on device/emulator ==="

# Check if adb is available
if ! command -v adb &> /dev/null; then
    ADB="$ANDROID_HOME/platform-tools/adb"
else
    ADB="adb"
fi

# Check for connected devices
DEVICES=$($ADB devices | grep -v "List" | grep -v "^$" | wc -l)

if [ "$DEVICES" -eq 0 ]; then
    echo "No devices connected. Starting emulator..."
    
    # Find first available AVD
    if command -v emulator &> /dev/null; then
        EMULATOR="emulator"
    else
        EMULATOR="$ANDROID_HOME/emulator/emulator"
    fi
    
    AVD=$($EMULATOR -list-avds | head -1)
    
    if [ -z "$AVD" ]; then
        echo "No AVDs found. Create one in Android Studio:"
        echo "  Tools → Device Manager → Create Device"
        echo ""
        echo "Or open project manually:"
        echo "  open -a 'Android Studio' $SCRIPT_DIR"
        exit 1
    fi
    
    echo "Starting emulator: $AVD"
    $EMULATOR -avd "$AVD" &
    
    echo "Waiting for emulator to boot..."
    $ADB wait-for-device
    sleep 10  # Extra time for boot
fi

echo "Installing APK..."
$ADB install -r "$APK_PATH"

echo "Launching app..."
$ADB shell am start -n com.example.sysvoice/.MainActivity

echo ""
echo "=== App launched! ==="
echo "View logs with: adb logcat -s SysVoice:*"
echo ""
echo "Streaming logcat output (Ctrl+C to stop):"
echo "========================================"
$ADB logcat -s SysVoice:* *:S
