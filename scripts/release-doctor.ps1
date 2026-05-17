param(
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Test-CommandExists([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function New-CheckResult([string]$Name, [string]$Status, [string]$Details) {
    return [PSCustomObject]@{
        name = $Name
        status = $Status
        details = $Details
    }
}

function Print-Check([object]$Check) {
    $color = switch ($Check.status) {
        "PASS" { "Green" }
        "WARN" { "Yellow" }
        default { "Red" }
    }
    Write-Host ("[{0}] {1}: {2}" -f $Check.status, $Check.name, $Check.details) -ForegroundColor $color
}

$checks = New-Object System.Collections.Generic.List[Object]

# Core toolchain
$checks.Add((New-CheckResult "cargo" ($(if (Test-CommandExists "cargo") { "PASS" } else { "FAIL" })) ($(if (Test-CommandExists "cargo") { "found" } else { "missing" }))))
$checks.Add((New-CheckResult "rustup" ($(if (Test-CommandExists "rustup") { "PASS" } else { "FAIL" })) ($(if (Test-CommandExists "rustup") { "found" } else { "missing" }))))

# Optional runtimes
$checks.Add((New-CheckResult "go" ($(if (Test-CommandExists "go") { "PASS" } else { "WARN" })) ($(if (Test-CommandExists "go") { "found (native Go smoke enabled)" } else { "missing (use -SkipNativeFfiSmoke)" }))))
$checks.Add((New-CheckResult "node" ($(if (Test-CommandExists "node") { "PASS" } else { "WARN" })) ($(if (Test-CommandExists "node") { "found (Node bridge tests enabled)" } else { "missing (skip Node-dependent smoke)" }))))

# JAVA_HOME readiness
$javaHome = $env:JAVA_HOME
if ($javaHome) {
    $javaExe = Join-Path $javaHome "bin\java.exe"
    if (Test-Path $javaExe) {
        $checks.Add((New-CheckResult "JAVA_HOME" "PASS" "$javaExe"))
    }
    else {
        $checks.Add((New-CheckResult "JAVA_HOME" "WARN" "set but invalid path (java.exe not found)"))
    }
}
else {
    $checks.Add((New-CheckResult "JAVA_HOME" "WARN" "not set (use -SkipNativeFfiSmoke or avoid -IncludeJavaNativeSmoke)"))
}

# LLVM 17 readiness for matter-llvm
$llvmConfig = Get-Command llvm-config -ErrorAction SilentlyContinue
$llvmPrefix = $env:LLVM_SYS_170_PREFIX
if (-not $llvmConfig -and $llvmPrefix) {
    $cfgExe = Join-Path $llvmPrefix "bin\llvm-config.exe"
    if (Test-Path $cfgExe) {
        $llvmConfig = @{ Source = $cfgExe }
    }
}

if ($llvmConfig) {
    try {
        $llvmConfigExe = $llvmConfig.Source
        $version = (& $llvmConfigExe --version).Trim()
        $prefix = if ($llvmPrefix) { $llvmPrefix } else { (& $llvmConfigExe --prefix).Trim() }
        $header = Join-Path $prefix "include\llvm-c\Target.h"
        if ($version.StartsWith("17.") -and (Test-Path $header)) {
            $checks.Add((New-CheckResult "LLVM 17" "PASS" "version=$version; header=ok"))
        }
        else {
            $checks.Add((New-CheckResult "LLVM 17" "WARN" "version=$version; header_exists=$([bool](Test-Path $header))"))
        }
    }
    catch {
        $checks.Add((New-CheckResult "LLVM 17" "WARN" $_.Exception.Message))
    }
}
else {
    $checks.Add((New-CheckResult "LLVM 17" "WARN" "llvm-config not found (matter-llvm checks will be skipped)"))
}

$failCount = @($checks | Where-Object { $_.status -eq "FAIL" }).Count
$warnCount = @($checks | Where-Object { $_.status -eq "WARN" }).Count

$recommendedValidateCmd = if ($warnCount -gt 0) {
    ".\scripts\validate-full-workspace.ps1 -RunPreflight -SkipNativeFfiSmoke -SkipRustFfiSmoke"
}
else {
    ".\scripts\validate-full-workspace.ps1 -RunPreflight -IncludeJavaNativeSmoke"
}

$summary = [PSCustomObject]@{
    root = $root
    generated_at = (Get-Date).ToString("s")
    fail_count = $failCount
    warn_count = $warnCount
    recommended_validate_command = $recommendedValidateCmd
    checks = $checks
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 5
    if ($failCount -gt 0) { exit 2 }
    if ($warnCount -gt 0) { exit 1 }
    exit 0
}

Write-Host "==========================================" -ForegroundColor Yellow
Write-Host "Matter Core Release Doctor" -ForegroundColor Yellow
Write-Host "Root: $root" -ForegroundColor Yellow
Write-Host "==========================================" -ForegroundColor Yellow
foreach ($check in $checks) {
    Print-Check $check
}
Write-Host "------------------------------------------" -ForegroundColor DarkGray
Write-Host "Recommended next command:" -ForegroundColor Cyan
Write-Host "  $recommendedValidateCmd" -ForegroundColor White

if ($failCount -gt 0) {
    Write-Host "Doctor result: FAIL" -ForegroundColor Red
    exit 2
}
if ($warnCount -gt 0) {
    Write-Host "Doctor result: WARN" -ForegroundColor Yellow
    exit 1
}
Write-Host "Doctor result: PASS" -ForegroundColor Green
exit 0
