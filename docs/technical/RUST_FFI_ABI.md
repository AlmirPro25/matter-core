# Rust FFI ABI

Updated in 2026-05-15.

`matter-bridge-rust` can load a Rust dynamic library (`.dll`, `.so`, `.dylib`) with `libloading` and call exported symbols that follow a small JSON ABI.

The bridge crate exposes the shared codec helpers `encode_value_json`, `decode_value_json`, and `decode_args_json`. The CLI uses these same helpers, so command-line calls and direct bridge calls share one ABI implementation.

## Function Shape

Export functions with this C ABI:

```rust
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn function_name(args_json: *const c_char) -> *mut c_char {
    // Return a heap-allocated C string containing one typed Matter value as JSON.
}
```

The bridge passes `args_json` as a JSON array. Each item is a typed Matter value:

```json
[
  { "type": "int", "value": 41 },
  { "type": "string", "value": "matter" }
]
```

The function returns one typed Matter value:

```json
{ "type": "int", "value": 42 }
```

Supported value types are `int`, `float`, `bool`, `string`, `unit`, `list`, `map`, `struct`, and `function`.

To return a bridge error, use:

```json
{ "type": "error", "message": "plugin failed" }
```

The bridge converts this into `Err("plugin failed")`.

## Memory Release

A dynamic library should also export:

```rust
#[no_mangle]
pub extern "C" fn matter_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(std::ffi::CString::from_raw(ptr)); }
    }
}
```

If this symbol exists, the bridge calls it after copying the returned bytes.

## Current Validation

The unit test for `matter-bridge-rust` compiles a temporary Rust `cdylib`, loads it, and validates calls returning:

- `int`: `add_one(41) -> 42`
- `string`: `describe("matter") -> "hello matter"`
- `list`: `pair() -> [1, "two"]`
- `error`: `fail() -> Err("plugin failed")`

The CLI can call the same ABI directly:

```powershell
matter-cli rust-ffi-validate-args-json @args.json
matter-cli rust-ffi-call-json <library_path> <symbol> '[{"type":"int","value":41}]'
matter-cli rust-ffi-call-json <library_path> <symbol> @args.json
```

Use `rust-ffi-validate-args-json` to validate payloads before loading a dynamic library. Use the `@args.json` form on shells that make inline JSON quoting difficult.

See `examples/rust_ffi_plugin/` for a complete compilable plugin with `add`, `describe`, `stats`, and `fail` exports.

This validates the dynamic ABI path. It does not mean arbitrary crates can be called automatically; compatible crates must export symbols with this ABI.
