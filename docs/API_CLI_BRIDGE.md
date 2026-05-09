# Matter Core API/CLI Bridge

Matter Core can be used as an execution target for APIs, agents, and cloud workers.
The important idea is simple:

```text
human intent -> AI/API generates Matter source -> matter-cli validates/runs/compiles it
```

## Package Manifest

A Matter project can declare its package metadata in `matter.toml`:

```toml
[package]
name = "matter-core"
version = "0.1.0"
entry = "examples/showcase.matter"

[paths]
stdlib = "stdlib"
store = ".matter_store.json"

[dependencies]
math_tools = "examples/modules/math_tools.matter"
```

APIs and cloud CLIs can inspect it without running code:

```bash
.\target\release\matter-cli.exe package-json
```

This returns package identity, entrypoint, runtime paths, and local dependency aliases as JSON.

Project commands can import dependency aliases declared in the manifest:

```matter
import "math_tools"

print dobro(21)
```

They can also execute the whole project contract directly from the manifest:

```bash
.\target\release\matter-cli.exe project-check-json
.\target\release\matter-cli.exe project-run-json
.\target\release\matter-cli.exe project-imports-json
.\target\release\matter-cli.exe project-lock-json
.\target\release\matter-cli.exe project-source-json
.\target\release\matter-cli.exe project-compile-json -o target\project.mbc
```

Project commands read `package.entry`, resolve `paths.stdlib` through `MATTER_STDLIB_PATH`, and route `paths.store` through `MATTER_STORE_PATH` for the current run. That lets a cloud worker mount a repository and call one stable command without knowing the internal file layout.

`project-lock-json` returns the same project graph plus file sizes and deterministic fingerprints for the manifest, entrypoint, dependency aliases, and transitive imports. It is meant for API caching, cloud execution plans, and reproducibility checks.

`project-source-json` returns the resolved Matter source after expanding local imports, dependency aliases, and standard-library imports. This gives APIs and agents one canonical source string to review, cache, compile, or ship to an isolated worker.

## Capability discovery

APIs can discover the available contract at runtime:

```bash
.\target\release\matter-cli.exe capabilities-json
```

Response:

```json
{"ok":true,"name":"matter-cli","version":"0.1.0","bytecode":"MBC1","stdin":true,"json_commands":["capabilities-json","package-json","project-check-json","project-run-json","project-imports-json","project-lock-json","project-source-json","project-compile-json","eval-json","tokens-json","imports-json","check-json","run-json","emit-json","compile-json","inspect-json","run-bytecode-json","emit-bytecode-json"],"source_commands":["run","eval","emit","check","compile"],"bytecode_commands":["run-bytecode","emit-bytecode","inspect"],"language_features":["variables","functions","recursion","if","while","loop","for","break","continue","events","lists","maps","structs","backend_calls","imports","stdlib","persistence","network","concurrency","packages"]}
```

## Local imports

Matter files can import other local Matter files:

```matter
import "modules/math_tools.matter"

print dobro(21)
```

Import paths are resolved relative to the file that declares the import. Imports are expanded before parsing and circular imports are rejected.

Matter also has a first standard-library namespace. Imports that start with `std/` are resolved from the project `stdlib` directory, or from `MATTER_STDLIB_PATH` when the CLI is running in another environment:

```matter
import "std/math.matter"

print square(6)
print clamp(99, 0, 10)
```

## Persistence

Matter programs can persist values through the built-in `store` backend. By default it writes `.matter_store.json` in the current directory. Set `MATTER_STORE_PATH` to choose another file, which is usually better for API workers and cloud CLIs.

```matter
store.set("counter", 41)
let counter = store.get("counter")
store.set("counter", counter + 1)

print store.get("counter")
print store.has("counter")
print store.delete("counter")
```

Supported methods: `store.set(key, value)`, `store.get(key)`, `store.has(key)`, `store.delete(key)`, `store.clear()`, and `store.list()`.

## Network

Matter programs can make HTTP calls through the built-in `net` backend. Set `MATTER_NET_TIMEOUT_MS` to configure the request timeout. The current portable backend supports `http://` URLs.

```matter
print net.status("http://example.com")
print net.ok("http://example.com")
let body = net.get("http://example.com")
```

Supported methods: `net.get(url)`, `net.status(url)`, `net.ok(url)`, and `net.post(url, body)`.

## Cooperative Concurrency

Matter can enqueue event work with `spawn`. The current model is cooperative: the VM finishes the current instruction stream, then drains the queued events in order. Events can enqueue more events.

```matter
on boot {
    print "boot"
    spawn tick
}

on tick {
    print "tick"
}

spawn boot
print "main"
```

This prints `main`, then `boot`, then `tick`.

Use `imports-json` to inspect an import graph without running the program:

```bash
.\target\release\matter-cli.exe imports-json examples\test_imports.matter
```

Response:

```json
{"ok":true,"input":"examples\\test_imports.matter","count":1,"imports":[{"from":"examples\\test_imports.matter","path":"modules/math_tools.matter","resolved":"...\\examples\\modules\\math_tools.matter"}]}
```

## Direct source execution

Use `eval` when the generated Matter source is short enough to pass as one command argument:

```bash
.\target\release\matter-cli.exe eval "print 41 + 1"
```

This is useful for quick agent actions, smoke tests, and small generated programs.

Use `eval-json` for the same direct-snippet workflow with structured output:

```bash
.\target\release\matter-cli.exe eval-json "print 41 + 1"
```

