use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();

    // Only run this logic for Android builds
    if !target.contains("android") {
        return;
    }

    // Explicitly link libc++_shared - this adds DT_NEEDED entry
    println!("cargo:rustc-link-lib=c++_shared");

    // Find the correct NDK library path using the compiler itself
    // Cargokit sets CC environment variable to point to the NDK clang
    if let Ok(cc_path) = env::var("CC") {
        // Ask clang where it keeps c++_shared.so
        let output = Command::new(&cc_path)
            .arg("-print-file-name=libc++_shared.so")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let lib_path = Path::new(path_str.trim());

                // If clang returned a full path, add its parent to search path
                if let Some(parent) = lib_path.parent() {
                    if parent.exists() {
                        println!("cargo:rustc-link-search=native={}", parent.display());
                    }
                }
            }
        }
    } else if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        // Fallback: Use ANDROID_NDK_HOME heuristic
        let host_tag = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "darwin-arm64"
            } else {
                "darwin-x86_64"
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "aarch64") {
                "linux-arm64"
            } else {
                "linux-x86_64"
            }
        } else {
            "linux-x86_64"
        };

        // Modern NDK path (r21+)
        let lib_path = format!(
            "{}/toolchains/llvm/prebuilt/{}/sysroot/usr/lib/{}/",
            ndk_home, host_tag, target
        );
        println!("cargo:rustc-link-search=native={}", lib_path);
    }

    // Rerun if these change
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
}
