# Matter Core - Documentation Index

This index points to the organized documentation after the workspace cleanup.

## Root

- [README.md](../README.md) - Main project overview.
- [PROGRESS.md](../PROGRESS.md) - Current development progress.
- [Cargo.toml](../Cargo.toml) - Rust workspace manifest.
- [LICENSE](../LICENSE) - MIT license.

## Core Documentation

- [Manifesto](MANIFESTO.md) - Language philosophy.
- [Specification](SPEC.md) - Language and bytecode specification.
- [Architecture](ARCHITECTURE.md) - Main compiler/runtime architecture.
- [Build Status](BUILD_STATUS.md) - Current validated build notes.
- [Language Tour](LANGUAGE_TOUR.md) - Syntax and feature tour.
- [Reflexive Core](REFLEXIVE_CORE.md) - Reflection and guard workflow.
- [Sentinel Integration](SENTINEL_INTEGRATION.md) - Sentinel bridge notes.
- [Rust FFI ABI](technical/RUST_FFI_ABI.md) - Dynamic Rust bridge ABI for exported symbols.
- [Native FFI Smoke](technical/FFI_NATIVE_SMOKE.md) - Reproducible Rust, Go, Node, and Java native bridge smoke tests.

## Organized Folders

- [docs/status](status/) - Current status, validation results, and reality checks.
- [docs/guides](guides/) - Current quickstart and operational guides.
- [docs/sprints](sprints/) - Sprint history and implementation notes.
- [docs/sessions](sessions/) - Development session summaries.
- [docs/vision](vision/) - Vision, roadmap, and strategy documents.
- [docs/technical](technical/) - Technical deep dives and historical technical reports.
- [docs/archive](archive/) - Older or superseded documents kept for reference, including old guides in `archive/guides-old`.
- [docs/releases](releases/) - Release notes and release-state documents.

## Start Here

1. New to the project: read [README.md](../README.md).
2. Want the current status: read [docs/status/REALIDADE_ATUAL_HONESTA.md](status/REALIDADE_ATUAL_HONESTA.md).
3. Want to compile and run: read [docs/guides/INSTRUCOES_FINAIS.md](guides/INSTRUCOES_FINAIS.md).
4. Want FFI status: read [docs/status/FFI_BRIDGE_AUDIT.md](status/FFI_BRIDGE_AUDIT.md).
5. Want the language shape: read [SPEC.md](SPEC.md) and [LANGUAGE_TOUR.md](LANGUAGE_TOUR.md).
6. Want implementation details: read [ARCHITECTURE.md](ARCHITECTURE.md) and [docs/technical](technical/).

## Current Validated State

The workspace is no longer blocked by the folder name containing spaces. The repo uses `.cargo/config.toml` to place Cargo output outside this directory.

Validated locally:

```powershell
cargo check -p matter-cli
cargo test -p matter-kernel-vm
cargo run -q -p matter-cli -- run examples\first_run.matter
```

## Notes

Some documents in `docs/archive`, `docs/sprints`, and older guides are historical and may contain outdated claims. Prefer `README.md`, `docs/BUILD_STATUS.md`, and `docs/status/` when checking what works today.
