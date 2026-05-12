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
```

The next bridge is `matter-sentinel-abi`: a `no_std` crate with shared constants, PVM package inspection, opcode validation, and a Matter L3 request header.

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
5. Extract a smaller `matter-kernel-vm` crate from Matter that is `no_std + alloc`.
6. Load precompiled MBC1 bytecode in Sentinel and execute a tiny Matter program.

Only after those steps should the repos be physically fused.
