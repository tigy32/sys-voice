# Android Voice Capture Example

Minimal Android app demonstrating sys-voice audio capture with AEC.

## Prerequisites

- Android Studio with NDK installed
- Rust with Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
  ```
- cargo-ndk:
  ```bash
  cargo install cargo-ndk
  ```

## Quick Start

### Build the Native Library

```bash
# Set NDK path (if not auto-detected)
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/26.1.10909125

# Build for all architectures
./build.sh
```

This builds `.so` files for arm64-v8a, armeabi-v7a, and x86_64, and copies them to `app/src/main/jniLibs/`.

### Run the App

1. Open `android-example/` in Android Studio
2. Connect a device or start an emulator
3. Run the app (Shift+F10)

## Project Structure

```
android-example/
├── Cargo.toml                    # Rust FFI crate
├── src/lib.rs                    # JNI exports
├── build.sh                      # Build script
├── build.gradle.kts              # Root Gradle config
├── settings.gradle.kts           # Gradle settings
└── app/
    ├── build.gradle.kts          # App module config
    └── src/main/
        ├── AndroidManifest.xml
        ├── jniLibs/              # Built .so files go here
        └── java/com/example/sysvoice/
            ├── MainActivity.kt
            └── VoiceCapture.kt   # JNI wrapper
```

## JNI Interface

The Rust library exports these JNI functions:

```kotlin
external fun nativeStart(sampleRate: Int): Int
external fun nativeRecv(buffer: FloatArray): Int
external fun nativeGetSampleRate(): Int
external fun nativeStop()
```

## Important Notes

- **Permissions required** - App needs `RECORD_AUDIO` permission
- **NDK version** - Tested with NDK r26, other versions should work

## ⚠️ Android Emulator Microphone Setup

**The Android emulator disables microphone input by default.** Without enabling it, you will only hear static/crackling instead of recorded audio.

### To enable microphone in emulator:

1. Open the emulator's **Extended Controls** (click `...` button on emulator toolbar)
2. Go to **Microphone** section
3. Enable **"Virtual microphone uses host audio input"**
4. Ensure your Mac's microphone permissions are granted for the emulator

### Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Recording shows level values but playback is static | Emulator mic disabled | Enable in Extended Controls → Microphone |
| Test tone works but recording doesn't | Emulator mic disabled | Enable in Extended Controls → Microphone |
| No audio at all | Emulator volume muted | Press volume up on emulator |
| Slightly static audio quality | Emulator limitation | Normal - emulator audio passthrough isn't perfect |

## Troubleshooting

### "NDK not found"
Set `ANDROID_NDK_HOME` to your NDK installation path:
```bash
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/26.1.10909125
```

### "UnsatisfiedLinkError"
Ensure the `.so` files are in the correct `jniLibs/<abi>/` directories and the library name matches.
