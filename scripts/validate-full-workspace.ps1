param(
    [switch]$RequireLLVM,
    [switch]$SkipFmt,
    [switch]$JsonSummary,
    [switch]$RunPreflight,
    [int]$PreflightMinFreeGB = 10
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$results = New-Object System.Collections.Generic.List[Object]
$startTime = Get-Date

# Optional preflight to fail early on common environment issues.
if ($RunPreflight) {
    $preflightScript = Join-Path $PSScriptRoot "preflight-env.ps1"
    if (Test-Path $preflightScript) {
        Write-Host "Running preflight..." -ForegroundColor Yellow
        & powershell -ExecutionPolicy Bypass -File $preflightScript -MinFreeGB $PreflightMinFreeGB
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Preflight failed. Aborting validation." -ForegroundColor Red
            exit 3
        }
    }
    else {
        Write-Host "Preflight script not found at $preflightScript. Continuing." -ForegroundColor Yellow
    }
}

# Stabilize builds on Windows GNU where paths with spaces can break dlltool/as.
$safeBase = if ($env:MATTER_BUILD_BASE) { $env:MATTER_BUILD_BASE } else { "C:\matter_core_build" }
$safeTarget = Join-Path $safeBase "target"
$safeTemp = Join-Path $safeBase "tmp"
New-Item -ItemType Directory -Path $safeTarget -Force | Out-Null
New-Item -ItemType Directory -Path $safeTemp -Force | Out-Null
$env:CARGO_TARGET_DIR = $safeTarget
$env:TEMP = $safeTemp
$env:TMP = $safeTemp

function Add-Result {
    param(
        [string]$Name,
        [bool]$Passed,
        [double]$DurationSec,
        [string]$Details
    )

    $results.Add([PSCustomObject]@{
            step     = $Name
            passed   = $Passed
            duration = [Math]::Round($DurationSec, 2)
            details  = $Details
        })
}

function Run-Step {
    param(
        [string]$Name,
        [string]$Command,
        [switch]$Critical
    )

    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
    Write-Host "    $Command" -ForegroundColor DarkGray

    $stepStart = Get-Date
    try {
        Invoke-Expression $Command
        if ($LASTEXITCODE -ne 0) {
            throw "Command exited with code $LASTEXITCODE"
        }
        $duration = ((Get-Date) - $stepStart).TotalSeconds
        Add-Result -Name $Name -Passed $true -DurationSec $duration -Details "ok"
        Write-Host "    PASS ($([Math]::Round($duration, 2))s)" -ForegroundColor Green
        return $true
    }
    catch {
        $duration = ((Get-Date) - $stepStart).TotalSeconds
        Add-Result -Name $Name -Passed $false -DurationSec $duration -Details $_.Exception.Message
        Write-Host "    FAIL ($([Math]::Round($duration, 2))s)" -ForegroundColor Red
        Write-Host "    $($_.Exception.Message)" -ForegroundColor Red
        if ($Critical) {
            throw
        }
        return $false
    }
}

Write-Host "==========================================" -ForegroundColor Yellow
Write-Host "Matter Core Full Workspace Validation" -ForegroundColor Yellow
Write-Host "Root: $root" -ForegroundColor Yellow
Write-Host "Target Dir: $env:CARGO_TARGET_DIR" -ForegroundColor Yellow
Write-Host "Temp Dir: $env:TEMP" -ForegroundColor Yellow
Write-Host "==========================================" -ForegroundColor Yellow

$llvmConfig = Get-Command llvm-config -ErrorAction SilentlyContinue
$llvmDetected = $false
$llvmVersion = $null
$llvmPrefix = $env:LLVM_SYS_170_PREFIX
$llvmReadyReason = "llvm-config not found in PATH"

if (-not $llvmConfig -and $llvmPrefix) {
    $candidateExe = Join-Path $llvmPrefix "bin\llvm-config.exe"
    $candidateCmd = Join-Path $llvmPrefix "bin\llvm-config"
    if (Test-Path $candidateExe) {
        $llvmConfig = @{ Source = $candidateExe }
        $env:Path = (Join-Path $llvmPrefix "bin") + ";" + $env:Path
    }
    elseif (Test-Path $candidateCmd) {
        $llvmConfig = @{ Source = $candidateCmd }
        $env:Path = (Join-Path $llvmPrefix "bin") + ";" + $env:Path
    }
}
$cargoPrefix = "cargo"

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $toolchains = (rustup toolchain list) -join "`n"
    $hasLinkExe = $null -ne (Get-Command link.exe -ErrorAction SilentlyContinue)
    if (($toolchains -match "stable-x86_64-pc-windows-msvc") -and $hasLinkExe) {
        # Prefer MSVC toolchain on Windows to avoid GNU dlltool path/space issues.
        $cargoPrefix = "cargo +stable-x86_64-pc-windows-msvc"
    }
}

