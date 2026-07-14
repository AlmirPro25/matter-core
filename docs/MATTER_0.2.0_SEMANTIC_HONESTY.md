# Matter 0.2.0 — Semantic Honesty

**Branch:** `develop/0.2.0-semantic-honesty`  
**Status:** development 0.2.0 (not `production_ready`, not `RELEASE_CANDIDATE`)  
**Baseline protected:** tag `matter-core-v0.1.0-baseline`, ZIP `dist/matter-core-0.1.0-windows-x64.zip` (hash unchanged)

## Goals

Stop pretending unimplemented surfaces work. Core 0.2.0 rejects silent no-ops and unenforced type annotations, formalizes existing `match` equality semantics, and introduces **File Capabilities v1** (see `docs/FILE_CAPABILITIES_V1.md`).

## Intentional semantic changes

| Surface | 0.1.0 behavior | 0.2.0 behavior |
|--------|----------------|----------------|
| `import` / `export` | Compiled as silent no-op (no module loaded) | **Error** on check/compile/run: not implemented; no module loaded |
| Type annotations (`let x: int`, `fn f(a: int) -> int`) | Parsed, ignored (false sense of type safety) | **Error**: `type annotations unsupported` (no typechecker) |
| `panic` | Reserved token; failed with generic parse noise | **Error**: reserved word, construct not implemented (not a language feature of this version) |
| `match` | Equality comparison, first arm wins | **Unchanged runtime**; documented + permanent tests |
| `file.*` / `fileio.*` | Open FS access when backends registered | **Default deny**; explicit `--allow-fs-read|write|delete <dir>` |

## Non-goals (this phase)

- No module system implementation  
- No partial or full typechecker  
- No new panic runtime mechanism  
- No destructuring / guards / full pattern matching  
- No polyglot / agent / net / visual / experimental-full feature work  
- No promotion to production_ready or RELEASE_CANDIDATE  
- No rewrite of git history, force-push, or mutation of 0.1.0 tag/ZIP  

## import / export

```
import "mod.matter"   → error: import is not implemented … no module was loaded
export foo            → error: export is not implemented … no symbols were exported
```

Enforced in `BytecodeBuilder::build_checked` → `validate_statement`.  
CLI `check`, `compile`, and `run` all use `build_checked`.

## Type annotations

Annotations remain **parseable** (grammar surface) so diagnostics can name the feature honestly.  
Semantic check rejects any non-empty annotation with a stable message:

`type annotations unsupported in Matter Core 0.2.0`

No partial checking that could look complete.

## panic

Lexer still reserves `panic`. Parser returns a dedicated not-implemented diagnostic.  
Do **not** document panic as an available language feature of Core 0.2.0.

## match (equality)

Syntax: `match subject { pattern => { body } ... }`

Existing compilation:

1. Evaluate subject once into a temporary.  
2. For each arm in source order: evaluate pattern, `Eq` with subject, run body on first true.  
3. Jump to end after a taken arm (no fall-through).  
4. If no arm matches, no body runs; execution continues after `match`.

**Not** supported / not advertised: destructuring, guards, exhaustiveness, `else` sugar as a special form (use a final equality arm if desired).

## Compatibility summary

Programs without import/export, type annotations, or program file I/O remain compatible.  
Programs that used silent import/export or unchecked annotations break intentionally (documented).  
Programs that used file I/O must pass explicit FS roots.

See `docs/COMPATIBILITY_0.1.0_TO_0.2.0.md`.

## Tests & evidence

- Unit: `matter-bytecode` `semantic_honesty_0_2_tests`, `matter-parser` panic tests, `matter-stdlib` `fs_capability`  
- Fixtures: `tests/fixtures/semantic_0_2/`, `tests/fixtures/file_cap_v1/`  
- Suite: `scripts/test-semantic-honesty-0.2.ps1`  
- Evidence: `target/validation/matter_0_2_semantic_honesty/`  

## Version / status

- Development track: **0.2.0 semantic honesty**  
- `production_ready`: **false**  
- `RELEASE_CANDIDATE`: **false**  
- Frozen package 0.1.0 artifacts: **untouched**
