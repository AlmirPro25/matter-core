param(
    [switch]$Quick,
    [switch]$QuickJson,
    [string]$JsonOut,
    [string]$CiOutputFile,
    [switch]$EmitCiOutput,
    [switch]$RequireLLVM,
    [switch]$SkipFmt,
    [switch]$SkipRunnableExamples,
    [switch]$SkipRustFfiSmoke,
    [switch]$SkipNativeFfiSmoke,
    [switch]$IncludeJavaNativeSmoke,
    [switch]$IncludeNodeNativeUnitTests,
    [switch]$JsonSummary,
    [switch]$RunPreflight,
    [switch]$RunDoctor,
    [switch]$RequireDoctorPass,
    [switch]$CiMode,
    [int]$PreflightMinFreeGB = 10,
    [switch]$RunAiCanonicalFlow,
    [string]$AiFlowProgramPath = "examples\\first_run.matter",
    [string]$AiFlowOutDir = "target\\ai-flow",
    [int]$AiFlowBenchmarkIterations = 20,
    [switch]$AiFlowSkipBenchmarkGate,
    [switch]$EnforceStatusTriadLatency,
    [double]$StatusTriadMaxCoreMs = 60000,
    [double]$StatusTriadMaxWorldMs = 60000,
    [double]$StatusTriadMaxFrontierMs = 60000,
    [switch]$EnforceStatusTriadDrift,
    [double]$StatusTriadDriftTolerancePercent = 50,
    [string]$StatusTriadDriftBaselineJson = "scripts\\status-triad-latency-baseline.json"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$results = New-Object System.Collections.Generic.List[Object]
$startTime = Get-Date

if ($QuickJson) {
    $Quick = $true
    $JsonSummary = $true
}

if ($JsonOut) {
    $JsonSummary = $true
}

if ($CiOutputFile) {
    $EmitCiOutput = $true
}

if ($Quick) {
    # Fast local feedback: keep core quality gates and skip heavier release/smoke checks.
    $SkipRunnableExamples = $true
    $SkipRustFfiSmoke = $true
    $SkipNativeFfiSmoke = $true
}

if ($CiMode -and -not $PSBoundParameters.ContainsKey("EnforceStatusTriadLatency")) {
    $EnforceStatusTriadLatency = $true
}
if ($CiMode -and -not $PSBoundParameters.ContainsKey("EnforceStatusTriadDrift")) {
    $EnforceStatusTriadDrift = $true
}

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

# Optional release doctor for environment/readiness diagnostics.
if ($RunDoctor) {
    $doctorScript = Join-Path $PSScriptRoot "release-doctor.ps1"
    if (Test-Path $doctorScript) {
        Write-Host "Running release doctor..." -ForegroundColor Yellow
        & powershell -ExecutionPolicy Bypass -File $doctorScript
        $doctorExit = $LASTEXITCODE
        if ($doctorExit -eq 2) {
            Write-Host "Release doctor reported FAIL. Aborting validation." -ForegroundColor Red
            exit 4
        }
        if (($doctorExit -eq 1) -and $RequireDoctorPass) {
            Write-Host "Release doctor reported WARN and -RequireDoctorPass was enabled. Aborting validation." -ForegroundColor Red
            exit 4
        }
    }
    else {
        Write-Host "Release doctor script not found at $doctorScript. Continuing." -ForegroundColor Yellow
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
        if (-not $IncludeNodeNativeUnitTests) {
            # matter-bridge-nodejs-native unit tests can spam Node-API symbol lookup logs
            # when not running under a Node host process. Native smoke covers bridge health.
            $workspaceTestCmd += " --exclude matter-bridge-nodejs-native"
        }
    }

    if (-not $SkipFmt) {
        $fmtCmd = "$cargoPrefix fmt --all -- --check"
        if ($CiMode) {
            $fmtCmd = "$cargoPrefix fmt --all -- --check -q"
        }
        Run-Step -Name "Format check" -Command $fmtCmd -Critical
    }
    else {
        Add-Result -Name "Format check" -Passed $true -DurationSec 0 -Details "skipped"
    }

    if ($CiMode) {
        $workspaceTestCmd = "$workspaceTestCmd -q"
    }
    Run-Step -Name "Clippy workspace (strict)" -Command $workspaceClippyCmd -Critical
    Run-Step -Name "Workspace tests" -Command $workspaceTestCmd -Critical
    $triadOut = "target\validation\status-triad-latest.json"
    $triadHistory = "target\validation\status-triad-history.ndjson"
    $triadCommand = "powershell -ExecutionPolicy Bypass -File .\scripts\test-status-triad-contract.ps1 -OutJson `"$triadOut`" -HistoryJsonl `"$triadHistory`""
    if ($EnforceStatusTriadLatency) {
        $triadCommand = "$triadCommand -EnforceLatencyBudget -MaxCoreMs $StatusTriadMaxCoreMs -MaxWorldMs $StatusTriadMaxWorldMs -MaxFrontierMs $StatusTriadMaxFrontierMs"
    }
    if ($EnforceStatusTriadDrift) {
        $triadCommand = "$triadCommand -EnforceLatencyDrift -DriftTolerancePercent $StatusTriadDriftTolerancePercent -DriftBaselineJson `"$StatusTriadDriftBaselineJson`""
    }
    Run-Step -Name "Status triad contract" -Command $triadCommand -Critical

    if ($RunAiCanonicalFlow) {
        $aiFlowCommand = "powershell -ExecutionPolicy Bypass -File .\scripts\ai-app-canonical-flow.ps1 -ProgramPath `"$AiFlowProgramPath`" -BenchmarkIterations $AiFlowBenchmarkIterations"
        $aiFlowCommand = "$aiFlowCommand -OutDir `"$AiFlowOutDir`""
        if ($AiFlowSkipBenchmarkGate) {
            $aiFlowCommand = "$aiFlowCommand -SkipBenchmarkGate"
        }
        Run-Step -Name "AI app canonical flow" -Command $aiFlowCommand -Critical
    }
    else {
        Add-Result -Name "AI app canonical flow" -Passed $true -DurationSec 0 -Details "skipped"
    }

    if (-not $SkipRunnableExamples) {
        Run-Step -Name "Runnable examples contract" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\test-runnable-examples.ps1 -JsonSummary" -Critical
    }
    else {
        Add-Result -Name "Runnable examples contract" -Passed $true -DurationSec 0 -Details "skipped"
    }

    if (-not $SkipRustFfiSmoke) {
        Run-Step -Name "Rust FFI plugin smoke" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1 -JsonOut target\ffi\rust-smoke.json" -Critical
    }
    else {
        Add-Result -Name "Rust FFI plugin smoke" -Passed $true -DurationSec 0 -Details "skipped"
    }

    if (-not $SkipNativeFfiSmoke) {
        $nativeFfiCommand = "powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1"
        if ($IncludeJavaNativeSmoke) {
            $nativeFfiCommand = "$nativeFfiCommand -IncludeJava"
        }
        $nativeFfiCommand = "$nativeFfiCommand -JsonOut target\ffi\native-smoke.json"
        Run-Step -Name "Native FFI smoke" -Command $nativeFfiCommand -Critical
    }
    else {
        Add-Result -Name "Native FFI smoke" -Passed $true -DurationSec 0 -Details "skipped"
    }

    if ((-not $SkipRustFfiSmoke) -and (-not $SkipNativeFfiSmoke)) {
        $verifyFfiCommand = "powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1"
        if ($IncludeJavaNativeSmoke) {
            $verifyFfiCommand = "$verifyFfiCommand -RequireJava"
        }
        Run-Step -Name "Verify FFI smoke summaries" -Command $verifyFfiCommand -Critical
        Run-Step -Name "Export FFI validation matrix" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\export-ffi-validation-matrix.ps1 -Out target\ffi\ffi-validation-matrix.json" -Critical
        $verifyFfiMatrixCommand = "powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix"
        if ($IncludeJavaNativeSmoke) {
            $verifyFfiMatrixCommand = "$verifyFfiMatrixCommand -RequireJava"
        }
        Run-Step -Name "Verify FFI validation matrix" -Command $verifyFfiMatrixCommand -Critical
        Run-Step -Name "Export FFI validation report" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\export-ffi-validation-report.ps1 -Out target\ffi\ffi-validation-report.md" -Critical
        Run-Step -Name "Test FFI validation report contract" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-report-contract.ps1" -Critical
        Run-Step -Name "Test FFI validation matrix contract" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-matrix-contract.ps1" -Critical
        Run-Step -Name "Test release package contract" -Command "powershell -ExecutionPolicy Bypass -File .\scripts\test-release-package-contract.ps1" -Critical
    }
    else {
        Add-Result -Name "Verify FFI smoke summaries" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Export FFI validation matrix" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Verify FFI validation matrix" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Export FFI validation report" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Test FFI validation report contract" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Test FFI validation matrix contract" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
        Add-Result -Name "Test release package contract" -Passed $true -DurationSec 0 -Details "skipped (smoke skipped)"
    }

    if ($llvmDetected) {
        $llvmClippyCmd = "$cargoPrefix clippy -p matter-llvm --all-targets -- -D warnings"
        $llvmBuildCmd = "$cargoPrefix build -p matter-cli --features llvm"
        if ($CiMode) {
            $llvmBuildCmd = "$llvmBuildCmd -q"
        }
        Run-Step -Name "Clippy matter-llvm (strict)" -Command $llvmClippyCmd -Critical
        Run-Step -Name "Build CLI with LLVM feature" -Command $llvmBuildCmd -Critical
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

if ($EmitCiOutput) {
    $statusValue = if ($success) { "pass" } else { "fail" }
    $lines = @(
        "status=$statusValue"
        "success=$($success.ToString().ToLowerInvariant())"
        "elapsed_sec=$([Math]::Round($elapsed, 2))"
        "llvm_detected=$($llvmDetected.ToString().ToLowerInvariant())"
        "failed_steps=$($failed.Count)"
    )

    if ($CiOutputFile) {
        $ciOutputPath = $CiOutputFile
        if (-not [System.IO.Path]::IsPathRooted($ciOutputPath)) {
            $ciOutputPath = Join-Path $root $ciOutputPath
        }
        $ciOutputDir = Split-Path -Parent $ciOutputPath
        if ($ciOutputDir) {
            New-Item -ItemType Directory -Path $ciOutputDir -Force | Out-Null
        }
        Set-Content -Path $ciOutputPath -Value $lines -Encoding UTF8
        Write-Host "Wrote CI output to $ciOutputPath" -ForegroundColor Green
    }
    else {
        Write-Host ""
        Write-Host "CI Outputs:" -ForegroundColor Yellow
        foreach ($line in $lines) {
            Write-Host $line
        }
    }
}

if ($JsonSummary) {
    $summary = [PSCustomObject]@{
        timestamp     = (Get-Date).ToString("o")
        root          = $root
        success       = $success
        llvm_detected = $llvmDetected
        llvm_version  = $llvmVersion
        elapsed_sec   = [Math]::Round($elapsed, 2)
        ai_flow       = [PSCustomObject]@{
            enabled = [bool]$RunAiCanonicalFlow
            program = $AiFlowProgramPath
            out_dir = $AiFlowOutDir
            benchmark_iterations = $AiFlowBenchmarkIterations
            skip_benchmark_gate = [bool]$AiFlowSkipBenchmarkGate
        }
        steps         = $results
    }
    Write-Host ""
    $summaryJson = $summary | ConvertTo-Json -Depth 6
    $summaryJson

    if ($JsonOut) {
        $jsonOutPath = $JsonOut
        if (-not [System.IO.Path]::IsPathRooted($jsonOutPath)) {
            $jsonOutPath = Join-Path $root $jsonOutPath
        }
        $jsonOutDir = Split-Path -Parent $jsonOutPath
        if ($jsonOutDir) {
            New-Item -ItemType Directory -Path $jsonOutDir -Force | Out-Null
        }
        Set-Content -Path $jsonOutPath -Value $summaryJson -Encoding UTF8
        Write-Host "Wrote JSON summary to $jsonOutPath" -ForegroundColor Green
    }
}

if ($success) {
    exit 0
}
exit 1
