# Status reports (Phases 1–6)

| Document | Topic |
|----------|--------|
| [PHASE_1_LANGUAGE_ONLY.md](PHASE_1_LANGUAGE_ONLY.md) | Binary isolation language-only |
| [PHASE_2_CORE_HARDENING.md](PHASE_2_CORE_HARDENING.md) | Parser / MBC1 / VM hardening |
| [PHASE_3_CAPABILITY_SECURITY.md](PHASE_3_CAPABILITY_SECURITY.md) | Capability isolation |
| [PHASE_4_PORTABLE_RELEASE.md](PHASE_4_PORTABLE_RELEASE.md) | Portable package |
| [PRODUCTION_READINESS_AUDIT_V1.md](PRODUCTION_READINESS_AUDIT_V1.md) | Audit V1 |
| [PRODUCTION_READINESS_AUDIT_V2.md](PRODUCTION_READINESS_AUDIT_V2.md) | Audit V2 |
| [EXTERNAL_WINDOWS_VALIDATION_V1.md](EXTERNAL_WINDOWS_VALIDATION_V1.md) | External Windows (BLOCKED) |
| [production-readiness-v2.json](production-readiness-v2.json) | Machine-readable gate result |
| [capability-matrix.json](capability-matrix.json) | Capability matrix |

Small evidence JSON: `../evidence/releases/0.1.0/`.

## Language surface inventory (code-backed)

| Document | Topic |
|----------|--------|
| [MATTER_LANGUAGE_SURFACE_V1.md](../MATTER_LANGUAGE_SURFACE_V1.md) | Keywords, types, operators, control flow, gaps |
| [MATTER_STDLIB_REFERENCE_V1.md](../MATTER_STDLIB_REFERENCE_V1.md) | Stdlib backends and methods |
| [MATTER_BACKEND_MATRIX_V1.md](../MATTER_BACKEND_MATRIX_V1.md) | Core vs experimental backends |
| [MATTER_GRAMMAR_REFERENCE_V1.md](../MATTER_GRAMMAR_REFERENCE_V1.md) | Descriptive grammar / precedence |
| [matter-language-surface-v1.json](../../matter-language-surface-v1.json) | Machine-readable index |

These documents map **what code actually implements** vs what only parses. They do not change the 0.1.0 frozen package.
