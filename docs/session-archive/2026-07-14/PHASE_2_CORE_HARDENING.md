# PHASE_2_CORE_HARDENING

**Date:** 2026-07-14  
**Scope:** Hardening of Matter Core **language-only** parser, MBC1 loader, and VM.  
**Not in scope:** polyglot, visual, agents, network, demos, syntax changes for valid programs, silent MBC1 format changes, next phase.  
**Verdict:** Phase 2 **complete** for the approved hardening gates. Runtime is still **not** production-ready overall (deeper audit/fuzz and packaging refresh remain later phases).

---

## 0. Preconditions (before edits)

| Item | Result |
|---|---|
| Phase 1 registered as baseline | **PASS** — `PHASE_1_LANGUAGE_ONLY.md` + evidence under `target/validation/phase_1_language_only/` |
| SHA-256 of `matter-cli.exe` (baseline) | `89A5495FC265A3B82298DE39C4E750EDB3389D32EC1802F1AD9C4A08F08EDDB1` (3 684 828 bytes) |
| SHA-256 of package zip (baseline) | `8D2116A642486130A3C7D79F5CD68631C326FFAB23E92F6CFB96A38BABAC4D82` (`dist/matter-core-windows-x64.zip`) |
| Core suite (no polyglot) | **Created** — `scripts/test-core-suite.ps1` |
| Phase 1 gates still green before hardening | **PASS** — `target/validation/phase_2_core_hardening/baseline_phase1/phase1-gates-recheck.json` |

Baseline artifacts: `target/validation/phase_2_core_hardening/baseline_phase1/`.

---

## 1. Goals achieved

| Goal | Status |
|---|:---:|
| Reject illegal tokens / residual invalid content | **DONE** |
| Full input consumption on parse | **DONE** |
| Structural MBC1 validation before execution | **DONE** |
| No panic on controlled user input (critical paths) | **DONE** (errors as `ParseError` / I/O / `VmError`) |
| Configurable size/depth limits (safe defaults) | **DONE** |
| Better diagnostics without changing valid Matter programs | **DONE** |
| CLI exit codes / stderr / JSON `ok:false` on failure | **DONE** |

---

## 2. Changes by area

### 2.1 Lexer / parser

**Files:** `crates/matter-lexer/src/lib.rs`, `crates/matter-parser/src/lib.rs`

| Change | Detail |
|---|---|
| `Token::Illegal(char)` | Unknown characters are emitted instead of silently skipped |
| Parser rejects `Illegal` | Structured `ParseError` with line/column |
| Full consumption | After statements, only `Eof` may remain |
| Source/token limits | `MATTER_MAX_SOURCE_BYTES` (default 1 MiB), `MATTER_MAX_TOKENS` (default 250 000) via `Parser::from_source_checked` |
| AST recursion cap | `MAX_RECURSION_DEPTH = 64` (safe for host stack; structured error, not OS overflow) |
| Early recursion check | `parse_primary` re-checks depth before nested forms |

**Compatibility:** Valid Matter programs (ASCII identifiers, `#` / `//` comments, existing syntax) are unchanged. `#` remains a **comment starter** (not illegal).

### 2.2 MBC1 loader / validator

**Files:**  
- `crates/matter-bytecode/src/validate.rs` (**new**)  
- `crates/matter-bytecode/src/deserialize.rs`  
- `crates/matter-bytecode/src/lib.rs`

| Change | Detail |
|---|---|
| Bounded decode | Counts/strings/captures/list sizes checked **before** large allocations |
| `Bytecode::validate` / `validate_with_limits` | Magic, section sizes, `LoadConst` indices, jump targets, `MakeClosure` targets, string/collection bounds |
| `load_from_file` | File size check → deserialize → **validate** → return (never executes unvalidated bytecode) |
| `load_from_bytes` | Same pipeline for in-memory buffers |
| Format | **Wire format unchanged** (still MBC1 v0.x sections) |

**Env limits (`BytecodeLimits::from_env`):**

