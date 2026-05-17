# FFI Bridge Audit

Updated in 2026-05-15.

This file records what is actually validated for the current FFI/polyglot bridge crates. It is intentionally conservative: a crate compiling is not the same as a production bridge.

Rust dynamic-library ABI details are documented in [RUST_FFI_ABI.md](../technical/RUST_FFI_ABI.md).
Executable smoke evidence is documented in [FFI_NATIVE_SMOKE.md](../technical/FFI_NATIVE_SMOKE.md). The local and CI smoke flow exports `target\ffi\ffi-validation-matrix.json`, a machine-readable matrix with bridge status, evidence, and explicit production blockers.

## Summary Matrix

| Bridge | Crate | Current status | What is validated |
| --- | --- | --- | --- |
| Python | `matter-bridge-python` | Real PyO3 bridge for basic module import, function call, attribute read, and value conversion | `math.sqrt(16)`, `math.pi`, list/dict/int/string conversion |
| Node.js subprocess | `matter-bridge-nodejs` | Prototype bridge using `node -e` subprocess calls | Node availability, simple JS execution, JSON converter tests, and a real built-in `path.basename` call |
| Node.js native | `matter-bridge-nodejs-native` | N-API addon builds and loads in a real Node.js host; exported metadata and typed JSON call are validated through a permanent host example | Rust creation test, `cargo build`, `examples/node_native_host/smoke.js`, real Node `require()` of copied `.node` addon, `matterBridgeInit()`, `matterBridgeVersion()`, and `matterBridgeAddIntsJson([{int 40},{int 2}]) -> {int 42}` |
| Rust | `matter-bridge-rust` | Dynamic-library bridge using `libloading` and a small JSON ABI; crate-name registration remains compile-time only; CLI commands available as `rust-ffi-call-json` and `rust-ffi-validate-args-json`; example plugin in `examples/rust_ffi_plugin` | Cargo availability, crate-name registration, registered-only error path, shared public ABI codec, invalid payload handling, a real temporary `cdylib` call returning int/string/list/error values, CLI arg validation tests, CLI E2E call validation, and example plugin calls |
| Go subprocess | `matter-bridge-go` | Prototype using generated temporary Go wrapper and `go run` | Value conversion, escaped string literals, list/map code generation, JSON conversion, and a real standard-library `math.Sqrt` call |
| Go native | `matter-bridge-go-native` | Default build is fallback; real cgo/libloading implementation is behind `cgo-native` and now has a real shared-library smoke plus permanent example | `cargo test --features cgo-native`, `examples/go_native_plugin/plugin.go` compiled as Go `c-shared` DLL, `libloading`, typed JSON args, `add` and `describe` calls |
| Java subprocess | `matter-bridge-java` | Prototype using generated temporary Java wrapper and standard-library JSON output | Value conversion, escaped string literals, array/map code generation, JSON conversion, classpath tests, and a real `java.lang.String.isEmpty` call |
| Java native | `matter-bridge-java-native` | Default build is fallback; `jni-native` feature compiles, structural tests pass, and an ignored JVM runtime smoke is wired into CI/release with JDK setup | `cargo test --features jni-native` structural tests passed locally; `native-ffi-smoke.ps1 -IncludeJava` runs `java/lang/System.lineSeparator()` on hosts with `java`/`javac` |

## Commands Run

```powershell
cargo test -p matter-bridge-python
cargo test -p matter-bridge-nodejs
cargo test -p matter-bridge-nodejs-native
cargo test -p matter-bridge-rust -p matter-bridge-go -p matter-bridge-java -p matter-bridge-go-native -p matter-bridge-java-native
cargo test -p matter-bridge-nodejs -p matter-bridge-go -p matter-bridge-java
cargo test -p matter-cli rust_ffi_
cargo test -p matter-bridge-go-native --features cgo-native
cargo test -p matter-bridge-java-native --features jni-native
cargo run -q -p matter-cli -- rust-ffi-validate-args-json @examples\rust_ffi_plugin\args_add.json
cargo run -q -p matter-cli -- rust-ffi-call-json <temporary.dll> add_one @args.json
cargo build --manifest-path examples\rust_ffi_plugin\Cargo.toml
cargo run -q -p matter-cli -- rust-ffi-call-json <example_plugin.dll> add @examples\rust_ffi_plugin\args_add.json
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1 -Release -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -IncludeJava
powershell -ExecutionPolicy Bypass -File .\scripts\ffi-smoke-all.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix
```

Observed result:

- Python: 8 tests passed.
- Node.js subprocess: 5 tests passed, including a real `path.basename("a/b.txt")` built-in module call.
- Node.js native: 1 Rust-side creation test passed, and `scripts\native-ffi-smoke.ps1` loaded the built native addon in Node.js v22.22.2, validated `matterBridgeInit` / `matterBridgeVersion`, and performed a typed JSON N-API call returning int 42.
- Go subprocess: 5 tests passed, including escaped string/composite literal generation tests and `math.Sqrt(2.25)`.
- Java subprocess: 6 tests passed, including escaped string/generated HashMap/Object array tests and `java.lang.String.isEmpty()`.
- Rust: 7 bridge tests passed, including ABI codec round-trips for all supported Matter value kinds, formal error decoding, invalid payload rejection, and a temporary Rust `cdylib` loaded through `libloading`. CLI Rust FFI arg/codec tests passed, `rust-ffi-validate-args-json` validated the example args file, `capabilities-json` exposes the Rust FFI commands, and a CLI E2E check loaded a temporary DLL and returned `{"type":"int","value":42}`.
- Example Rust FFI plugin: built successfully and validated `add`, `describe`, `stats`, and formal error return through `matter-cli rust-ffi-call-json`. This path is now covered by `scripts\rust-ffi-plugin-smoke.ps1`, `scripts\validate-full-workspace.ps1`, `scripts\test_all.ps1`, `.github\workflows\ci.yml`, and the Windows release workflow against the compiled release CLI.
- Go native: `cgo-native` feature tests passed, including building a real Go `c-shared` DLL and calling `add` / `describe` through `libloading`.
- Java native: `jni-native` feature now compiles and structural tests pass. Runtime JVM call smoke is implemented as an ignored test and wired into CI/release through `native-ffi-smoke.ps1 -IncludeJava`; local validation entrypoints expose `-IncludeJavaNativeSmoke`. It was not executed locally because this machine does not have `java`/`javac` on PATH.

## Production Claim Rules

Use these labels in public documentation:

- Python: "basic PyO3 bridge validated for built-in modules and simple calls."
- Node.js subprocess: "prototype subprocess bridge with one built-in module call validated."
- Node.js native: "N-API addon loads in a real Node host and validates a typed JSON call; broader module/function conversion tests still needed."
- Rust: "dynamic-library bridge with JSON ABI validated for temporary and example `cdylib` calls; arbitrary crate calls still require explicit exported symbols."
- Go subprocess: "prototype wrapper bridge with one standard-library call validated."
- Go native: "feature-gated cgo bridge validated against a generated Go shared library."
- Java subprocess: "prototype wrapper bridge with one standard-library class method validated."
- Java native: "feature-gated JNI bridge compiles; runtime JVM call smoke exists and must be run on a JDK/JNI host."

Do not describe any bridge as production-ready until it has integration tests that call real external packages/classes/functions in CI.
The validation matrix enforces this policy by keeping `production_claim_allowed` false for every native bridge until that standard changes deliberately.
