# Matter Callable and Closure Semantics v1

**Date:** 2026-07-15  
**Branch:** `feature/callable-closure-v1`  
**Base:** `main` @ `b995da5`  
**Status:** implemented on branch (not merged until approval)  
**Not in scope:** package resolver, modules, type system, release/tag  

---

## Semantics (fixed)

| Question | Decision (v1) |
|----------|----------------|
| Can functions be stored in variables? | **Yes** — `let f = fn(...) { ... }` produces a **closure** value (`MakeClosure`). |
| Passed as arguments? | **Yes** — e.g. `apply(double, 5)`. |
| Returned from functions? | **Yes** — factory `make_adder(n)` returns a closure. |
| Capture of free variables? | **Yes** — free names (not lambda params / body locals) are listed on `MakeClosure`. |
| Capture model | **By value** (snapshot of current bindings when the closure is created). Later `set` on outer bindings does **not** update already-created closures. |
| VM representation | `Value::Closure { func_name, captures }` → invoke via `Instruction::Call`. Named `FunctionDef` remains `CallNamed`. |
| Non-callable value | Runtime type error: *value is not callable (expected function or closure)*. |
| Recursion / lifetime | Same call-depth limits as named calls (`CallStackOverflow`). Captures held in the closure value; no GC beyond `Rc` sharing of nested values. |
| Unknown name call | Compile-time: `unknown function or callable 'name'` if not a FunctionDef and not a value binding. |

### Call compilation rule

- `foo(...)` where `foo` is a **FunctionDef** → `CallNamed` (static arity checked at validate).  
- `x(...)` where `x` is a **variable/parameter** → load `x`, then `Call(n)` (arity/type at runtime).  
- Complex callee expressions → evaluate callee, `Call(n)`.

### Lambda body

- Params bound like named functions: `LoadParam` + `StoreLocal`.  
- Body compiled via `compile_function_statement` (same as FunctionDef).

---

## Implementation map

| Area | Change |
|------|--------|
| `matter-bytecode` | Dynamic Call for non-FunctionDef identifiers; validate callables; pre-register FunctionDef names; free-var collection with locals; lambda param prologue |
| `matter-vm` | Closure call frame scope_depth fixed (pop capture block on Return); clearer non-callable error |
| `tests/integration_test.rs` | Four historical lambda/closure tests + non-callable error |

---

## Tests

| Suite | Result |
|-------|--------|
| `matter-bytecode` callable_closure_v1 | PASS |
| Integration: `test_lambda_basic`, `test_lambda_as_argument`, `test_closure_capture`, `test_nested_closure`, `test_triple_nested_closure`, `test_call_non_callable_errors` | PASS |
| Core / Semantic / Security (language-only CLI with new bytecode) | **37 / 37 / 26** |
| `matter-lsp` unit tests | **15/15** (honesty unchanged) |
| ZIP 0.1.0 | intact |

---

## Explicit non-goals

- No import/export / package resolver  
- No mutable capture environments / cells  
- No full first-class method objects  
- No release or tag  

---

## Recommendation

**CALLABLE_CLOSURE_V1_READY** for merge after review (separate approval).
