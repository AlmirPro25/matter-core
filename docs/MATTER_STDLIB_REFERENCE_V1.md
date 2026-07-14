# Matter Stdlib Reference V1

**Source of truth:** `crates/matter-stdlib` (+ `matter-energy` for `energy`).  
**Registration:** `register_stdlib_backends` in `crates/matter-runtime/src/lib.rs`.

Call form (backend method): `name.method(args...)` → compiles to `Instruction::BackendCall`.

**Core run path** (`Runtime::new`): `include_extended = true` → includes **extended** backends below.

---

## Always registered (Core + silent)

| Backend | Methods (from `match method`) | Status | Tests |
|---------|-------------------------------|--------|-------|
| `math` | `abs`, `min`, `max`, `pow`, `sqrt`, `mod`, `clamp`, `pi`, `e`, `sin`, `cos`, `tan`, `log`, `ln`, `floor`, `ceil`, `round` | IMPLEMENTED_AND_TESTED (subset) | `test_math_abs`, `min_max`, `pow`, `mod`, `clamp` |
| `string` | `len`, `upper`, `lower`, `trim`, `split`, `join`, `contains`, `replace`, `starts_with`, `ends_with`, `substring`, `repeat`, `index_of`, `char_at`, `format`, `pad_left`, `pad_right` | IMPLEMENTED_AND_TESTED (subset) | `test_string_upper_lower`, `split_join` |
| `list` | `sort`, `reverse`, `sum`, `min`, `max`, `push`, `pop`, `slice`, `range`, `concat`, `contains`, `len`, `get` | IMPLEMENTED_AND_TESTED (subset) | `test_list_sort`, `sum`, `get` |
| `time` | `now`, `sleep` | IMPLEMENTED_AND_TESTED / UNTESTED | `test_time_now`; sleep untested automated |
| `random` | `int` (0/1/2 args), `bool`, `choice` | IMPLEMENTED_AND_TESTED | `test_random_*` |
| `json` | `stringify`, `parse` | IMPLEMENTED_AND_TESTED | `test_json_*` |
| `world` | `configure`, `reset`, `spawn`, `move`, `plan`, `status` | IMPLEMENTED_UNTESTED | no dedicated stdlib unit test listed |
| `audio` | `beep`, `laser`, `jump`, `melody`, `chord`, … | IMPLEMENTED_AND_TESTED (partial) | `test_audio_melody_and_chord` |
| `Vec` | `new`, `with_capacity`, `push`, `pop`, `get`, `set`, `len`, `is_empty`, `clear`, `contains`, `index_of`, `insert`, `remove`, `extend`, `slice`, `reverse`, `sort`, `filter`, `map`, `first`, `last` | IMPLEMENTED_UNTESTED | module `vec.rs` |
| `HashMap` | `new`, `insert`, `get`, `get_or_default`, `contains_key`, `remove`, `keys`, `values`, `len`, `is_empty`, `clear`, `merge`, `from_pairs`, `to_pairs`, `filter`, `map_values` | IMPLEMENTED_UNTESTED | `hashmap.rs` |
| `tensor` | `zeros`, `fill`, `random`, `from_list`, `to_list`, `copy`, `rows`, `cols`, `shape`, `get`, `set`, `row_argmax`, `matmul`, `transpose`, `add`, `sub`, `hadamard`, `scale`, `relu`, `relu_grad`, `softmax_rows`, `axpy`, `sum_rows`, `ce_loss`, `free`, `count`, `seed` | IMPLEMENTED_UNTESTED | `tensor.rs` |
| `result` | `ok`, `err`, `is_ok`, `is_err`, `unwrap`, `unwrap_or`, `try_unwrap`, `map` | IMPLEMENTED_UNTESTED | used by ok/err compile |
| `option` | `some`, `none`, `is_some`, `is_none`, `unwrap`, `unwrap_or` | IMPLEMENTED_UNTESTED | used by some/none compile |

---

## Extended only (`include_extended = true` — normal Core `run`)

| Backend | Methods | FS effects | Status | Tests |
|---------|---------|------------|--------|-------|
| `map` | `new`, `get`, `set`, `remove`, `has`, `keys`, `values`, `size`, `merge` | none | IMPLEMENTED_UNTESTED | — |
| `type` | `of`, `to_int`, `to_float`, `to_string`, `is_int`, `is_float`, `is_string`, `is_list`, `is_map`, `is_bool` | none | IMPLEMENTED_UNTESTED | — |
| `console` | `read`/`read_line`, `write` | stdin/stdout | IMPLEMENTED_UNTESTED | — |
| `file` | `read`, `write`, `exists`, `delete`, `append`, `lines`, `write_lines` | **R/W/D** | IMPLEMENTED_AND_TESTED (partial) | `test_file_lines_and_write_lines` |
| `fileio` | `read`, `write`, `append`, `exists`, `delete`, `read_lines`, `write_lines`, `copy`, `rename`, `size`, `is_file`, `is_dir`, `create_dir`, `list_dir`, `remove_dir` | **R/W/D + dirs** | IMPLEMENTED_UNTESTED | file_io module |

**Note:** There are **two** file-oriented backends (`file` from `lib.rs` and `fileio` from `file_io.rs`) with overlapping method names. Prefer documenting both as registered names.

---

## Non-stdlib but always on Core default backends

From `register_default_backends` (not feature-gated):

| Backend | Crate | Role | Status |
|---------|-------|------|--------|
| `graph` | matter-backend | SVG/chart helpers | IMPLEMENTED_UNTESTED |
| `store` | matter-backend | file-backed key/value | IMPLEMENTED_UNTESTED (**disk write**) |
| `energy` | matter-energy | `profile`, `estimate`, … | IMPLEMENTED_AND_TESTED (energy crate tests) |
| `tool` | matter-backend | tool list/describe/register/call (metadata stub) | IMPLEMENTED_AND_TESTED (VM cost tests) |

Feature-gated (see backend matrix): `agent`, `visual`, `net`, `device`, polyglot, frontier.

---

## Built-in methods **not** going through named stdlib backend

Compiled as VM instructions when target is **not** a backend identifier:

| Syntax | Instruction | Status | Test |
|--------|-------------|--------|------|
| `xs.push(v)` (identifier) | `ListPushVar` | IMPLEMENTED_AND_TESTED | VM list push type errors |
| `xs.pop()` | `ListPopVar` | IMPLEMENTED_AND_TESTED | VM |
| `.len()` / `.has` / `.keys` / `.values` on values | ListLen, MapHas, … | IMPLEMENTED_AND_TESTED | VM |

---

## Core file I/O effects (explicit)

Allowed on language-only **run** (extended stdlib):

- Read: `file.read`, `file.lines`, `fileio.read*`, `fileio.list_dir`, …  
- Write: `file.write`, `file.append`, `file.write_lines`, `fileio.write*`, `fileio.create_dir`, …  
- Delete: `file.delete`, `fileio.delete`, `fileio.remove_dir`, …  

**Not** shell execution. Paths are raw host paths (no sandbox).
