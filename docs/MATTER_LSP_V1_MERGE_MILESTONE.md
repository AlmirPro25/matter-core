# Matter LSP v1 Merge Milestone

**Date:** 2026-07-15  
**Status:** **MERGED** (Pre-Merge Gate v1 **PASS**)  
**Remote:** `github-matter-core` (`https://github.com/AlmirPro25/matter-core.git`)  
**Merge type:** `git merge --no-ff`  
**Force push:** not used  
**Release / tag / production_ready:** not changed  

---

## Commits

| Role | SHA | Message |
|------|-----|---------|
| Feature tip (pre-merge) | `edc2cdd` | style(lsp): rustfmt matter-lsp for pre-merge gate |
| Assessment tip | `c2517f9` | docs: LSP recovery v1 assessment… |
| LSP implementation | `ea7f568` | feat(lsp): recovery v1 honesty-aligned… |
| Parent main | `7533efc` | residual close milestone docs |
| **Merge commit** | `580e3c2` / `580e3c2cb97d533e49c638da656a6a2be5781bea` | `merge: recover honest Matter LSP v1` |

**Not merged:** package resolver, lambda/closure integration tests (documented only).

---

## Pre-merge gates

| Gate | Result |
|------|--------|
| Clean worktree / checkout | PASS (`D:\matter-lsp-premerge-gate` @ feature tip) |
| `cargo fmt --check -p matter-lsp` | PASS (after rustfmt commit `edc2cdd`) |
| `cargo check -p matter-lsp` | PASS |
| `cargo test -p matter-lsp --lib` | **15/15 PASS** |
| JSON-RPC smoke (stdio host → `matter_lsp::start_server`) | PASS: initialize, initialized, didOpen diagnostics, shutdown, exit no hang |
| Valid code diagnostics | 0 diags |
| import / export unsupported | honesty messages |
| type annotation unsupported | honesty message |
| panic unsupported | honesty message |
| Hover/completions honesty | no modules-as-working, no typechecker, no full pattern matching, no sandbox claim |
| No shell/network/abs paths in LSP crate | PASS |
| Core / Semantic / Security | **37 / 37 / 26** PASS |
| language-only build | PASS |
| ZIP 0.1.0 | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |

Evidence: `target/validation/lsp_premerge_gate_v1/`

---

## Post-merge recheck (on merge commit)

| Gate | Result |
|------|--------|
| `cargo test -p matter-lsp --lib` | **15/15 PASS** |
| JSON-RPC smoke on merge tree | PASS (exit 0) |

---

## Note on language-only CLI

Default `matter-cli` (`language_main.rs`) does **not** expose `lsp`. LSP is started via `matter_lsp::start_server` (experimental CLI feature `lsp` / `experimental-full`). Gate smoke used a temporary stdio host binary (not shipped).

---

## Intentional non-scope

- No package resolver integration  
- No lambda/closure feature enablement  
- No 0.2.0 release or tag  
- No ZIP rewrite  
