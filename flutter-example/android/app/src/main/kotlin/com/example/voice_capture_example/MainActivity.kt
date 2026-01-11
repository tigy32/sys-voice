package com.example.voice_capture_example

import android.util.Log
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    companion object {
        init {
            // Load C++ runtime first, then Rust library
            // Both must be loaded via System.loadLibrary for proper symbol resolution
            try {
                System.loadLibrary("c++_shared")
                Log.i("MainActivity", "Loaded libc++_shared.so")
            } catch (e: UnsatisfiedLinkError) {
                Log.e("MainActivity", "Failed to load libc++_shared.so: ${e.message}")
            }
            try {
                System.loadLibrary("rust_lib_voice_capture_example")
                Log.i("MainActivity", "Loaded librust_lib_voice_capture_example.so")
            } catch (e: UnsatisfiedLinkError) {
                Log.e("MainActivity", "Failed to load Rust library: ${e.message}")
            }
        }
    }
}