| Variable | Default |
|---|---:|
| `MATTER_MBC_MAX_FILE_BYTES` | 32 MiB |
| `MATTER_MBC_MAX_CONSTANTS` | 100 000 |
| `MATTER_MBC_MAX_FUNCTIONS` | 50 000 |
| `MATTER_MBC_MAX_EVENT_HANDLERS` | 50 000 |
| `MATTER_MBC_MAX_INSTRUCTIONS_PER_BLOCK` | 500 000 |
| `MATTER_MBC_MAX_INSTRUCTIONS_TOTAL` | 2 000 000 |
| `MATTER_MBC_MAX_STRING_BYTES` | 1 MiB |

### 2.3 VM

**File:** `crates/matter-vm/src/lib.rs`

| Change | Detail |
|---|---|
| Extended `VmError` | `StackOverflow`, `CallStackOverflow`, `InstructionLimitExceeded`, `LimitExceeded(String)` |
| `VmLimits` | Configurable stack, call depth, instruction budget, event drains, scope depth |
| `Vm::with_limits` / `set_limits` | Explicit host policy; defaults via env |
| Critical path | Removed user-reachable `unwrap` on closure capture scope; stack/call/instruction guards |
| Existing safety kept | Stack underflow, division by zero, type errors already returned as `VmError` |

**Env limits (`VmLimits::from_env`):**

| Variable | Default |
|---|---:|
| `MATTER_VM_MAX_STACK` | 1 000 000 |
| `MATTER_VM_MAX_CALL_DEPTH` | 10 000 |
| `MATTER_VM_MAX_INSTRUCTIONS` | 100 000 000 |
| `MATTER_VM_MAX_EVENT_DRAINS` | 10 000 |
| `MATTER_VM_MAX_SCOPE_DEPTH` | 50 000 |

**Note:** Unwraps in unit tests and non-user paths were **not** mechanically removed.

### 2.4 CLI (language-only)

**Files:** `crates/matter-cli/src/language_main.rs`, `crates/matter-cli/src/commands/run.rs`

| Rule | Behavior |
|---|---|
| Parse path | Uses `Parser::from_source_checked` then `parse` |
| `check` / `check-json` | Non-zero exit on invalid source; JSON never claims `ok:true` on failure |
| `run` / `run-bytecode` | Non-zero exit on failure; program output on stdout; diagnostics on stderr |
| `run-bytecode` / JSON | Loads via `Bytecode::load_from_file` (validate-before-run); JSON failures include `"ok": false` |
| Disabled experimental cmds | Still exit 2 (Phase 1 contract) |

---

## 3. Tests

### 3.1 Unit tests (core crates)

```
cargo test -p matter-lexer -p matter-parser -p matter-bytecode -p matter-vm \
  --target x86_64-pc-windows-gnu --lib
```

| Crate | Result |
|---|---|
| matter-lexer | 16 passed |
| matter-parser | 13 passed |
| matter-bytecode | 16 passed |
| matter-vm | 35 passed (incl. instruction limit, call depth, underflow → error not panic) |

### 3.2 Corpus layout

| Role | Path |
|---|---|
| **Canonical fixtures** (source of truth) | `tests/fixtures/invalid/` |
| Source invalid | `tests/fixtures/invalid/source/*.matter` |
| MBC invalid | `tests/fixtures/invalid/mbc/*` |
| Fixture index | `tests/fixtures/invalid/README.md` |
| **Evidence copies** (suite sync) | `target/validation/phase_2_core_hardening/corpus_invalid/` |
| | `target/validation/phase_2_core_hardening/corpus_mbc/` |

Suite copies fixtures → evidence on each run (`scripts/test-core-suite.ps1`).

### 3.3 Core-only suite (no polyglot)

```powershell
.\scripts\test-core-suite.ps1
```

Evidence: `target/validation/phase_2_core_hardening/core-suite-results.json`

| Category | Result |
|---|---|
| Valid core programs (`hello`, `fibonacci`, `events`, `agent_policy`) run+check | **PASS** |
| compile + run-bytecode hello | **PASS** |
| Invalid source corpus (7 fixtures) | **PASS** (exit ≠ 0, JSON `ok:false`, stderr diagnostics) |
| Corrupt MBC corpus (5 fixtures) | **PASS** (reject + JSON `ok:false`) |
| PATH = System32 only | **PASS** |
| Deterministic fuzz smoke (32 seeds) | **PASS** (0 crashes) |
| **Total** | **37 / 37 PASS** |

### 3.4 Failure table: before Phase 2 vs after Phase 2

