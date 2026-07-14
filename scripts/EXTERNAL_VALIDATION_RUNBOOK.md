# External Windows Validation Runbook — Matter Core 0.1.0

## What to take to the clean machine

1. `matter-core-0.1.0-windows-x64.zip`  
   SHA-256: `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`
2. `external-windows-validation.ps1` (this folder’s script)
3. Nothing else (no project tree, no `target/`, no toolchains)

## Clean machine checklist

- Not the build PC
- No Rust/Cargo, GCC/MinGW/MSYS2, Python, Node
- No prior `D:\Matter`
- Run as **standard user** first

## Commands

```powershell
# In a folder that contains ONLY the zip + script:
powershell -NoProfile -ExecutionPolicy Bypass -File .\external-windows-validation.ps1 `
  -ZipPath .\matter-core-0.1.0-windows-x64.zip
```

Exit codes:

| Code | Meaning |
|-----:|---------|
| 0 | EXTERNAL_VALIDATION_PASS |
| 1 | EXTERNAL_VALIDATION_FAIL |
| 2 | BLOCKED (environment not independent) |

Copy the entire `WorkRoot` folder (printed at end) back as evidence.

## Do not

- Install DLLs/toolchains to hide failures before recording the original error
- Use `-AllowNonIndependent` for an official PASS
- Modify the frozen ZIP
