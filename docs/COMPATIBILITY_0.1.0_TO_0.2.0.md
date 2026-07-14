# Compatibility report: Matter Core 0.1.0 → 0.2.0 (development)

**From:** Matter Core 0.1.0 baseline (`matter-core-v0.1.0-baseline`)  
**To:** development branch `develop/0.2.0-semantic-honesty`  
**Claim:** development only — **not** production_ready / RELEASE_CANDIDATE  

## Protected artifacts (must remain identical)

| Artifact | Protection |
|----------|------------|
| Git tag `matter-core-v0.1.0-baseline` | Do not move or delete |
| `dist/matter-core-0.1.0-windows-x64.zip` | SHA-256 must remain `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |
| History | No force-push / rewrite |

## Compatible without change

Programs that only use:

- Core control flow (`if` / `while` / `for` / `loop` / `match` equality)
- Functions without type annotations
- `let` / `set` / `print` without annotations
- Events (`on` / `spawn`) without modules
- Stdlib non-FS backends (`math`, `string`, `list`, `json`, …)

Core suite samples used in regression: `examples/hello.matter`, `fibonacci.matter`, `events.matter`.

## Intentional breaking changes

### 1. import / export

| Before | After |
|--------|-------|
| Silent no-op at compile | Hard error: module system not implemented; **no module loaded** |

**Impact:** ~35 example files with import/export (see inventory below). They fail `check`/`compile`/`run` until a future modules milestone.

### 2. Type annotations

| Before | After |
|--------|-------|
| Parsed and ignored | Hard error: type annotations unsupported |

**Impact:** gradual typing / effect demos with `: int`, `-> string`, etc.  
Parser still accepts annotations so the error can be specific; semantics refuse them.

### 3. panic

| Before | After |
|--------|-------|
| Reserved token / generic parse failure | Explicit “reserved, not implemented” diagnostic |

Not advertised as a language feature.

### 4. File I/O

| Before | After |
|--------|-------|
| Program could read/write/delete arbitrarily via `file`/`fileio` when backends present | Default **deny**; must pass `--allow-fs-read|write|delete <dir>` |

**Migration:**

```text
matter-cli run app.matter --allow-fs-read ./data --allow-fs-write ./data
# delete still requires:
matter-cli run app.matter --allow-fs-delete ./data
```

Bytecode cannot bypass the policy (`run-bytecode` uses the same policy).

## Match

**Not a break:** behavior unchanged (equality, first arm, no match ⇒ continue).  
Documentation and tests formalize the contract; no claim of full pattern matching.

## Inventory of affected examples (code scan)

### import / export (representative)

- `examples/chiptune_synth.matter` (import + file I/O)
- Package / multi-file demos under `examples/` that start with `import` / `export`
- Integration AST cases in `tests/integration_test.rs` (parse still OK; compile with `build_checked` fails)

### Type annotations (representative)

- `examples/gradual_typing_demo.matter`
- `examples/effect_system_demo.matter`
- `examples/effect_inference_demo.matter`
- Other demos using `fn f(a: int) -> …`

### File I/O (representative)

- `examples/chiptune_synth.matter` (`file.write_lines`, `file.exists`, `file.lines`)
- `examples/effect_handlers_demo.matter` (`file.write`)
- Any script calling `file.*` / `fileio.*` without new CLI grants

## Suites

| Suite | Expectation on 0.2.0 branch |
|-------|-----------------------------|
| Core 37/37 | Remain green for programs without intentional breaks |
| Security 26/26 | Remain green; FS policy strengthens, does not weaken |
| Portable 20/20 | Remain green; 0.1.0 ZIP hash check must stay PASS |
| `test-semantic-honesty-0.2.ps1` | New permanent suite for honesty + FS caps |

## Status fields

- `development_track`: `0.2.0-semantic-honesty`
- `production_ready`: `false`
- `release_candidate`: `false`
