# windows-gnu import libraries

## `libshlwapi.a`

Required to **link** `matter-cli` with `--features experimental-full` on
`x86_64-pc-windows-gnu` when the Rust self-contained sysroot does not ship
`libshlwapi.a` (pulled in by GUI deps such as egui/arboard).

- Source used for residual 0.2.0 close: llvm-mingw  
  `D:\dev-tools\llvm-mingw\current\x86_64-w64-mingw32\lib\libshlwapi.a`
- The permanent gate `scripts/test-clean-checkout-build.ps1 -IncludeExperimental`
  copies this file into the clean worktree and, if possible, into the active
  toolchain `self-contained` directory.

Language-only builds do not need this library.
