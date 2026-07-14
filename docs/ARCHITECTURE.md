# Architecture — Matter Core

## Overview

Matter is organized as a Cargo workspace. The **Core** path is a thin vertical slice:

```
Source (.matter)
    │
    ▼
matter-lexer ──► matter-parser ──► matter-ast
    │
    ▼
matter-bytecode (builder + MBC1 serialize/deserialize + validate)
    │
    ▼
matter-vm  ◄── matter-runtime (backends registration)
    │
    ▼
matter-cli (language_main.rs)  /  package matter.exe
```

## Core crates

| Crate | Responsibility |
|-------|----------------|
| `matter-lexer` | Tokenization |
| `matter-parser` | Parsing, limits |
| `matter-ast` | AST types |
| `matter-bytecode` | MBC1 IR, codec, structural validate |
| `matter-vm` | Execution, limits, errors |
| `matter-backend` | Values, backend trait, optional net/agent types |
| `matter-stdlib` | Built-in backends (math, string, file, …) |
| `matter-runtime` | Wire VM + backends; feature-gated polyglot/visual |
| `matter-cli` | CLI binaries: language-only + experimental |

## Feature flags (runtime / CLI)

- **Default Core:** no `polyglot`, `visual`, `device`, `agent`, `net`  
- **`experimental-full`:** full previous surface for `matter-cli-experimental`  

## MBC1

- Magic `MBC1`, versioned sections (constants, functions, events, main)  
- Load path: size bounds → deserialize → **validate** → execute  
- Invalid / truncated / random files must fail before run  

## Security boundaries

| Layer | Policy |
|-------|--------|
| Core CLI commands | Denylist for agent/shell/net/package names |
| Core backends | Agent/net/python not registered without features |
| Experimental local process | Allowlist + injection reject + timeout; **not a sandbox** |

## Package layout (portable)

```
bin/matter-cli.exe
bin/matter.exe
examples/
schemas/
scripts/   # install, verify, update, uninstall
MANIFEST.json
SHA256SUMS
```

No `target/`, no toolchains on the destination machine.

## Related docs

- [INSTALL_WINDOWS.md](INSTALL_WINDOWS.md)  
- [RELEASE_PROCESS.md](RELEASE_PROCESS.md)  
- Phase writeups under [status/](status/)  
