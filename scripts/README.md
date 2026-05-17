# Matter Core Scripts

Active automation and validation scripts live in this folder. Older one-off organization scripts are kept in `scripts/archive/`.

## Install Scripts

```powershell
.\scripts\install.ps1
.\scripts\install-local.ps1
.\scripts\uninstall.ps1
.\scripts\uninstall-local.ps1
```

- `install.ps1` installs the Matter CLI globally.
- `install-local.ps1` installs the Matter CLI for the current user.
- `uninstall.ps1` removes the global install.
- `uninstall-local.ps1` removes the local install.

## Validation Scripts

```powershell
.\scripts\preflight-env.ps1
.\scripts\release-doctor.ps1
.\scripts\release-doctor.ps1 -Json
.\scripts\validate-full-workspace.ps1
.\scripts\test-runnable-examples.ps1
.\scripts\test_all.ps1
.\scripts\test_api_bridge.ps1
.\scripts\test_bytecode_equivalence.ps1
.\scripts\test_repl_simple.ps1
.\scripts\test_repl_persistent.ps1
.\scripts\ffi-smoke-all.ps1
.\scripts\ffi-smoke-all.ps1 -IncludeJava
.\scripts\rust-ffi-plugin-smoke.ps1
.\scripts\rust-ffi-plugin-smoke.ps1 -CliPath F:\Users\almir\Desktop\matter_target\debug\matter-cli.exe
.\scripts\rust-ffi-plugin-smoke.ps1 -JsonOut target\ffi\rust-smoke.json
.\scripts\native-ffi-smoke.ps1
.\scripts\native-ffi-smoke.ps1 -IncludeJava
.\scripts\native-ffi-smoke.ps1 -JsonOut target\ffi\native-smoke.json
.\scripts\verify-ffi-smoke-summaries.ps1
.\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix
.\scripts\export-ffi-validation-matrix.ps1
.\scripts\export-ffi-validation-report.ps1
.\scripts\export-release-readiness.ps1
.\scripts\export-release-package-manifest.ps1 -PackageRoot dist\matter-core-windows-x64
.\scripts\test-ffi-validation-matrix-contract.ps1
.\scripts\test-ffi-validation-report-contract.ps1
.\scripts\test-release-readiness-contract.ps1
.\scripts\test-release-package-contract.ps1
.\scripts\verify-release-package.ps1 -PackageRoot dist\matter-core-windows-x64
.\scripts\verify-release-package.ps1 -ZipPath dist\matter-core-windows-x64.zip
```

- `preflight-env.ps1` checks tools, disk space, and LLVM readiness.
- `release-doctor.ps1` provides a release-oriented PASS/WARN/FAIL diagnosis (toolchain/runtime readiness + recommended validation command). Use `-Json` for CI consumption.
- `validate-full-workspace.ps1` runs format, clippy, and workspace tests.
- `test-runnable-examples.ps1` validates the stable runnable example contract with `check-json`, `perf-diagnose-json`, and `run-json`.
- `test_all.ps1` runs the project test script.
- `test_api_bridge.ps1` validates API/CLI JSON behavior.
- `test_bytecode_equivalence.ps1` checks source vs bytecode execution.
- `test_repl_simple.ps1` and `test_repl_persistent.ps1` validate REPL behavior.
- `ffi-smoke-all.ps1` runs the Rust FFI smoke, native FFI smoke, JSON summary verifier, validation matrix/report export, validation checks, and release package contract test as one command. Use `-IncludeJava` only on machines with a working JDK/JNI setup.
- `rust-ffi-plugin-smoke.ps1` builds the Rust FFI example plugin and calls its exported ABI symbols through `matter-cli`. By default it uses `cargo run -p matter-cli`; use `-CliPath` to validate an already built executable and `-JsonOut` to save a machine-readable summary.
- `native-ffi-smoke.ps1` validates Node native through a real Node.js host and Go native through `cgo-native` feature tests with a real generated Go shared library. Use `-IncludeJava` only on machines with `java`, `javac`, and a working JDK/JNI setup; use `-JsonOut` to save a machine-readable summary.
- `verify-ffi-smoke-summaries.ps1` validates the minimum contract of generated FFI smoke JSON summaries. Use `-CheckMatrix` after exporting the validation matrix, and use `-RequireJava` when the native smoke was run with Java enabled.
- `export-ffi-validation-matrix.ps1` writes `target\ffi\ffi-validation-matrix.json`, a machine-readable matrix of validated bridge paths and remaining production blockers. The matrix references `schemas\ffi-validation-matrix.schema.json`.
- `export-ffi-validation-report.ps1` writes `target\ffi\ffi-validation-report.md`, a human-readable report generated from the validation matrix.
- `export-release-readiness.ps1` writes `target\ffi\release-readiness.json`, deciding whether the current matrix is an experimental release candidate while still blocking general production claims. It records matrix timestamp/status linkage for package verification.
- `export-release-package-manifest.ps1` writes `target\ffi\release-package-manifest.json` for a prepared release folder, with relative paths, sizes, and SHA-256 hashes.
- `test-ffi-validation-matrix-contract.ps1` verifies that invalid matrices fail, including accidental production-claim enablement, missing blockers, and missing or invalid schema references.
- `test-ffi-validation-report-contract.ps1` verifies that the human-readable report contains the expected bridge rows, relative example paths, and no production claim allowance.
- `test-release-readiness-contract.ps1` verifies that readiness allows only an experimental release candidate and fails on production claims, missing required smoke, missing blockers, absolute matrix paths, or missing matrix linkage.
- `test-release-package-contract.ps1` verifies that invalid release package folders fail, and that valid package folders and final zips pass the release package verifier, including manifest and report drift checks.
- `verify-release-package.ps1` checks that a prepared Windows release folder or final zip contains the expected binary, docs, FFI examples, scripts, schema, generated FFI JSON summaries, report, and release manifest.

`validate-full-workspace.ps1` and `test_all.ps1` generate FFI smoke summaries under `target\ffi\` and verify them automatically unless the related smoke steps are skipped.

Useful validation options:

```powershell
.\scripts\validate-full-workspace.ps1 -JsonSummary
.\scripts\validate-full-workspace.ps1 -RunPreflight
.\scripts\validate-full-workspace.ps1 -RunDoctor
.\scripts\validate-full-workspace.ps1 -RunDoctor -RequireDoctorPass
.\scripts\validate-full-workspace.ps1 -CiMode
.\scripts\validate-full-workspace.ps1 -RequireLLVM
.\scripts\validate-full-workspace.ps1 -SkipRunnableExamples
.\scripts\validate-full-workspace.ps1 -SkipRustFfiSmoke
.\scripts\validate-full-workspace.ps1 -SkipNativeFfiSmoke
.\scripts\validate-full-workspace.ps1 -IncludeJavaNativeSmoke
.\scripts\validate-full-workspace.ps1 -IncludeNodeNativeUnitTests
.\scripts\test_all.ps1 -SkipRustFfiSmoke
.\scripts\test_all.ps1 -SkipNativeFfiSmoke
.\scripts\test_all.ps1 -SkipRunnableExamples
.\scripts\test_all.ps1 -IncludeJavaNativeSmoke
.\scripts\test_all.ps1 -IncludeNodeNativeUnitTests
.\scripts\test_all.ps1 -CiMode
.\scripts\preflight-env.ps1 -MinFreeGB 20
```

## Archived Scripts

`scripts/archive/` contains older workspace organization and move scripts. They are preserved for history and should not be used for normal development.

## PowerShell Policy

If PowerShell blocks script execution:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```
