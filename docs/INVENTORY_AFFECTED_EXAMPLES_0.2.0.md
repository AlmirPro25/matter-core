# Inventory: examples/tests affected by Matter 0.2.0 semantic honesty

Generated for branch `develop/0.2.0-semantic-honesty`. Paths relative to repo root.

## import / export (~35)

Examples and demos that contain `import` or `export` statements will fail check/compile/run with an explicit module-not-implemented error.

Scan method: content match for `\bimport\b` / `\bexport\b` under `examples/`.

Known hits include (non-exhaustive at inventory time):

- `examples/chiptune_synth.matter`
- Polyglot / multi-file demos that load modules via `import`
- Package-style samples under `examples/` using export lists

Tests that only **parse** import AST (`tests/integration_test.rs`) remain valid; `build_checked` rejects.

## Type annotations (~20)

Files with `: type` on `let` / parameters / returns:

- `examples/gradual_typing_demo.matter`
- `examples/effect_system_demo.matter`
- `examples/effect_inference_demo.matter`
- Related effect / typing demos under `examples/`

Parser unit test `test_parse_let_with_type` still expects successful **parse**.

## panic

No example is required to use `panic`. Fixture: `tests/fixtures/semantic_0_2/panic_reject.matter`.

## match

Behavior preserved. Fixtures:

- `tests/fixtures/semantic_0_2/match_equality_positive.matter`
- `tests/fixtures/semantic_0_2/match_no_arm.matter`
- `tests/fixtures/semantic_0_2/match_string.matter`

## File I/O

- `examples/chiptune_synth.matter`
- `examples/effect_handlers_demo.matter`
- Fixtures under `tests/fixtures/file_cap_v1/`

## Unchanged Core suite paths

- `examples/hello.matter`
- `examples/fibonacci.matter`
- `examples/events.matter`
- `examples/agent_policy_demo.matter` (map keys only; no import/types/file)
