# Build matter-cli on D: (project-local target/) with MinGW-w64.
# Usage:
#   .\scripts\build-matter-cli.ps1 -Release
#   .\scripts\build-matter-cli.ps1 -Release -ExperimentalFull

param(
    [switch]$Release,
    [switch]$ExperimentalFull
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

# Build-host toolchain only (NOT required at runtime on destination machines).
# Override with MATTER_MINGW_BIN / existing CC/CXX if MinGW is elsewhere.
$mingwBin = $env:MATTER_MINGW_BIN
if (-not $mingwBin) {
    foreach ($c in @("D:\mingw64\mingw64\bin", "C:\mingw64\mingw64\bin", "C:\msys64\mingw64\bin")) {
        if (Test-Path (Join-Path $c "gcc.exe")) { $mingwBin = $c; break }
    }
}
if (-not $mingwBin -or -not (Test-Path (Join-Path $mingwBin "gcc.exe"))) {
    $gcc = Get-Command gcc -ErrorAction SilentlyContinue
    if ($gcc) {
        $mingwBin = Split-Path -Parent $gcc.Source
    } else {
        throw "GCC not found. Set MATTER_MINGW_BIN to your mingw64\bin directory (build host only)."
    }
}

$env:PATH = "$mingwBin;" + $env:PATH
if (-not $env:CC) { $env:CC = Join-Path $mingwBin "gcc.exe" }
if (-not $env:CXX) { $env:CXX = Join-Path $mingwBin "g++.exe" }
if (-not $env:DLLTOOL) {
    $dt = Join-Path $mingwBin "dlltool.exe"
    if (Test-Path $dt) { $env:DLLTOOL = $dt }
}
$mingwRoot = Split-Path -Parent $mingwBin
$libCandidates = @(
    (Join-Path $mingwRoot "x86_64-w64-mingw32\lib"),
    (Join-Path $mingwRoot "lib")
) | Where-Object { Test-Path $_ }
if ($libCandidates) {
    $env:LIBRARY_PATH = ($libCandidates -join ";")
}
$env:CC_x86_64_pc_windows_gnu = $env:CC
$env:CXX_x86_64_pc_windows_gnu = $env:CXX
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue

$cargoArgs = @("build", "--target", "x86_64-pc-windows-gnu", "-p", "matter-cli")
if ($Release) { $cargoArgs += "--release" }
$binName = "matter-cli"
if ($ExperimentalFull) {
    $cargoArgs += @("--features", "experimental-full", "--bin", "matter-cli-experimental")
    $binName = "matter-cli-experimental"
} else {
    $cargoArgs += @("--bin", "matter-cli")
}

Write-Host "gcc: $((& gcc --version | Select-Object -First 1))" -ForegroundColor Cyan
Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor Cyan
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$profile = if ($Release) { "release" } else { "debug" }
$exe = Join-Path $Root "target\x86_64-pc-windows-gnu\$profile\$binName.exe"
if (-not (Test-Path $exe)) { throw "Build reported success but exe missing: $exe" }

$item = Get-Item $exe
Write-Host ("OK {0} ({1:N1} MB) edition={2}" -f $item.FullName, ($item.Length / 1MB), $(if ($ExperimentalFull) { "experimental-full" } else { "language-only" })) -ForegroundColor Green
& $exe --help | Select-Object -First 8
exit 0
