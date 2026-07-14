# File Capabilities v1

**Applies to:** program-initiated `file.*` and `fileio.*`  
**Does not apply to:** CLI host operations such as `compile -o`, reading the source file for `run`/`check`, packaging scripts

## Policy

| Rule | Behavior |
|------|----------|
| Default | **Deny all** program FS access |
| Grant | Explicit roots only via CLI (or equivalent API) |
| Permissions | **Read**, **Write**, **Delete** are distinct |
| Write ⇒ Delete? | **Never** |
| Read ⇒ Write? | **Never** |
| Environment variables | **Must not** grant access |
| Scope | Named directory roots only (not whole computer) |

## CLI flags

```
matter-cli run program.matter --allow-fs-read <dir> --allow-fs-write <dir> --allow-fs-delete <dir>
matter-cli run-json program.matter --allow-fs-read <dir>
matter-cli eval '...' --allow-fs-write <dir>
matter-cli run-bytecode program.mbc --allow-fs-read <dir> ...
```

Same policy object is used for `run`, `eval`, `run-bytecode`, and JSON variants.

## Path containment

1. Roots are canonicalized at grant time (must exist and be directories).  
2. Operation paths are resolved with canonicalize when the path exists; otherwise parent is canonicalized and the final component is joined.  
3. Resolved path must be a component-wise prefix of an authorized root.  
4. `..` traversal, absolute paths outside roots, and symlink/junction resolution that escapes a root are **denied** (`capability_denied`).  
5. If the OS cannot resolve a path safely (broken reparse / unresolvable junction), the access is **blocked** rather than allowed.

### Windows notes

- Verbatim prefixes (`\\?\`) are normalized before prefix comparison.  
- Alternate data streams (`file:stream`) are rejected.  
- Symlinks and junctions: containment uses the **resolved** path; escape ⇒ deny.  
- If a reparse point cannot be resolved confidently, deny and document in diagnostics.

## Error surface

- Runtime / CLI text errors include the stable token **`capability_denied`**.  
- JSON errors set `"error": "capability_denied"` / `"error_code": "capability_denied"` without dumping sensitive host path layouts when possible.

## Method → permission map

| Methods | Permission |
|---------|------------|
| `read`, `exists`, `lines`, `read_lines`, `size`, `is_file`, `is_dir`, `list_dir` | Read |
| `write`, `append`, `write_lines`, `create_dir` | Write |
| `delete`, `remove_dir` | Delete |
| `copy` | Read on source + Write on destination |
| `rename` | Read + Delete on source + Write on destination |

## Central layer

`matter_stdlib::FsCapabilityPolicy` (`crates/matter-stdlib/src/fs_capability.rs`) is the single policy used by both `FileBackend` (`file`) and `FileIOBackend` (`fileio`).

Runtime: `Runtime::with_fs_policy(bytecode, policy, silent)`.

## What this is not

- Not an OS sandbox / AppContainer / seccomp.  
- Not a substitute for running untrusted code in a VM.  
- Does not claim protection against all TOCTOU races or every reparse edge case; unknown/unsafe resolution is denied.

## Tests

See `scripts/test-semantic-honesty-0.2.ps1` and unit tests in `fs_capability.rs` / `file_io.rs`.
