# Matter Core User Onboarding

This is the first-user path. Optimize this before adding more features.

## First Audience

Primary first user: a developer who wants a small embeddable scripting language with bytecode, VM execution, JSON automation, and guarded reflection.

Secondary users:

- students learning how language runtimes work
- builders experimenting with safe DSLs
- agent/tooling developers who need machine-readable validation

## First Three Commands

After downloading a release archive and opening PowerShell in the extracted folder:

```powershell
.\matter-cli.exe run examples\first_run.matter
.\matter-cli.exe reflect-json examples\first_run.matter
.\matter-cli.exe reflexive-guard-json examples\first_run.matter
```

The first command proves execution. The second proves reflection. The third proves guarded self-analysis.

The next command teaches syntax:

```powershell
.\matter-cli.exe run examples\language_tour.matter
```

## Release Friction Checklist

- Windows zip contains `matter-cli.exe`
- zip contains `examples\first_run.matter`
- zip contains `examples\language_tour.matter`
- zip contains `README.md`
- zip contains `LANGUAGE_TOUR.md`
- README starts with binary download path before Rust build instructions
- release page shows the three commands above
- no command in the first path requires Rust, Git, network access, or source-code reading

## Positioning

Matter Core is not positioned as a general replacement for Python, Rust, or JavaScript.

First message:

> Matter Core is an experimental embeddable language runtime: source files compile to bytecode, run on a VM, expose JSON tooling, and can inspect their own program structure safely.
