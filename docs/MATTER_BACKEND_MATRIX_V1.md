# Matter Backend Matrix V1

**Sources:**  
- `crates/matter-runtime/src/lib.rs` — `register_default_backends`, `register_stdlib_backends`, `register_polyglot_backends`  
- `crates/matter-cli` features — Core vs `experimental-full`  
- Capability matrix Phase 3: `docs/status/capability-matrix.json`

## Registration matrix

| Backend name | Registered in Core `Runtime::new` | Feature gate | Dangerous side effects | Status |
|--------------|-----------------------------------|--------------|------------------------|--------|
| `math` | yes (stdlib) | none | none | IMPLEMENTED_AND_TESTED |
| `string` | yes | none | none | IMPLEMENTED_AND_TESTED |
| `list` | yes | none | none | IMPLEMENTED_AND_TESTED |
| `time` | yes | none | sleep blocks thread | IMPLEMENTED_AND_TESTED |
| `random` | yes | none | none | IMPLEMENTED_AND_TESTED |
| `json` | yes | none | none | IMPLEMENTED_AND_TESTED |
| `world` | yes | none | in-memory world model | IMPLEMENTED_UNTESTED |
| `audio` | yes | none | system beep | IMPLEMENTED_AND_TESTED |
| `Vec` | yes | none | none | IMPLEMENTED_UNTESTED |
| `HashMap` | yes | none | none | IMPLEMENTED_UNTESTED |
| `tensor` | yes | none | heap tensors | IMPLEMENTED_UNTESTED |
| `result` | yes | none | none | IMPLEMENTED_UNTESTED |
| `option` | yes | none | none | IMPLEMENTED_UNTESTED |
| `map` | yes if extended | none | none | IMPLEMENTED_UNTESTED |
| `type` | yes if extended | none | none | IMPLEMENTED_UNTESTED |
| `console` | yes if extended | none | stdin/stdout | IMPLEMENTED_UNTESTED |
| `file` | yes if extended | none | **filesystem R/W/D** | IMPLEMENTED_AND_TESTED |
| `fileio` | yes if extended | none | **filesystem R/W/D + dirs** | IMPLEMENTED_UNTESTED |
| `graph` | yes (default) | none | generates strings/files possible | IMPLEMENTED_UNTESTED |
| `store` | yes (default) | none | **persistent file store** | IMPLEMENTED_UNTESTED |
| `energy` | yes (default) | none | estimation only | IMPLEMENTED_AND_TESTED |
| `tool` | yes (default) | none | stub metadata (no OS spawn) | IMPLEMENTED_AND_TESTED |
| `agent` | **no** (cfg agent) | `agent` | prints / agent surface | EXPERIMENTAL |
| `visual` | **no** (cfg visual) | `visual` | UI/process | EXPERIMENTAL |
| `net` | **no** (cfg net) | `net` | **HTTP / sockets** | EXPERIMENTAL |
| `device` | **no** (cfg device) | `device` | camera/audio HW | EXPERIMENTAL |
| `python`/`node`/`go`/`java`/`rust` | **no** | `polyglot` | foreign runtimes, possible package managers | EXPERIMENTAL |
| frontier (`quantum`, `chemistry`, …) | **no** | `frontier` | simulated science | EXPERIMENTAL |

## CLI edition vs backends

| Edition | Binary | Backends available at run |
|---------|--------|---------------------------|
| Language-only Core | `matter-cli` / package `matter.exe` | Table rows without feature gates; **no** agent/net/polyglot/visual/device/frontier |
| Experimental full | `matter-cli-experimental` | All feature-enabled backends + experimental CLI tools |

**Evidence:** Phase 3 security suite — `agent.say` / `net.get` / `python.call` fail with backend not found on Core (`scripts/test-capability-security.ps1`).

## CLI commands (not backends) — Core denylist

From `capability_policy::LANGUAGE_ONLY_DENIED_COMMANDS` / language_main:

`agent-ui`, `shell`, `exec`, `package-install`, `curl`, `polyglot-status-json`, `net-serve`, bridges, etc. → exit 2, NOT executed.

Status: IMPLEMENTED_AND_TESTED (security suite).

## `net` methods (experimental)

`get`, `status`, `ok`, `post`, `serve`, `serve_routes` — `matter-backend` NetBackend.

## `tool` methods (Core stub)

`list`, `describe`, `register`, `call`, `classify`, `invoke_json` — **no process spawn**.

## Divergences

- `store` and `file`/`fileio` give Core **disk write** even though “no shell”.  
- `tool.call` looks agent-like but is a stub map in Core.  
- README historically lists polyglot as “what Matter can do” without always saying **experimental**.  
