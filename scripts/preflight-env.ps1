param(
    [int]$MinFreeGB = 10
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Print-Status {
    param(
        [string]$Label,
        [bool]$Ok,
        [string]$Details
    )
    $tag = if ($Ok) { "PASS" } else { "FAIL" }
    $color = if ($Ok) { "Green" } else { "Red" }
    Write-Host ("[{0}] {1}: {2}" -f $tag, $Label, $Details) -ForegroundColor $color
}

Write-Host "==========================================" -ForegroundColor Yellow
Write-Host "Matter Core Environment Preflight" -ForegroundColor Yellow
Write-Host "Root: $root" -ForegroundColor Yellow
Write-Host "==========================================" -ForegroundColor Yellow

$allOk = $true

# Check core tools
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
$rustup = Get-Command rustup -ErrorAction SilentlyContinue
Print-Status "cargo" ($null -ne $cargo) ($(if ($cargo) { $cargo.Source } else { "not found" }))
Print-Status "rustup" ($null -ne $rustup) ($(if ($rustup) { $rustup.Source } else { "not found" }))
if (-not $cargo -or -not $rustup) { $allOk = $false }

# Check build base and free disk
$buildBase = if ($env:MATTER_BUILD_BASE) { $env:MATTER_BUILD_BASE } else { "C:\matter_core_build" }
$rootPath = [System.IO.Path]::GetPathRoot($buildBase)
$driveLetter = $rootPath.TrimEnd('\').TrimEnd(':')
$drive = Get-PSDrive -Name $driveLetter -ErrorAction SilentlyContinue
if ($drive) {
    $freeGB = [Math]::Round($drive.Free / 1GB, 2)
    $okDisk = $freeGB -ge $MinFreeGB
    Print-Status "Disk space ($rootPath)" $okDisk "$freeGB GB free (min: $MinFreeGB GB)"
    if (-not $okDisk) { $allOk = $false }
}
else {
    Print-Status "Disk space ($rootPath)" $false "drive not found"
    $allOk = $false
}

# LLVM readiness probe
$llvmPrefix = $env:LLVM_SYS_170_PREFIX
$llvmConfig = Get-Command llvm-config -ErrorAction SilentlyContinue
if (-not $llvmConfig -and $llvmPrefix) {
    $cfgExe = Join-Path $llvmPrefix "bin\llvm-config.exe"
    if (Test-Path $cfgExe) {
        $llvmConfig = @{ Source = $cfgExe }
    }
}

if ($llvmConfig) {
    try {
        $version = (& llvm-config --version).Trim()
        $prefix = if ($llvmPrefix) { $llvmPrefix } else { (& llvm-config --prefix).Trim() }
        $header = Join-Path $prefix "include\llvm-c\Target.h"
        $okLLVM = $version.StartsWith("17.") -and (Test-Path $header)
        $details = "version=$version; prefix=$prefix; llvm-c header=" + (Test-Path $header)
        Print-Status "LLVM 17 readiness" $okLLVM $details
    }
    catch {
        Print-Status "LLVM 17 readiness" $false $_.Exception.Message
    }
}
else {
    Print-Status "LLVM 17 readiness" $false "llvm-config not found"
}

Write-Host "==========================================" -ForegroundColor Yellow
if ($allOk) {
    Write-Host "Preflight result: PASS" -ForegroundColor Green
    exit 0
}
Write-Host "Preflight result: ATTENTION NEEDED" -ForegroundColor Yellow
exit 1
