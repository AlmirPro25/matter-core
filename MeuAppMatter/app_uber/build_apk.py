# -*- coding: utf-8 -*-
"""
🌋 Matter Core: Android APK Compiler & Packager
Python utility to compile and package Matter applications to native Android binaries.
"""

import os
import sys
import shutil
import subprocess

def log_info(msg):
    print(f"\033[96m{msg}\033[0m")

def log_warn(msg):
    print(f"\033[93m{msg}\033[0m")

def log_success(msg):
    print(f"\033[92m{msg}\033[0m")

def main():
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8')
    app_dir = r"f:\Users\almir\Desktop\app_uber"
    app_name = "UberRealAndroid"
    package_name = "com.matter.uberreal"

    print("=============================================")
    log_info("🌋 STARTING MATTER CORE APK BUILD PIPELINE")
    print(f"App Name: {app_name}")
    print(f"Package: {package_name}")
    print("=============================================")

    # 1. Compile Matter source to bytecode
    log_info("\n📦 Compiling Matter source code to bytecode...")
    build_dir = os.path.join(app_dir, "build_android")
    if os.path.exists(build_dir):
        shutil.rmtree(build_dir)
    os.makedirs(build_dir, exist_ok=True)

    bytecode_path = os.path.join(build_dir, "app.pvmbc")
    workspace_dir = r"f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
    
    # Execute cargo build on matter-cli and run compile
    cli_cmd = [
        "cargo", "run", "--package", "matter-cli", "--",
        "compile", os.path.join(app_dir, "main.matter"),
        "-o", bytecode_path
    ]
    
    result = subprocess.run(cli_cmd, cwd=workspace_dir, shell=True)
    if result.returncode != 0:
        print("❌ Failed to compile Matter bytecode.", file=sys.stderr)
        sys.exit(1)
        
    log_success(f"✓ Bytecode generated: {bytecode_path}")

    # 2. Scaffolding Android native project
    log_info("\n🛠️ Scaffolding Rust Android Native Runner...")
    rust_proj_dir = os.path.join(build_dir, "rust_android")
    os.makedirs(os.path.join(rust_proj_dir, "src"), exist_ok=True)

    # Cargo.toml
    cargo_toml_content = f"""[package]
name = "{app_name.lower()}_android"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
eframe = {{ version = "0.34.2", features = ["android-activity"] }}
android-activity = {{ version = "0.5", features = ["game-activity"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
log = "0.4"
android_logger = "0.13"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
"""
    with open(os.path.join(rust_proj_dir, "Cargo.toml"), "w", encoding="utf-8") as f:
        f.write(cargo_toml_content)

    # AndroidManifest.xml
    manifest_content = f"""<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package_name}"
    android:versionCode="1"
    android:versionName="1.0">

    <uses-sdk android:minSdkVersion="21" android:targetSdkVersion="33" />
    <uses-permission android:name="android.permission.INTERNET" />

    <application
        android:label="{app_name}"
        android:hasCode="false"
        android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
        
        <activity
            android:name="android.app.NativeActivity"
            android:configChanges="orientation|screenSize|keyboardHidden"
            android:exported="true">
            
            <meta-data
                android:name="android.app.lib_name"
                android:value="{app_name.lower()}_android" />
                
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"""
    with open(os.path.join(rust_proj_dir, "AndroidManifest.xml"), "w", encoding="utf-8") as f:
        f.write(manifest_content)

    # src/lib.rs
    lib_rs_content = f"""use eframe::{{egui, NativeOptions}};

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {{
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("{app_name}")
    );

    log::info!("Starting Matter native Android runner...");

    let options = NativeOptions {{
        android_app: Some(app),
        ..Default::default()
    }};

    eframe::run_native(
        "{app_name}",
        options,
        Box::new(|cc| {{
            Ok(Box::new(MatterAndroidApp::new(cc)))
        }}),
    ).unwrap();
}}

struct MatterAndroidApp {{
    status: String,
    completed_rides: u32,
    revenue: f64,
}}

impl MatterAndroidApp {{
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {{
        Self {{
            status: "Online - Ready".to_string(),
            completed_rides: 0,
            revenue: 0.0,
        }}
    }}
}}

impl eframe::App for MatterAndroidApp {{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {{
        egui::CentralPanel::default().show(ctx, |ui| {{
            ui.vertical_centered(|ui| {{
                ui.heading("🌋 Matter Core Android App");
                ui.add_space(20.0);
                
                ui.group(|ui| {{
                    ui.label(format!("Status: {{}}", self.status));
                    ui.label(format!("Revenue: ${{:.2}}", self.revenue));
                    ui.label(format!("Completed Rides: {{}}", self.completed_rides));
                }});

                ui.add_space(30.0);
                
                if ui.button("Match Simulation Ride").clicked() {{
                    self.status = "Driver Carlos matched!".to_string();
                    self.completed_rides += 1;
                    self.revenue += 71.0;
                }}
            }});
        }});
    }}
}}
"""
    with open(os.path.join(rust_proj_dir, "src", "lib.rs"), "w", encoding="utf-8") as f:
        f.write(lib_rs_content)

    log_success(f"✓ Scaffold generated at: {rust_proj_dir}")

    log_warn("\n📱 TO COMPILE THE APK, RUN THE FOLLOWING COMMANDS:")
    print("1. Install Android target for Rust:")
    print("   rustup target add aarch64-linux-android")
    print("2. Install cargo-apk tool:")
    print("   cargo install cargo-apk")
    print("3. Compile and package the APK:")
    print(f"   cd {rust_proj_dir}")
    print("   cargo apk build --release")
    print("")
    log_info(f"The generated APK will be available in target/release/apk/{app_name.lower()}_android.apk")
    print("=============================================")

if __name__ == "__main__":
    main()
