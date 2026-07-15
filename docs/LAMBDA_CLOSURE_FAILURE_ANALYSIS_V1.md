# Lambda / Closure Failure Analysis v1

**Date:** 2026-07-15  
**Source tests:** `wip/lsp-package-resolver-recovery` → `tests/integration_test.rs`  
**Not modified:** expectations not weakened to force PASS  
**Not merged** into `feature/lsp-recovery-v1`

Related residual surfaces: AST `Expression::Lambda`, bytecode `MakeClosure`, native arms (`991dcf4` / residual) — **present for integrity**, not full language feature.

---

## Cross-cutting observation

All four failures share:

| Stage | Result |
|-------|--------|
| Lexer | Accepts source (no illegal-token failure on samples) |
| Parser | Accepts `fn (params) { body }` as expression form (no parse error) |
| **Compiler / semantic** | **FAIL** — `Compilation error: unknown function '<name>'` when the lambda is **called** as a value |
| VM | Not reached |

So: **syntax is partially recognized; first-class callable values are not wired end-to-end.**  
`MakeClosure` exists as an instruction surface for clean builds/tests, but these programs do not complete a successful compile+run path.

---

## Test 1 — `test_lambda_basic`

| Field | Value |
|-------|--------|
| **Input** | `let add = fn(a, b) { return a + b }` then `print add(3, 4)` |
| **Expected** | Output `["7"]` |
| **Observed** | Compile error: `unknown function 'add'` |
| **Phase** | **Compiler** (after parse) |
| **Syntax** | Current WIP/parser surface (`fn` expression), **not** fully supported as Core 0.2.0 feature |
| **Cause** | Call site treats `add` as named function lookup, not closure value call; lambda binding not registered as callable |
| **MakeClosure** | Instruction may exist; pipeline from `Expression::Lambda` → callable binding incomplete for this program |
| **Recommendation** | **KEEP BLOCKED** as feature work; **do not** change test to expect compile error as “success” without product decision. Fix code only under explicit lambda feature approval. |

---

## Test 2 — `test_lambda_as_argument`

| Field | Value |
|-------|--------|
| **Input** | `fn apply(f, x) { return f(x) }` + `let double = fn(x) { return x * 2 }` + `print apply(double, 5)` |
| **Expected** | `["10"]` |
| **Observed** | Compile error: `unknown function 'f'` |
| **Phase** | **Compiler** |
| **Syntax** | Higher-order call of parameter as function — requires first-class functions |
| **Cause** | Parameter `f` not treated as callable value |
| **MakeClosure** | Same gap as Test 1 |
| **Recommendation** | **KEEP BLOCKED** / **REWORK product**; test is forward-looking, not obsolete for a future feature set. |

---

## Test 3 — `test_closure_capture`

| Field | Value |
|-------|--------|
| **Input** | `let x = 10` + `let add_x = fn(y) { return x + y }` + `print add_x(5)` |
| **Expected** | `["15"]` |
| **Observed** | Compile error: `unknown function 'add_x'` |
| **Phase** | **Compiler** (fails before capture semantics can be observed) |
| **Syntax** | Closure with free variable `x` |
| **Cause** | Same call-site resolution failure; capture untested in practice |
| **MakeClosure** | Capture would require environment in closure data; not validated by this run |
| **Recommendation** | **KEEP BLOCKED**. Do not rewrite test to drop capture solely to pass. |

---

## Test 4 — `test_nested_closure`

| Field | Value |
|-------|--------|
| **Input** | `fn make_adder(n) { return fn(x) { return x + n } }` then `let add5 = make_adder(5)` / `print add5(3)` (and add10 variant in full test) |
| **Expected** | `["8", "17"]` |
| **Observed** | Compile error: `unknown function 'add5'` |
| **Phase** | **Compiler** |
| **Syntax** | Nested lambda return / factory pattern |
| **Cause** | Returned lambda not bound as callable |
| **MakeClosure** | Nested closure support incomplete |
| **Recommendation** | **KEEP BLOCKED** as multi-level first-class function work. |

---

## Relation to residual MakeClosure work

| Item | Status |
|------|--------|
| AST `Lambda` / Result constructors | Present for clean build integrity |
| `Instruction::MakeClosure` match arms | Present (native/bytecode surface tests) |
| End-to-end `let f = fn(...) { ... }; f(x)` | **Not** working — compiler `unknown function` |
| Conclusion | Residual closed **compile surfaces**; **not** a completed lambda product feature |

---

## Recommendation (separate from LSP)

| Choice | Verdict |
|--------|---------|
| Merge these four tests into main as PASS | **No** |
| Change expectations to accept compile error | **No** (hides product gap) |
| Track as **REWORK / feature backlog** | **Yes** |
| Block with LSP recovery | **Yes — keep separate** |

**Classification:** implementation **absent / incomplete** (not obsolete tests for an abandoned syntax that never parsed).
