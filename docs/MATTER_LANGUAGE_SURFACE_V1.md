# Matter Language Surface V1

**Date:** 2026-07-14  
**Scope:** Inventory only — mapped from executable sources (lexer, parser, AST, bytecode builder, VM, runtime registration).  
**Authority:** code + tests win over marketing docs when they disagree.  
**Does not claim:** production_ready, RC, Matter 0.2.0.

**Status key**

| Status | Meaning |
|--------|---------|
| `IMPLEMENTED_AND_TESTED` | Implemented end-to-end and covered by unit/suite tests |
| `IMPLEMENTED_UNTESTED` | Compiles/runs path exists; no dedicated automated test found |
| `PARTIAL` | Parsed and/or partially executed; gaps or no-ops |
| `PARSER_ONLY` | Recognized in lexer/parser/AST but not executed meaningfully |
| `EXPERIMENTAL` | Requires experimental-full / feature flags off Core default |
| `DEPRECATED` | Legacy or superseded path |
| `NOT_IMPLEMENTED` | Named elsewhere but not in Core path |

**Core runtime registration reference:** `matter-runtime::Runtime::new` → `register_default_backends(..., false)` + `register_stdlib_backends(..., true)` (`crates/matter-runtime/src/lib.rs`).

---

## 1. Keywords (lexer)

**Source:** `crates/matter-lexer/src/lib.rs` (`Token` enum + keyword table ~428–459)

| Keyword | Token | Status | Evidence |
|---------|-------|--------|----------|
| `let` | `Let` | IMPLEMENTED_AND_TESTED | parser `test_parse_let_with_type`; suite `hello` |
| `set` | `Set` | IMPLEMENTED_AND_TESTED | suite loops/`set` in core programs; compiler `Statement::Set` |
| `fn` | `Fn` | IMPLEMENTED_AND_TESTED | `test_parse_function_with_types`; VM `test_vm_recursive_fib` |
| `return` | `Return` | IMPLEMENTED_AND_TESTED | fib test; parser function test |
| `if` / `else` | `If`/`Else` | IMPLEMENTED_AND_TESTED | `test_parse_else_if` |
| `while` | `While` | IMPLEMENTED_AND_TESTED | compile emits Jump/JumpIfFalse (`matter-bytecode`) |
| `for` / `in` | `For`/`In` | IMPLEMENTED_AND_TESTED | for-loop compiler `compile_for_loop` |
| `loop` | `Loop` | IMPLEMENTED_AND_TESTED | bytecode `Statement::Loop` |
| `break` / `continue` | `Break`/`Continue` | IMPLEMENTED_AND_TESTED | patch jumps in while/for/loop compile |
| `struct` | `Struct` | IMPLEMENTED_AND_TESTED | AST `StructDef`; VM field tests |
| `print` | `Print` | IMPLEMENTED_AND_TESTED | VM `test_vm_captures_print_output` |
| `on` / `spawn` | `On`/`Spawn` | IMPLEMENTED_AND_TESTED | suite `events`; `SpawnEvent` instruction |
| `match` | `Match` | IMPLEMENTED_AND_TESTED | compile equality arms; limited pattern model |
| `null` | `Null` | IMPLEMENTED_AND_TESTED | AST + constant path |
| `true` / `false` | `Bool` | IMPLEMENTED_AND_TESTED | lexer tests logical; VM bool ops |
| `and` / `or` / `not` | `And`/`Or`/`Not` | IMPLEMENTED_AND_TESTED | `test_word_logical_ops`; `test_parse_word_logical_ops` |
| `ok` / `err` / `some` / `none` | tags | IMPLEMENTED_UNTESTED | compile → `result`/`option` backends |
| `import` / `from` / `as` / `export` | import/export | **PARTIAL** | parsed; **compile is no-op** (`// Imports are resolved...`) |
| `panic` | `Panic` | **PARSER_ONLY** | lexed; **not** handled in `parse_primary`/`parse_statement` → parse error if used as keyword |

