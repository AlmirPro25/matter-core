// Build script for Go native bridge
// Compiles Go packages as shared libraries for FFI

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Check if Go is available
    if let Ok(output) = Command::new("go").arg("version").output() {
        if output.status.success() {
            println!("cargo:warning=Go compiler found, cgo FFI available");
        }
    } else {
        println!("cargo:warning=Go compiler not found, cgo FFI will not be available");
        println!("cargo:warning=Install Go from https://go.dev/dl/");
    }

    // Set library search path
    if let Ok(go_path) = env::var("GOPATH") {
        let lib_path = PathBuf::from(go_path).join("lib");
        println!("cargo:rustc-link-search=native={}", lib_path.display());
    }
}
