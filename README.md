# Matter Core

Matter Core is an experimental embeddable language runtime.

It has a source language (`.matter`), parser, AST, bytecode format, VM/runtime, CLI tooling, JSON automation commands, events, package manifests, and guarded reflection.

In one sentence:

> Matter Core runs small programs on its own VM and can inspect program structure before execution.

## First Run

Download the latest Windows release zip from GitHub Releases, extract it, open PowerShell in the extracted folder, then run:

```powershell
.\matter-cli.exe run examples\first_run.matter
.\matter-cli.exe reflect-json examples\first_run.matter
.\matter-cli.exe reflexive-guard-json examples\first_run.matter
```

Those three commands prove the core loop:

- `run`: source -> bytecode -> VM -> output
- `reflect-json`: source -> structured program facts
- `reflexive-guard-json`: reflection -> safety policy decision

No Rust install is required when using a release zip.

To learn the syntax, run the executable tour:

```powershell
.\matter-cli.exe run examples\language_tour.matter
```

Launch the native Rust terminal studio:

```powershell
.\matter-cli.exe studio-native examples\matter_studio_ui.matter
.\matter-cli.exe studio-native examples\matter_studio_ui.matter --interactive
.\matter-cli.exe studio-native-json examples\matter_studio_ui.matter
.\matter-cli.exe sentinel-pvmbc examples\matter_studio_ui.matter -o target\matter-studio.pvmbc --name matter-studio
```

## Build From Source

If you are developing Matter itself:

```powershell
git clone https://github.com/AlmirPro25/mater.git
cd mater
cargo run -q -p matter-cli -- run examples\first_run.matter
```

Build the CLI:

```powershell
cargo build -p matter-cli --release
.\target\release\matter-cli.exe run examples\first_run.matter
```

## What Matter Can Do Today

- execute `.matter` source files
- compile source into MBC1 bytecode
- run bytecode on the Matter VM
- define functions and use recursion
- use variables, lists, maps, structs, branches, loops, and events
- expose JSON commands for tools, agents, and CI
- inspect source structure with `reflect-json`
- evaluate guarded reflexive workflows with `reflexive-guard-json`
- benchmark programs and gate performance budgets
- run a local visual AI workbench through Matter Studio
- declare UI layouts in Matter with `visual.*` and preview them in Matter Studio
- render a native Rust terminal studio with `studio-native`
- export Matter visual layouts to Sentinel OS PVM2 bytecode with `sentinel-pvmbc`

## Example

```matter
fn fib(n) {
    if n <= 1 {
        return n
    }

    return fib(n - 1) + fib(n - 2)
}

print "fib(8)"
print fib(8)
```

Run it:

```powershell
.\matter-cli.exe run examples\first_run.matter
```

Inspect it as data:

```powershell
.\matter-cli.exe reflect-json examples\first_run.matter
```

Guard it before a reflexive/self-modifying workflow:

```powershell
.\matter-cli.exe reflexive-guard-json examples\first_run.matter
```

## CLI Highlights

- `run` / `run-json`: execute Matter source.
- `check` / `check-json`: parse and compile without running.
- `compile` / `compile-json`: emit Matter bytecode.
- `run-bytecode` / `run-bytecode-json`: execute compiled bytecode.
- `reflect-json`: inspect source as AST and bytecode facts.
- `reflexive-guard-json`: evaluate safety gates for generated or self-mutating code.
- `benchmark-json`: measure execution time with machine-readable stats.
- `benchmark-gate-json`: enforce performance budgets.
- `init` / `init-json`: scaffold a Matter project.
- `project-*`: operate on `matter.toml` package manifests.

Machine-readable capabilities:

```powershell
.\matter-cli.exe capabilities-json
```

## Matter Studio

Matter Studio is a local visual interface for the language. It provides a dark chat/workbench UI, calls Matter CLI locally, renders Matter `visual.*` UI declarations, and can connect to OpenAI-compatible or Gemini APIs through server-side environment variables.

```powershell
cd apps\matter-studio
copy .env.example .env
npm start
```

Open:

```text
http://127.0.0.1:4177
```

API keys stay in `apps/matter-studio/.env`; they are not sent to the browser as raw keys.

## Native Studio

The native path is `studio-native`: a Rust CLI shell that renders a Matter `visual.*` program directly in the terminal. This is the foundation for a non-web Matter interface.

```powershell
.\matter-cli.exe studio-native examples\matter_studio_ui.matter
```

Use `--interactive` to keep the native shell open with commands for run, check, visual refresh, and guard.
Inside the interactive shell, `tap Run`, `tap Reflect`, and `tap Guard` dispatch actions from Matter-declared visual regions.

## Sentinel OS Bridge

Matter can export a `visual.*` interface as Sentinel-compatible `PVM2` bytecode. This is the bridge between the language and the native operating system: Matter describes the surface, Sentinel loads the `.pvmbc`.

```powershell
.\matter-cli.exe sentinel-pvmbc examples\matter_studio_ui.matter -o target\matter-studio.pvmbc --name matter-studio
```

The generated file can be copied into a Sentinel disk image and loaded from the Sentinel PVM shell with its `installpvmbc` / `loadpvmbc` flow.

## Who This Is For

Matter Core is currently best for:

- developers exploring embeddable scripting and VM design
- agent/tooling builders who need JSON validation and execution
- students learning how lexers, parsers, bytecode, and VMs fit together
- DSL experiments where code needs to be inspected before it runs

It is not yet a polished general-purpose language distribution. The runtime is experimental, but the core pipeline is real and tested.

## Repository Layout

```text
crates/      Rust workspace crates
docs/        Technical documentation and onboarding notes
examples/    Matter source examples and demo apps
stdlib/      Matter standard library material
tests/       Workspace integration tests
```

## Development

Format and test:

```powershell
cargo fmt --all
cargo check -p matter-cli
cargo test --workspace --all-targets
```

The repository uses `.cargo/config.toml` to place build output outside this directory. This avoids Windows toolchain failures caused by spaces in the workspace path.

## Documentation

Start here:

- [User onboarding](docs/USER_ONBOARDING.md)
- [Language tour](docs/LANGUAGE_TOUR.md)
- [Reflexive core](docs/REFLEXIVE_CORE.md)
- [Build status](docs/BUILD_STATUS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Language spec](docs/SPEC.md)
- [Documentation index](docs/INDEX.md)

## GitHub Repository Metadata

Suggested repository description:

```text
Experimental embeddable language runtime: parser, bytecode, VM, CLI, JSON tooling, and guarded reflection.
```

Suggested topics:

```text
rust language-runtime bytecode vm compiler cli scripting-language dsl
```

## License

MIT. See [LICENSE](LICENSE).
