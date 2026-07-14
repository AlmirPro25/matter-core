# Invalid Matter / MBC1 fixtures (Phase 2)

Canonical corpus for language-only hardening tests.

## source/
| File | Intent |
|------|--------|
| illegal_at.matter | Illegal `@` token mid-expression |
| at_token.matter | Illegal `@` after valid statement |
| trailing_garbage.matter | Backtick residual garbage |
| truncated_if.matter | Truncated block |
| truncated_fn.matter | Truncated function header |
| unicode_garbage.matter | Non-ASCII illegal character (U+20AC) |
| deep_nest.matter | Nesting beyond parser recursion limit |

## mbc/
| File | Intent |
|------|--------|
| empty.mbc | Zero-byte file |
| truncated.mbc | Magic only + truncated header |
| bad_magic.mbc | Wrong magic `XXXX` |
| random.bin | Random bytes |
| huge_count.mbc | Valid magic/version but absurd section count |

Evidence copies live under `target/validation/phase_2_core_hardening/corpus_*`.
Suite: `scripts/test-core-suite.ps1` (copies or reads from these fixtures).