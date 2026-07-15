# Package Resolver Assessment v1

**Date:** 2026-07-15  
**Base:** `7533efc`  
**Source:** `wip/lsp-package-resolver-recovery` @ `30cf5d7`  
**Integrated into `feature/lsp-recovery-v1`:** **no**

---

## Summary classification

### **FUTURE_MODULE_SYSTEM** (primary)

with notes of **REQUIRES_REDESIGN** before any honest integration into Core 0.2.0.

Not **STANDALONE_USEFUL** for current language-only CLI paths.  
Not pure **DEAD_CODE** in the abstract (workspace member exists; WIP expands it), but **dead on all executable Core paths today**.

---

## Diagnostic answers

| Question | Answer |
|----------|--------|
| Used by any current executable path? | **No.** Grep of workspace shows only `Cargo.toml` workspace membership (`crates/matter-package-resolver`). No CLI, LSP, runtime, or VM dependency. |
| Depends on import/export? | **Yes.** WIP redesign is an **import path resolver** that reads/inlines imported sources. Core 0.2.0 hard-errors import/export as not implemented. |
| Filesystem / network / shell / registry? | WIP uses **`std::fs::read_to_string`** and path joining. **No** HTTP/reqwest, **no** `std::process`, **no** Windows registry APIs in the WIP module. |
| Local paths only or external packages? | Local/project-root + declared dependency map + `std:` prefix helper in tests. **Not** a remote package registry client. |
| Future-usable API? | Concepts yes: `ResolverConfig`, `ImportResolver`, circular detection, depth limit, cache. API is import-centric → only valuable **after** a real module system decision. |
| Do the four unit tests reflect real behavior? | They test **parser helpers and path resolution** in isolation (`parse_import_lines`, alias, `std` prefix, relative-to-file). They do **not** prove integration with VM/CLI or Semantic Honesty. |
| Integrating now violate Semantic Honesty? | **Yes.** Shipping a resolver that loads/inlines modules while Core diagnostics say import is not implemented would **contradict** 0.2.0 honesty unless import remains a hard error and the crate stays unused (pointless) or import is re-enabled (out of scope / feature). |

---

## WIP surface (from stash recovery branch)

- ~+336 lines over stub `add()` crate  
- Types: `ResolverConfig`, `ResolvedImport`, `ResolveError`, `ImportResolver`  
- FS reads of `.matter` files under project root  
- Unit tests: **4 PASS** when built on WIP branch  

---

## Recommendation (separate from LSP)

| Option | Verdict |
|--------|---------|
| Merge with LSP recovery | **No** |
| Keep on WIP branch for future modules | **Yes** |
| Classification | **FUTURE_MODULE_SYSTEM** + **REQUIRES_REDESIGN** (honesty + capability policy for FS) |
| Next time to reopen | Only with an explicit modules design that supersedes “import not implemented” |

Do **not** enable import/export to “make the resolver useful.”