**Divergence:** older docs may treat `import` as full module system; **runtime does not load modules from import statements** in the Core bytecode path.

---

## 2. Literals and types

| Literal / type | Layer | Status | Origin / test |
|----------------|-------|--------|---------------|
| Integer `i64` | lexer `Token::Int` | IMPLEMENTED_AND_TESTED | lexer `test_basic_tokens`; VM basic |
| Float `f64` | lexer `Token::Float` | IMPLEMENTED_AND_TESTED | lexer `test_float_literal`; parser `test_parse_float` |
| String (escapes) | lexer | IMPLEMENTED_AND_TESTED | `test_string_escape_sequences` |
| String interpolation `{...}` in string | parser | **PARTIAL** | `parse_interpolated_string` if `{` present |
| Bool | lexer keywords | IMPLEMENTED_AND_TESTED | logical tests |
| `null` | AST `Null` / Constant | IMPLEMENTED_AND_TESTED | serialize constant path |
| Unit (implicit return) | Constant::Unit | IMPLEMENTED_AND_TESTED | function compile injects Unit+Return |
| Gradual type annotations (`: int`, `?`, `!`, lists, maps, unions, fn types) | AST `TypeAnnotation` | **PARTIAL** | parsed (`test_parse_let_with_type`); **not enforced by VM** |
| Runtime values | `matter_backend::Value` | IMPLEMENTED_AND_TESTED | Int/Float/Bool/String/List/Map/Struct/Function/Closure/Null/Unit |

---

## 3. Operators, precedence, associativity

**Source:** `matter-parser` expression stack:  
`parse_logical_or` → `and` → `comparison` → `additive` → `multiplicative` → `unary` → `call/index/field` → `primary`

| Level (high→low binding inside chain) | Operators | Assoc | Status |
|---------------------------------------|-----------|-------|--------|
| Primary / postfix | `()`, `[]`, `.field`, method `.m()` | left | IMPLEMENTED_AND_TESTED |
| Unary | `not`/`!`, unary `-` | right | IMPLEMENTED_AND_TESTED |
| Multiplicative | `* / %` | left | IMPLEMENTED_AND_TESTED (div0 VM test) |
| Additive | `+ -` | left | IMPLEMENTED_AND_TESTED (string concat VM test) |
| Comparison | `== != < > <= >=` | left (chained pairwise) | IMPLEMENTED_AND_TESTED |
| Logical and | `and` / `&&` | left | IMPLEMENTED_AND_TESTED |
| Logical or | `or` / `||` | left | IMPLEMENTED_AND_TESTED |
| Compound assign (statement) | `+= -= *= /=` via `set` | n/a | IMPLEMENTED_UNTESTED (`parse_set` expands to binary) |
| Arrow | `->` | type/fn syntax | PARTIAL (annotations only) |
| `?` try-propagate | postfix | PARTIAL/IMPLEMENTED_UNTESTED | `TryPropagate` → `result.try_unwrap` |

---

## 4. Declarations, assignment, scopes

| Construct | Status | Origin | Test |
|-----------|--------|--------|------|
| `let name = expr` | IMPLEMENTED_AND_TESTED | AST `Let`; StoreGlobal/Local | parser + suite |
| `let name: T = expr` | PARTIAL | annotation stored, not checked at run | `test_parse_let_with_type` |
| `set name = expr` | IMPLEMENTED_AND_TESTED | `StoreExisting` | suite / compiler |
| `set name[i] = v` / field set | IMPLEMENTED_AND_TESTED | `SetIndex`/`SetField` | VM store field/index type-error tests |
| Function/local scopes | IMPLEMENTED_AND_TESTED | PushScope/PopScope; LoadLocal | fib recursion |
| Global vs local | IMPLEMENTED_AND_TESTED | StoreGlobal vs StoreLocal | VM |
| Block scope in if/while/match | IMPLEMENTED_AND_TESTED | PushScope around bodies | compile path |