Response:

```json
{"ok":true,"input":"<eval>","output":["42"]}
```

## Token inspection

Use `tokens-json` to inspect how Matter source is tokenized:

```bash
"let x = 42" | .\target\release\matter-cli.exe tokens-json -
```

Response:

```json
{"ok":true,"input":"<stdin>","tokens":[{"index":0,"kind":"let","line":1,"column":1},{"index":1,"kind":"ident","line":1,"column":5,"value":"x"},{"index":2,"kind":"eq","line":1,"column":7},{"index":3,"kind":"int","line":1,"column":9,"value":"42"},{"index":4,"kind":"eof","line":1,"column":11}]}
```

## Stdin execution

Use `-` as the input path when an API, script, or cloud process sends source through stdin:

```bash
"print 7 * 6" | .\target\release\matter-cli.exe run -
```

For APIs that need structured execution output, use `run-json`:

```bash
"print 1 print 2" | .\target\release\matter-cli.exe run-json -
```

Response:

```json
{"ok":true,"input":"<stdin>","output":["1","2"]}
```

Runtime errors also preserve output produced before the failure:

```json
{"ok":false,"stage":"runtime","input":"<stdin>","error":{"message":"division by zero"},"output":["1"]}
```

In JSON execution modes, the default `agent` and `visual` backends are silent so their human-readable logs do not contaminate the JSON response.

## Event execution

Events can also be emitted with structured output:

```bash
.\target\release\matter-cli.exe emit-json examples\events.matter boot
```

Response:

```json
{"ok":true,"input":"examples\\events.matter","event":"boot","output":["Sistema online"]}
```

Missing events are treated as a successful no-op:

```json
{"ok":true,"input":"examples\\events.matter","event":"missing_event","output":[]}
```

The same pattern works for validation:

```bash
"print 10" | .\target\release\matter-cli.exe check -
```

For APIs and agents, prefer JSON validation:

```bash
"print 10" | .\target\release\matter-cli.exe check-json -
```

Successful response:

```json
{"ok":true,"input":"<stdin>","summary":{"constants":1,"functions":0,"event_handlers":0,"instructions":3}}
```

Parse error response:

```json
{"ok":false,"stage":"parse","input":"<stdin>","error":{"message":"line 1, column 5: Expected identifier","line":1,"column":5}}
```

Semantic error response:

```json
{"ok":false,"stage":"semantic","input":"<stdin>","error":{"message":"undefined variable 'missing'"}}
```

And for bytecode compilation:

```bash
"print 10" | .\target\release\matter-cli.exe compile - -o programa.mbc
.\target\release\matter-cli.exe run-bytecode programa.mbc
```

For structured compilation, use `compile-json`:

```bash
"print 10" | .\target\release\matter-cli.exe compile-json - -o programa.mbc
```

Response:

```json
{"ok":true,"input":"<stdin>","output":"programa.mbc","summary":{"constants":1,"functions":0,"event_handlers":0,"instructions":3}}
```

Compiled bytecode can be inspected as JSON:

```bash
.\target\release\matter-cli.exe inspect-json programa.mbc
```

Response:

```json
{"ok":true,"input":"programa.mbc","magic":"MBC1","summary":{"constants":1,"functions":0,"event_handlers":0,"instructions":3},"functions":[],"event_handlers":[],"constants":[{"index":0,"type":"int","value":10}]}
```

Compiled bytecode can also be executed with structured output:

```bash
.\target\release\matter-cli.exe run-bytecode-json programa.mbc
```

Response:

```json
{"ok":true,"input":"programa.mbc","output":["10"]}
```

Event handlers stored in bytecode can be emitted directly:

```bash
.\target\release\matter-cli.exe emit-bytecode-json app.mbc boot
```

Response:

```json
{"ok":true,"input":"app.mbc","event":"boot","output":["Sistema online"]}
```

## Suggested API flow

1. Receive user intent in natural language.
2. Optionally call `matter-cli capabilities-json` to discover the local contract.
3. Ask the AI layer to produce Matter Core source.
4. Run `matter-cli check-json -` with the generated source.
5. If validation passes, run `matter-cli run-json -`, emit an event with `matter-cli emit-json - event_name`, run `matter-cli run -`, or compile with `matter-cli compile-json - -o app.mbc`.
6. For compiled artifacts, run `matter-cli inspect-json app.mbc` when the caller needs bytecode metadata.
7. Run compiled artifacts with `matter-cli run-bytecode-json app.mbc` when the caller needs structured execution output.
8. Emit bytecode events with `matter-cli emit-bytecode-json app.mbc event_name`.
9. Return stdout, stderr, and exit code to the caller.

## Bridge test

The JSON API contract is covered by:

```bash
powershell -ExecutionPolicy Bypass -File .\test_api_bridge.ps1
```

This script validates `capabilities-json`, `eval-json`, `tokens-json`, `imports-json`, `check-json`, `run-json`, `emit-json`, `compile-json`, `inspect-json`, `run-bytecode-json`, `emit-bytecode-json`, runtime errors, and load errors.

For a full system validation, run:

```bash
powershell -ExecutionPolicy Bypass -File .\test_all.ps1
```

This runs Rust tests, release build, bytecode equivalence, and the API bridge contract in order.

## Why this matters

This makes Matter Core a contract between natural language and execution.
People can ask in Portuguese or any other language, while the system stores, validates, compiles, and executes Matter Core underneath.
