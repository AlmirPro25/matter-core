# Native FFI Smoke

This repository has executable smoke tests for the current native FFI paths.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\ffi-smoke-all.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1
```

Both scripts can save a JSON summary:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1 -JsonOut target\ffi\rust-smoke.json
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -JsonOut target\ffi\native-smoke.json
```

CI and release workflows upload these JSON files as artifacts. The Windows release package also includes the generated release summaries under `target\ffi\` and the verifier script under `scripts\`.
The summaries include timestamped evidence, checked symbols/capabilities, host versions, and example paths used by the smoke run.
Use `scripts\verify-ffi-smoke-summaries.ps1` to validate the minimum JSON contract before publishing artifacts. Add `-CheckMatrix` after exporting the matrix to verify that every native bridge keeps an explicit production blocker.
Use `scripts\export-ffi-validation-matrix.ps1` to turn the summaries into `target\ffi\ffi-validation-matrix.json`, including conservative production blockers for each native bridge. The matrix declares `schemas\ffi-validation-matrix.schema.json` as its repo-relative schema reference, and the verifier fails if that schema file is missing.
Use `scripts\export-ffi-validation-report.ps1` to turn the matrix into `target\ffi\ffi-validation-report.md` for a human-readable release artifact.
Use `scripts\export-release-readiness.ps1` to turn the matrix into `target\ffi\release-readiness.json`, which allows an experimental release candidate only when Rust, Node, and Go smoke paths are validated and all production claims remain blocked. The readiness file records the matrix timestamp and required bridge statuses, so the package verifier can reject stale or drifted readiness artifacts.
Use `scripts\export-release-package-manifest.ps1` on a prepared release folder to write `target\ffi\release-package-manifest.json` with relative paths, sizes, and SHA-256 hashes.
Use `scripts\test-ffi-validation-report-contract.ps1` to ensure the report keeps relative example paths and does not allow production claims.
Use `scripts\test-ffi-validation-matrix-contract.ps1` to prove that invalid matrices fail validation, including accidental production-claim enablement or missing blockers.
Use `scripts\test-release-readiness-contract.ps1` to prove that readiness fails on production claim leakage, missing required smoke, missing production blockers, absolute matrix paths, or missing matrix linkage.
For FFI-only validation, `scripts\ffi-smoke-all.ps1` runs both smoke scripts, verifies the generated summaries, and exports the validation matrix. The broader local validation entrypoints `scripts\validate-full-workspace.ps1` and `scripts\test_all.ps1` already generate and verify these summaries unless the FFI smoke steps are skipped.

Use Java only on machines with a working JDK/JNI setup:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -IncludeJava
```

Validated paths:

- Rust FFI: `examples\rust_ffi_plugin\`, JSON ABI, `rust-ffi-call-json`.
- Go native: `examples\go_native_plugin\`, Go `c-shared`, `libloading`, typed JSON args.
- Node native: `examples\node_native_host\`, Node.js host loading the N-API addon and calling typed JSON.
- Java native: `jni-native` compiles; JVM runtime smoke requires JDK/JNI and is wired through `-IncludeJava`.

The Rust ABI details are in `docs\technical\RUST_FFI_ABI.md`.

The Rust smoke also checks `matter-cli capabilities-json` and fails if `rust-ffi-call-json` or `rust-ffi-validate-args-json` disappear from the public command catalog.
The native smoke also checks that the permanent Go and Node examples are present, so release/package regressions are caught before runtime calls start.
The Windows release workflow also runs `scripts\verify-release-package.ps1` before compression and again against the final zip, so missing FFI scripts, examples, schema, summary artifacts, report drift, or manifest hash drift fail the release job.
`scripts\test-release-package-contract.ps1` covers that verifier with positive folder/zip cases and negative missing-artifact, missing-manifest, broken-report, or stale-report cases.