if ($llvmConfig) {
    try {
        $llvmVersion = (& llvm-config --version).Trim()
        $llvmDetected = $llvmVersion.StartsWith("17.")
        if (-not $llvmDetected) {
            $llvmReadyReason = "llvm-config returned version '$llvmVersion' (expected 17.x)"
        }
        if (-not $llvmPrefix) {
            $derivedPrefix = (& llvm-config --prefix).Trim()
            if ($derivedPrefix) {
                $llvmPrefix = $derivedPrefix
                $env:LLVM_SYS_170_PREFIX = $derivedPrefix
            }
        }
        if ($llvmDetected) {
            $llvmHeader = Join-Path $llvmPrefix "include\\llvm-c\\Target.h"
            if (-not (Test-Path $llvmHeader)) {
                $llvmDetected = $false
                $llvmReadyReason = "missing LLVM C headers at '$llvmHeader'"
            }
        }
    }
    catch {
        $llvmDetected = $false
        $llvmReadyReason = $_.Exception.Message
    }
}

if ($llvmDetected) {
    Write-Host "LLVM: detected ($llvmVersion)" -ForegroundColor Green
    Write-Host "LLVM_SYS_170_PREFIX: $llvmPrefix" -ForegroundColor Green
}
else {
    Write-Host "LLVM: not ready for matter-llvm checks (expected 17.x)." -ForegroundColor Yellow
    Write-Host "Reason: $llvmReadyReason" -ForegroundColor Yellow
    if ($RequireLLVM) {
        Write-Host "RequireLLVM enabled; aborting." -ForegroundColor Red
        exit 2
    }
    Write-Host "Continuing with non-LLVM workspace validation." -ForegroundColor Yellow
}

try {
    $workspaceClippyCmd = "$cargoPrefix clippy --workspace --all-targets -- -D warnings"
    $workspaceTestCmd = "$cargoPrefix test --workspace --exclude matter-wasm"
    if (-not $llvmDetected) {
        # Keep full validation flow operational in environments without LLVM.
        $workspaceClippyCmd = "$cargoPrefix clippy --workspace --exclude matter-llvm --all-targets -- -D warnings"
        $workspaceTestCmd = "$cargoPrefix test --workspace --exclude matter-llvm --exclude matter-wasm"
    }

    if (-not $SkipFmt) {
        Run-Step -Name "Format check" -Command "$cargoPrefix fmt --all -- --check" -Critical
    }
    else {
        Add-Result -Name "Format check" -Passed $true -DurationSec 0 -Details "skipped"
    }

    Run-Step -Name "Clippy workspace (strict)" -Command $workspaceClippyCmd -Critical
    Run-Step -Name "Workspace tests" -Command $workspaceTestCmd -Critical

    if ($llvmDetected) {
        Run-Step -Name "Clippy matter-llvm (strict)" -Command "$cargoPrefix clippy -p matter-llvm --all-targets -- -D warnings" -Critical
        Run-Step -Name "Build CLI with LLVM feature" -Command "$cargoPrefix build -p matter-cli --features llvm" -Critical
    }
    else {
        Add-Result -Name "Clippy matter-llvm (strict)" -Passed $true -DurationSec 0 -Details "skipped (LLVM unavailable)"
        Add-Result -Name "Build CLI with LLVM feature" -Passed $true -DurationSec 0 -Details "skipped (LLVM unavailable)"
    }
}
catch {
}

$endTime = Get-Date
$elapsed = ($endTime - $startTime).TotalSeconds
$failed = @($results | Where-Object { -not $_.passed })
$success = $failed.Count -eq 0

Write-Host ""
Write-Host "========== Validation Summary ==========" -ForegroundColor Yellow
foreach ($r in $results) {
    $status = if ($r.passed) { "PASS" } else { "FAIL" }
    $color = if ($r.passed) { "Green" } else { "Red" }
    Write-Host ("[{0}] {1} ({2}s) - {3}" -f $status, $r.step, $r.duration, $r.details) -ForegroundColor $color
}
Write-Host ("Total duration: {0}s" -f [Math]::Round($elapsed, 2)) -ForegroundColor Yellow
Write-Host ("Result: {0}" -f ($(if ($success) { "PASS" } else { "FAIL" }))) -ForegroundColor $(if ($success) { "Green" } else { "Red" })

if ($JsonSummary) {
    $summary = [PSCustomObject]@{
        timestamp     = (Get-Date).ToString("o")
        root          = $root
        success       = $success
        llvm_detected = $llvmDetected
        llvm_version  = $llvmVersion
        elapsed_sec   = [Math]::Round($elapsed, 2)
        steps         = $results
    }
    Write-Host ""
    $summary | ConvertTo-Json -Depth 6
}

if ($success) {
    exit 0
}
exit 1
