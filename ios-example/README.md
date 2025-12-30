# iOS Voice Capture Example

Minimal iOS app demonstrating sys-voice audio capture with AEC (Acoustic Echo Cancellation).

## Prerequisites

- macOS with Xcode installed
- Rust iOS targets:
  ```bash
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim
  ```

## Quick Start

### Simulator
```bash
./build.sh sim
```

This will:
1. Build the Rust static library for iOS simulator (arm64)
2. Build the Xcode project
3. Boot the iPhone 16 simulator
4. Install and launch the app with console output

### Device (for real AEC testing)

Device builds require code signing with your Apple Developer Team ID:

```bash
# Set your team ID (find in Xcode → Preferences → Accounts)
TEAM_ID=YOUR_TEAM_ID ./build.sh device

# Or configure in Xcode:
open VoiceTest/VoiceTest.xcodeproj
# Project → Target → Signing & Capabilities → Select your team
# Then: Cmd+R to build and run
```

## Project Structure

```
ios-example/
├── README.md
├── build.sh                    # Automated build script
└── VoiceTest/
    ├── VoiceTest.xcodeproj/    # Xcode project
    └── VoiceTest/
        ├── VoiceTestApp.swift  # App entry point
        ├── ContentView.swift   # SwiftUI interface
        ├── VoiceCapture.swift  # Rust FFI wrapper
        ├── Bridge.h            # C function declarations
        └── Info.plist          # App configuration
```

## FFI Interface

The Rust library exports these C functions (declared in `Bridge.h`):

```c
void* capture_start(uint32_t sample_rate);
int32_t capture_recv(void* handle, float* buffer, size_t buffer_len);
uint32_t capture_sample_rate(void* handle);
void capture_stop(void* handle);
```

## Build Script Usage

```bash
./build.sh sim        # Build and launch in simulator
./build.sh simulator  # Same as above
./build.sh device     # Build for device (open Xcode to deploy)
```

For device builds with signing:
```bash
TEAM_ID=A1B2C3D4E5 ./build.sh device
```

## Troubleshooting

### "Signing requires a development team"
Device builds need code signing. Either:
- Set `TEAM_ID` environment variable
- Or configure team in Xcode UI (Signing & Capabilities tab)

### Simulator audio not working
iOS Simulator has limited audio capture support. For real AEC testing, use a physical device.

### Linker errors with undefined symbols
Ensure the Rust library was built for the correct target:
- Simulator: `aarch64-apple-ios-sim`
- Device: `aarch64-apple-ios`
