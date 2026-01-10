# Voice Capture Flutter Example

Cross-platform voice capture example using sys-voice with flutter_rust_bridge.

## Prerequisites

- Flutter SDK 3.10+
- Rust toolchain
- flutter_rust_bridge_codegen: `cargo install flutter_rust_bridge_codegen`

## Setup

1. Generate Rust/Dart bindings:
   ```bash
   flutter_rust_bridge_codegen generate
   ```

2. Get Flutter dependencies:
   ```bash
   flutter pub get
   ```

## Running

### iOS
```bash
flutter run -d ios
```

### Android
```bash
flutter run -d android
```

### macOS
```bash
flutter run -d macos
```

### Windows
```bash
flutter run -d windows
```

### Linux
```bash
flutter run -d linux
```

## Project Structure

```
flutter-example/
├── lib/                    # Dart/Flutter code
│   ├── main.dart          # App entry point
│   └── src/rust/          # Generated Dart bindings (do not edit)
├── rust/                   # Rust bridge code
│   ├── Cargo.toml         # Rust dependencies
│   └── src/
│       ├── lib.rs         # Crate root
│       └── api/           # API exposed to Flutter
├── pubspec.yaml           # Flutter dependencies
└── flutter_rust_bridge.yaml  # FRB configuration
```

## Platform Notes

### iOS
Requires microphone permission in Info.plist:
```xml
<key>NSMicrophoneUsageDescription</key>
<string>Required for voice capture</string>
```

### Android
Requires RECORD_AUDIO permission in AndroidManifest.xml:
```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```
