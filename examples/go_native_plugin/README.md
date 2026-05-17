# Go Native Plugin Example

This example builds a Go `c-shared` dynamic library that can be loaded by `matter-bridge-go-native` with the `cgo-native` feature.

```powershell
go build -buildmode=c-shared -o F:\Users\almir\Desktop\matter_target\debug\matter_go_native_plugin.dll examples\go_native_plugin\plugin.go
cargo test -p matter-bridge-go-native --features cgo-native
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1
```

Exports:

- `add(args)` accepts typed JSON int args and returns a typed JSON int.
- `describe(args)` returns a typed JSON string.
- `matter_free_string(ptr)` releases Go-allocated result strings.

The bridge keeps loaded Go shared libraries alive for the process lifetime because Go `c-shared` libraries host a Go runtime and should not be unloaded while the host process continues.
