# Matter + Sentinel Integration

The target architecture is one product, with Sentinel as the native operating system and Matter as the language/runtime layer inside it.

The safe merge path is not to copy the full Matter workspace into the kernel. Matter currently contains desktop tooling, `std` crates, CLI commands, host bridges, docs, CI tools, and release packaging. Sentinel is a `no_std` kernel. The integration needs a narrow ABI first.

## Repository Shape

Short term:

- Matter remains the source of truth for the language, compiler, bytecode tools, and export commands.
- Sentinel remains the source of truth for boot, framebuffer, shell, scheduler, filesystems, PVM, and QEMU.
- `matter-sentinel-abi` is the shared `no_std` contract crate that both sides can compile.

Final product shape:

```text
sentinel/
  kernel/
  apps/
  pvm/
  third_party/matter-core/
    crates/matter-sentinel-abi/
    crates/matter-kernel-vm/
    examples/
```

Sentinel should become the root product repository when the dirty OS worktree is clean enough to move safely. Matter can be vendored as `third_party/matter-core` or added as a Git submodule/subtree. The kernel should depend only on `no_std` Matter crates.

## Runtime Layers

```text
Host Windows / developer machine
  Matter CLI
    .matter source -> MBC1 bytecode
    visual.* -> PVM2 .pvmbc
    inspect/guard/benchmark

Sentinel kernel / QEMU
  L0 boot + memory + framebuffer
  L1 scheduler + filesystem + shell
  L2 PVM visual package loader
  L3 Matter service
    load PVM2
    inspect package
    run visual app
    later: run MBC1 Matter bytecode
```

The first bridge is already implemented:

```powershell
matter-cli sentinel-pvmbc examples\matter_studio_ui.matter -o target\matter-studio.pvmbc --name matter-studio
matter-cli sentinel-pvmbc-inspect-json target\matter-studio.pvmbc
matter-cli sentinel-pvmbc-rust-array examples\matter_studio_ui.matter --const MATTER_STUDIO_PVMBC --name matter-studio
matter-cli sentinel-mbc1-rust-array examples\sentinel_boot.matter --const MATTER_BOOT_MBC1
matter-cli sentinel-mbc1-kernel-check-json examples\sentinel_boot.matter --budget 10000
```

The next bridge is `matter-sentinel-abi`: a `no_std` crate with shared constants, PVM package inspection, opcode validation, and a Matter L3 request header.

Use `sentinel-pvmbc` when the package should live on a Sentinel disk image. Use `sentinel-pvmbc-rust-array` when the same package should be compiled directly into a Sentinel kernel catalog as a `pub const &[u8]`.
Use `sentinel-mbc1-rust-array` when a compiled Matter program should be embedded beside it as `MBC1` input for `matter-kernel-vm`.
Use `sentinel-mbc1-kernel-check-json` before embedding to prove the source compiles to MBC1, passes `matter-kernel-vm` inspection, and runs within a bounded instruction budget.

The first kernel-side Matter execution crate is `matter-kernel-vm`. It is `no_std + alloc`, validates real `MBC1` bytecode, reports section counts, and runs a controlled integer subset with globals, function-local variables, branches, loops, named calls, bounded recursion, and captured backend calls as kernel-visible syscalls. Runtime telemetry includes executed instruction count, maximum call depth, and requested syscalls, so Sentinel can prove recursive Matter ran within limits before granting any OS service. The first syscall family is `sentinel.telemetry`, `sentinel.log`, `sentinel.screen`, `sentinel.file`, and `sentinel.process`; `log` writes to the kernel log, `screen` opens a Sentinel alert window, `file` writes only to the fixed capability path `/matter/boot.log`, and `process` writes an audit-only scheduler request to `/matter/process.request`.

## Why L3

Matter should not start as kernel code with unlimited power. It should run as a controlled L3 service:

- Sentinel owns memory, windows, input, and process scheduling.
- Matter owns language execution and app logic.
- The ABI carries requests such as `LoadPvmbc`, `InspectPvmbc`, `RunVisualApp`, and later `RunMatterBytecode`.

That keeps the OS stable while the language becomes native.

## Next Build Steps

1. Add `matter-sentinel-abi` to the Sentinel build as a path dependency.
2. Create a Sentinel `matter_l3` module that calls `inspect_pvmbc` on bytes loaded from the filesystem.
3. Add a Sentinel shell command like `matter inspect /pvm/apps/matter-studio.pvmbc`.
4. Boot QEMU and verify the OS can inspect the Matter-generated package internally.
5. Generate embedded Sentinel catalog constants from Matter with `sentinel-pvmbc-rust-array`.
6. Generate embedded Matter program constants with `sentinel-mbc1-rust-array`.
7. Use `sentinel-mbc1-kernel-check-json` in CI for every Matter boot payload.
8. Wire Sentinel to `matter-kernel-vm::inspect_mbc1` and `matter-kernel-vm::run_mbc1_main`.
9. Load precompiled MBC1 bytecode in Sentinel and execute a tiny Matter program.
10. Expand `matter-kernel-vm` from integer execution toward more controlled value types and capability-scoped OS services.

Only after those steps should the repos be physically fused.
