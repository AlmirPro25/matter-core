# Rust FFI Plugin Example

This example builds a Rust dynamic library that can be called by:

```powershell
cargo build --manifest-path examples\rust_ffi_plugin\Cargo.toml
cargo test --manifest-path examples\rust_ffi_plugin\Cargo.toml
matter-cli rust-ffi-validate-args-json @examples\rust_ffi_plugin\args_add.json
matter-cli rust-ffi-call-json F:\Users\almir\Desktop\matter_target\debug\matter_rust_ffi_plugin.dll add @examples\rust_ffi_plugin\args_add.json
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1
```

This workspace uses `.cargo/config.toml` to place build output in `F:\Users\almir\Desktop\matter_target`. If you build the example outside this workspace, use that project's `target\debug` path instead. On Linux/macOS, replace the library path with the generated `.so` or `.dylib`.

Exports:

- `add(args)` returns an `int`.
- `describe(args)` returns a `string`.
- `stats(args)` returns a `map`.
- `fail(args)` returns a formal ABI error.

The ABI is documented in `docs/technical/RUST_FFI_ABI.md`.

The crate includes unit tests that call the exported C ABI functions directly and verify success plus formal error payloads.
Use `.\scripts\rust-ffi-plugin-smoke.ps1 -CliPath <path-to-matter-cli.exe>` to validate a compiled CLI binary instead of the default `cargo run -p matter-cli` path.