Behavior of **language-only** `matter-cli` on adversarial / invalid inputs.

| Case | Input | **Before Phase 2** | **After Phase 2** |
|---|---|---|---|
| Illegal `@` mid-expression | `let x = 1 @ 2` | Lexer **skipped** `@`; often parsed/ran with wrong AST or cryptic later error | **Reject** parse: `illegal character '@'` (exit 1, stderr) |
| Trailing garbage | `print 1` + `` ` `` junk | Trailing chars often **ignored** after “complete” parse | **Reject** illegal token / residual content |
| Truncated `if` | `if true {` | Parse error (already) | Parse error (structured, exit 1) — **unchanged class**, clearer path via `from_source_checked` |
| Truncated `fn` | `fn f( {` | Parse error (already) | Parse error (exit 1) |
| Non-ASCII garbage | `let x = 1 €` | Char often **skipped** silently | **Reject** `Illegal` |
| Deep nesting (120 parens) | `print ((((…1…))))` | Risk of **OS stack overflow / panic** in debug | **ParseError** recursion depth (cap 64) |
| Oversized source | > `MATTER_MAX_SOURCE_BYTES` | Accepted until OOM/slow | **Reject** at `from_source_checked` |
| Random `.mbc` bytes | `random.bin` | Deserialize fail (partial); possible large alloc on huge counts | **Reject** magic/bounds; no execute |
| Bad magic | `XXXX…` | Reject | Reject (validate path) |
| Truncated MBC header | `MBC1` + 2 bytes | I/O short-read error | Reject; no execute |
| Empty `.mbc` | 0 bytes | I/O error | Reject; no execute |
| Huge section count | valid header + absurd count | **Could allocate huge Vec** before fail | **Reject count** before allocate |
| OOB `LoadConst` in crafted MBC | index ≥ constants | Could panic / type-fail at runtime | **Reject at validate** before `run` |
| OOB jump target | jump past block | Undefined IP behavior / panic risk | **Reject at validate** |
| Stack underflow bytecode | `Add` with empty stack | Already `VmError::StackUnderflow` in many paths | Same structured error + instruction/stack limits |
| Infinite loop bytecode | jump-back loop | Could hang forever | **`InstructionLimitExceeded`** (configurable) |
| Infinite recursion | recursive CallNamed | Stack overflow / hang | **`CallStackOverflow`** |
| Division by zero | `Div` with 0 | Already `VmError::DivisionByZero` | Unchanged structured error |
| Valid core programs | hello/fib/events/… | Run OK | Run OK (**no intentional syntax break**) |
| JSON on failure | `check-json` invalid | Varied | Always `"ok": false` + exit ≠ 0 |
| `run` diagnostics | invalid source | Often stderr | stderr only; stdout not polluted with diagnostics |

Summary: invalid inputs that were **silently accepted or panic-prone** now **fail closed** with structured diagnostics; valid core programs keep working.

### 3.5 Phase 1 gates after Phase 2

Evidence: `target/validation/phase_2_core_hardening/post_phase2/phase1-gates-recheck.json`

| Gate | Result |
|---|:---:|
| help | PASS |
| core-status | PASS |
| run-hello | PASS |
| compile | PASS |
| run-bytecode | PASS |
| agent-ui disabled (exit 2) | PASS |
| polyglot-status disabled (exit 2) | PASS |
| minimal PATH | PASS |
| no `python3.dll` | PASS |
| no OpenGL / MF | PASS |

---

## 4. Unwrap / expect inventory (critical path)

Scope: **language-only** path — lexer → parser → MBC1 load/validate → VM execute → `language_main` / `commands/run`.  
Policy: fix only **user-reachable** panic sites; do **not** mechanically strip test/internal unwraps.

### 4.1 Corrected (Phase 2)

| Location | Before | After |
|---|---|---|
| Lexer unknown char | Silent skip (no token) | `Token::Illegal(ch)` → parser error |
| Parser after program | Residual tokens often ignored | Require `Eof`; reject trailing |
| Parser entry (CLI) | `Parser::from_source` unbounded | `from_source_checked` + size/token limits |
| Parser deep nest | Host stack overflow risk | Depth cap + early `check_recursion` in `parse_primary` |
| VM closure call | `scope_stack.last_mut().unwrap()` | `if let Some(scope)` → `VmError::InvalidInstruction` if missing |
| VM call path | Unbounded recursion/time | `check_call_depth` before mutate; `tick_instruction` budget |
| VM event drain | Hardcoded 10k type-error | `VmLimits::max_event_queue_drains` → `LimitExceeded` |
| VM scope push | Unbounded | `max_scope_depth` → `LimitExceeded` |
| MBC1 load | Deserialize only | Size-bound decode + **full `validate` before return** |
| MBC1 huge counts | `Vec::with_capacity(huge)` risk | `read_bounded_count` rejects before alloc |
| CLI run/check/compile | Mixed parse paths | Unified checked parse; non-zero exit; JSON `ok:false` |

Evidence of no-panic unit tests: `test_instruction_limit_*`, `test_call_depth_limit_*`, `test_stack_underflow_*` in `matter-vm`.

### 4.2 Remaining (intentionally not “fixed” in Phase 2)

| Location | Kind | Why kept |
|---|---|---|
| `matter-vm` `#[cfg(test)]` module | `.unwrap()` on expected-success tests | Test harness only |
| `matter-bytecode` serialize/deserialize/validate tests | `.unwrap()` | Test harness only |
| `matter-parser` unit tests | `.unwrap()` on valid programs | Test harness only |
| `matter-parser` `fn expect(...)` | Method name, not `Result::expect` | Returns `ParseResult` |
| Lexer `num.parse().unwrap_or(0)` / `unwrap_or(0.0)` | Soft fallback on digit run | Digits-only buffer; failure → 0 not panic; not user panic path |
| CLI `serde_json::to_string_pretty(...).unwrap()` in some branches | JSON encode of our own Value | In-process infallible for constructed maps; some paths already use `unwrap_or_else` |
| CLI `unwrap_or_else(\|e\| { eprintln!; exit(1) })` | Controlled process exit | Intentional CLI error handling, not panic |
| VM env feature toggles | `unwrap_or(true)` on env parse | Defaults only; not Matter input |
| `unsafe` instruction slice pointers | Lifetime of owned bytecode | Pre-existing design; validation reduces bad indices; not unwrap |
| `matter-cli` experimental `main.rs` | Out of language-only scope | Phase 1 isolation; not Phase 2 |
| JIT/native helpers (`lookup_native_func` etc.) | Optional fast path | Not pure Matter-source critical path for this phase |

Inventory JSON: `target/validation/phase_2_core_hardening/unwrap-inventory.json`.

---

## 5. Explicit incompatibilities

Only **invalid or adversarial** inputs change behavior. Documented explicitly:

| # | Incompatibility | Who is affected | Mitigation / note |
|---|---|---|---|
| I1 | Programs that previously relied on **silently skipped garbage characters** no longer parse | Broken/accidental sources with `@`, `` ` ``, non-ASCII junk, etc. | **Intentional.** Valid Matter never required those chars. |
| I2 | Trailing junk after a complete program is now an error | Concatenated multi-file paste with garbage | **Intentional** full-consumption rule. |
| I3 | Source larger than `MATTER_MAX_SOURCE_BYTES` (default 1 MiB) rejected | Extremely large sources | Raise env limit; default is safety bound. |
| I4 | Token count > `MATTER_MAX_TOKENS` (default 250 000) rejected | Generated megatokens | Raise env limit. |
| I5 | Expression nesting deeper than **64** recursive parse frames rejected | Pathological nested parens | Raise only by code change (host-stack safety). Normal programs far below 64. |
| I6 | MBC1 files that deserialize but fail structural validation (OOB const/jump, unknown MakeClosure target, oversize sections) no longer load | Crafted/corrupt `.mbc` that old loader might partially accept | **Intentional.** Wire format of **valid** MBC1 unchanged. |
| I7 | VM may stop with `InstructionLimitExceeded` / `CallStackOverflow` on runaway programs | Infinite loops / infinite recursion that previously hung | Defaults high; override via `MATTER_VM_MAX_*`. |
| I8 | UTF-8 **BOM** (`U+FEFF`) at start of source is `Illegal` | Editors that write BOM | Save UTF-8 without BOM (suite/fixtures use no BOM). |

**Not incompatible:**

- Valid Phase 1 core suite programs (hello, fibonacci, events, agent_policy, etc.)
- MBC1 produced by current `compile` (round-trip + validate)
- Comment syntax `#` and `//`
- JSON schema of successful `ok:true` responses for valid runs

---

## 6. Binary size & dependencies

| Metric | Phase 1 baseline | After Phase 2 | Delta |
|---|---:|---:|---:|
| `matter-cli.exe` | 3 684 828 B (~3.51 MB) | 3 725 413 B (~3.55 MB) | **+40 585 B (~1.1%)** |
| SHA-256 (post) | — | `A83233A7D5091CD5283F647A1162213A80ABF2627D516B138DEAD94F948DCAEF` | changed (expected) |
| Package zip | unchanged in this phase | same baseline hash | **not rebuilt** (language-only exe updated under `target/`) |
| DLL surface | kernel/crt/ws2/userenv | unchanged class | no python/opengl/mf |

Post hash file: `target/validation/phase_2_core_hardening/post_phase2/sha256-post.json`  
DLL dump: `target/validation/phase_2_core_hardening/post_phase2/dll-after.txt`

**Dependency regression:** No new crates added to language-only graph for this phase. Size increase is small and consistent with added validation/limit paths.

---

## 7. Reproducible commands

```powershell
cd "D:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
$env:PATH = "D:\mingw64\mingw64\bin;" + $env:PATH
$env:CC = "D:\mingw64\mingw64\bin\gcc.exe"
$env:CXX = "D:\mingw64\mingw64\bin\g++.exe"
$env:DLLTOOL = "D:\mingw64\mingw64\bin\dlltool.exe"
$env:LIBRARY_PATH = "D:\mingw64\mingw64\x86_64-w64-mingw32\lib;D:\mingw64\mingw64\lib"
# RUSTUP_HOME / CARGO_HOME as on this host (e.g. D:\dev-tools\...)

cargo test -p matter-lexer -p matter-parser -p matter-bytecode -p matter-vm `
  --target x86_64-pc-windows-gnu --lib

cargo build -p matter-cli --release --target x86_64-pc-windows-gnu --bin matter-cli
# or: .\scripts\build-matter-cli.ps1 -Release

.\scripts\test-core-suite.ps1
```

---

## 8. Explicit non-goals / restrictions respected

| Restriction | Honored |
|---|:---:|
| No polyglot / visual / agent / network work | yes |
| No mechanical unwrap removal outside critical path | yes |
| No demo/example features | yes |
| No valid-syntax change without incompatibility note | yes (only reject invalid; `#` comments preserved) |
| No silent MBC1 format change | yes |
| Do not start next phase | yes |

---

## 9. Residual risks (for later phases — not started)

- Package zip under `dist/` still contains **Phase 1** binary; re-package when distribution is next in plan.
- Fast-loop / JIT superinstruction paths still use dense stack operations; limits apply at instruction tick boundaries.
- Host OS stack for extremely deep **debug** parser recursion is mitigated (cap 64); release is safer but adversarial corpus expansion still valuable.
- Full production readiness (supply-chain, continuous fuzz, formal threat model) remains **out of Phase 2**.

---

## 10. Deliverables checklist

| Deliverable | Path / status |
|---|---|
| Phase 1 baseline hashes | `target/validation/phase_2_core_hardening/baseline_phase1/` |
| Core-only suite | `scripts/test-core-suite.ps1` |
| Canonical invalid fixtures | `tests/fixtures/invalid/{source,mbc}/` + `README.md` |
| Evidence corpus copies | `target/validation/phase_2_core_hardening/corpus_{invalid,mbc}/` |
| Before/after failure table | §3.4 this document |
| Unwrap inventory | §4 this document + `target/validation/phase_2_core_hardening/unwrap-inventory.json` |
| Explicit incompatibilities | §5 this document |
| Suite results | `target/validation/phase_2_core_hardening/core-suite-results.json` |
| Post hashes + Phase 1 recheck | `target/validation/phase_2_core_hardening/post_phase2/` |
| This report | `PHASE_2_CORE_HARDENING.md` |

**Phase 2 status: COMPLETE.**  
**Stop here.** Do not start Phase 3 (or any next phase) without **new explicit approval**.
