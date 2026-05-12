// Build script for Java native bridge
// Sets up JNI paths for FFI

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try to find JAVA_HOME
    if let Ok(java_home) = env::var("JAVA_HOME") {
        let java_path = PathBuf::from(java_home);

        // Add JNI library paths
        #[cfg(target_os = "windows")]
        {
            let lib_path = java_path.join("lib");
            println!("cargo:rustc-link-search=native={}", lib_path.display());

            let bin_path = java_path.join("bin");
            println!("cargo:rustc-link-search=native={}", bin_path.display());
        }

        #[cfg(target_os = "linux")]
        {
            let lib_path = java_path.join("lib").join("server");
            println!("cargo:rustc-link-search=native={}", lib_path.display());
        }

        #[cfg(target_os = "macos")]
        {
            let lib_path = java_path.join("lib").join("server");
            println!("cargo:rustc-link-search=native={}", lib_path.display());
        }

        println!("cargo:warning=JAVA_HOME found, JNI FFI available");
    } else {
        println!("cargo:warning=JAVA_HOME not set, JNI FFI may not work");
        println!("cargo:warning=Set JAVA_HOME to your JDK installation path");
    }
}