---

## 5. Functions, parameters, return, recursion

| Feature | Status | Origin | Test |
|---------|--------|--------|------|
| `fn name(params) { ... }` | IMPLEMENTED_AND_TESTED | `FunctionDef` | `test_parse_function_with_types`; suite fib |
| Params + optional types | PARTIAL | types not enforced | parser test |
| Return type `-> T` | PARTIAL | annotation only | parser test |
| Effect list on fn | PARTIAL | AST `effects`; semantic effect_check module | effect tests in bytecode crate |
| `return expr` | IMPLEMENTED_AND_TESTED | `Instruction::Return` | fib |
| Recursion | IMPLEMENTED_AND_TESTED | CallNamed + call stack | `test_vm_recursive_fib_outputs_expected_value` |
| Call arity check | IMPLEMENTED_AND_TESTED | VM setup_function_call | `test_vm_call_arity_error` |
| Lambda `fn (a) { ... }` | IMPLEMENTED_UNTESTED | `MakeClosure` | compile path; limited unit coverage |
| Closures / captures | IMPLEMENTED_AND_TESTED | VM Closure arm | critical-path hardened Phase 2 |

---

## 6. Control flow

| Construct | Status | Notes | Test |
|-----------|--------|-------|------|
| `if` / `else` / `else if` | IMPLEMENTED_AND_TESTED | JumpIfFalse | `test_parse_else_if` |
| `while` | IMPLEMENTED_AND_TESTED | back-edge Jump | compile |
| `for x in iterable` | IMPLEMENTED_AND_TESTED | desugared index loop | `compile_for_loop` |
| `loop { }` | IMPLEMENTED_AND_TESTED | infinite until break | compile |
| `break` / `continue` | IMPLEMENTED_AND_TESTED | patched jumps | compile |
| `match subject { pat { body } }` | **PARTIAL** | arms use **expression equality** only (not full pattern matching) | compile Match |

---

## 7. Lists, maps, structs

| Construct | Status | Origin | Test |
|-----------|--------|--------|------|
| List literal `[a,b]` | IMPLEMENTED_AND_TESTED | `NewList` | VM list len |
| Index `a[i]` | IMPLEMENTED_AND_TESTED | LoadIndex | type-error tests |
| `list.push` / `pop` / `len` methods | IMPLEMENTED_AND_TESTED | ListPush/Pop/Len or *Var | VM list tests |
| Map literal `{ k: v }` | IMPLEMENTED_AND_TESTED | `NewMap` | MapHas type error test |
| Map methods `has`/`keys`/`values` | IMPLEMENTED_AND_TESTED | instructions | VM |
| `struct Name { f: t }` def | IMPLEMENTED_AND_TESTED | StructDef | semantic validate fields |
| Struct literal `Name { f: v }` | IMPLEMENTED_AND_TESTED | NewStruct | field load/store tests |
| Field access `.f` | IMPLEMENTED_AND_TESTED | LoadField | VM |

---

## 8. Events `on` / `spawn`

| Construct | Status | Origin | Test |
|-----------|--------|--------|------|
| `on event { ... }` | IMPLEMENTED_AND_TESTED | EventHandler section | suite `events` |
| `spawn event` | IMPLEMENTED_AND_TESTED | `SpawnEvent` + queue drain | suite / core-status sample |

**Limits:** `MATTER_VM_MAX_EVENT_DRAINS` (Phase 2) — IMPLEMENTED_AND_TESTED via suite fuzz/limits story.

---

## 9–10. Stdlib / backends

See **MATTER_STDLIB_REFERENCE_V1.md** and **MATTER_BACKEND_MATRIX_V1.md**.

**Core `Runtime::new` always registers (non-feature):**  
`graph`, `store`, `energy`, `tool`, plus stdlib set with **extended** file/console/map/type.

**Feature-gated (not in default Core):**  
`agent`, `visual`, `net`, `device`, polyglot (`python`/`node`/`go`/`java`/`rust`), frontier science backends.

---

## 11. Experimental-full only

Requires `matter-cli-experimental` / `experimental-full` or runtime features:

| Area | Status |
|------|--------|
| Polyglot bridges | EXPERIMENTAL |
| `net.*` HTTP client/server | EXPERIMENTAL |
| `agent` backend / agent-ui CLI | EXPERIMENTAL |
| `visual` / device / frontier domains | EXPERIMENTAL |
| PowerShell-era local shell tools | EXPERIMENTAL (allowlisted; not sandbox) |

---

## 12. Parser-recognized but incomplete / weak VM path

| Item | Classification | Detail |
|------|----------------|--------|
| `import` / `export` | PARTIAL | No module loader in bytecode builder |
| Type annotations | PARTIAL | Erased at runtime |
| `panic` keyword | PARSER_ONLY | Lexed, not parsed as expression/statement |
| `match` patterns | PARTIAL | Equality only |
| Interpolated strings | PARTIAL | Special parser path; limited tests |
| `energy profile { }` sugar | IMPLEMENTED_UNTESTED | Desugars to `energy.profile` MethodCall |

---

## 13. Implemented with weak/no dedicated tests

Examples (non-exhaustive): `ok`/`err`/`some`/`none` wrappers; many `string.*` / `list.*` methods beyond sort/sum; `fileio.*` extras; `world.*`; `tensor.*`; lambda free-var capture edge cases; compound `+=` forms.

---

## 14. Legacy / incompatibilities

| Item | Classification | Notes |
|------|----------------|-------|
| Silent skip of illegal chars | DEPRECATED (removed Phase 2) | Now `Token::Illegal` + parse error |
| Full CLI as default binary | DEPRECATED for Core | Replaced by `language_main.rs` |
| Dual `FileBackend` in `lib.rs` vs `file_io::FileIOBackend` as `fileio` | IMPLEMENTED_UNTESTED | Both registered when extended=true; overlapping APIs |

---

## 15. Effects in Core — files

| API | Read | Write | Delete | Status | Registration |
|-----|------|-------|--------|--------|--------------|
| `file.read/write/append/exists/delete/lines/write_lines` | yes | yes | yes | IMPLEMENTED_AND_TESTED (partial: lines/write_lines unit test) | `FileBackend` as `"file"` when extended |
| `fileio.*` (copy, rename, dirs, size, …) | yes | yes | yes | IMPLEMENTED_UNTESTED | `FileIOBackend` as `"fileio"` |
| `console.read` / `write` | stdin | stdout | — | IMPLEMENTED_UNTESTED | extended stdlib |
| `print` statement | — | stdout | — | IMPLEMENTED_AND_TESTED | instruction Print |

**Conclusion:** Core **does allow** filesystem read/write/delete via stdlib backends when using normal `Runtime::new` (language-only CLI run path). This is intentional language I/O, not shell.

`Runtime::new_silent` sets `include_extended=false` → **no** `file`/`fileio`/`console` (used for some internal status paths).

---

## Divergences (docs vs code)

1. Marketing/README historically lists Studio/polyglot/world as “what Matter can do” — **true only for experimental or extended backends**, not bare language-only feature flags.  
2. `import` looks like a module system; **bytecode ignores imports**.  
3. Type annotations look static; **VM does not typecheck**.  

---

## Related

- [MATTER_STDLIB_REFERENCE_V1.md](MATTER_STDLIB_REFERENCE_V1.md)  
- [MATTER_BACKEND_MATRIX_V1.md](MATTER_BACKEND_MATRIX_V1.md)  
- [MATTER_GRAMMAR_REFERENCE_V1.md](MATTER_GRAMMAR_REFERENCE_V1.md)  
- Machine index: `../matter-language-surface-v1.json` (repo root)  
