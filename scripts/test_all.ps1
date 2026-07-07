param(
    [switch]$SkipRunnableExamples,
    [switch]$SkipRustFfiSmoke,
    [switch]$SkipNativeFfiSmoke,
    [switch]$IncludeJavaNativeSmoke,
    [switch]$IncludeNodeNativeUnitTests,
    [switch]$StrictEnv,
    [switch]$CiMode,
    [switch]$RunAiCanonicalFlow,
    [string]$AiFlowProgramPath = "examples\\first_run.matter",
    [int]$AiFlowBenchmarkIterations = 20,
    [switch]$AiFlowSkipBenchmarkGate
)

# Test: Full Matter Core validation
Write-Host "=== Matter Core: Validacao Completa ===" -ForegroundColor Cyan
Write-Host ""

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$allPassed = $true
$releaseCliPath = $null

function Test-CommandExists([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-JavaHomeReady {
    if (-not $env:JAVA_HOME) { return $false }
    $javaExe = Join-Path $env:JAVA_HOME "bin\java.exe"
    return Test-Path $javaExe
}

function Run-Step($name, [scriptblock]$command) {
    Write-Host "==> $name" -ForegroundColor White
    & $command
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        Write-Host "FALHOU: $name (exit code $exitCode)" -ForegroundColor Red
        $script:allPassed = $false
    }
    else {
        Write-Host "PASSOU: $name" -ForegroundColor Green
    }

    Write-Host ""
}

function Get-ReleaseCliPath {
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    $exeName = if ($IsWindows -or $env:OS -eq "Windows_NT") { "matter-cli.exe" } else { "matter-cli" }
    return Join-Path (Join-Path $metadata.target_directory "release") $exeName
}

Write-Host "Preflight ambiente:" -ForegroundColor DarkCyan
$hasGo = Test-CommandExists "go"
$hasNode = Test-CommandExists "node"
$javaHomeReady = Test-JavaHomeReady
Write-Host "  go:        $hasGo"
Write-Host "  node:      $hasNode"
Write-Host "  JAVA_HOME: $javaHomeReady"
Write-Host ""

if ($StrictEnv) {
    if (-not $hasGo) {
        Write-Host "FALHOU: StrictEnv exige Go instalado para fluxos nativos." -ForegroundColor Red
        exit 2
    }
    if ($IncludeJavaNativeSmoke -and -not $javaHomeReady) {
        Write-Host "FALHOU: StrictEnv exige JAVA_HOME válido com -IncludeJavaNativeSmoke." -ForegroundColor Red
        exit 2
    }
}

Run-Step "cargo test --workspace" {
    if ($IncludeNodeNativeUnitTests) {
        if ($CiMode) {
            cargo test --workspace -q
        }
        else {
            cargo test --workspace
        }
    }
    else {
        if ($CiMode) {
            cargo test --workspace --exclude matter-bridge-nodejs-native -q
        }
        else {
            cargo test --workspace --exclude matter-bridge-nodejs-native
        }
    }
}

Run-Step "cargo build --release --workspace" {
    if ($CiMode) {
        cargo build --release --workspace -q
    }
    else {
        cargo build --release --workspace
    }
}

$releaseCliPath = Get-ReleaseCliPath
if (-not (Test-Path $releaseCliPath)) {
    Write-Host "FALHOU: CLI release nao encontrada em $releaseCliPath" -ForegroundColor Red
    exit 1
}
Write-Host "CLI release: $releaseCliPath" -ForegroundColor DarkCyan
Write-Host ""

Run-Step "bytecode equivalence" {
    powershell -ExecutionPolicy Bypass -File .\scripts\test_bytecode_equivalence.ps1 -CliPath $releaseCliPath
}

Run-Step "api bridge json contract" {
    powershell -ExecutionPolicy Bypass -File .\scripts\test_api_bridge.ps1 -CliPath $releaseCliPath
}

Run-Step "status triad contract" {
    powershell -ExecutionPolicy Bypass -File .\scripts\test-status-triad-contract.ps1 -CliPath $releaseCliPath
}

if ($RunAiCanonicalFlow) {
    Run-Step "ai app canonical flow" {
        $args = @(
            "-ExecutionPolicy", "Bypass",
            "-File", ".\\scripts\\ai-app-canonical-flow.ps1",
            "-ProgramPath", $AiFlowProgramPath,
            "-BenchmarkIterations", "$AiFlowBenchmarkIterations",
            "-CliPath", $releaseCliPath
        )
        if ($AiFlowSkipBenchmarkGate) {
            $args += "-SkipBenchmarkGate"
        }
        powershell @args
    }
}
else {
    Write-Host "PULADO: ai app canonical flow" -ForegroundColor Yellow
    Write-Host ""
}

if (-not $SkipRunnableExamples) {
    Run-Step "runnable examples contract" {
        powershell -ExecutionPolicy Bypass -File .\scripts\test-runnable-examples.ps1 -CliPath $releaseCliPath -JsonSummary
    }
}
else {
    Write-Host "PULADO: runnable examples contract" -ForegroundColor Yellow
    Write-Host ""
}

if (-not $SkipRustFfiSmoke) {
    Run-Step "rust ffi plugin smoke" {
        powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1 -JsonOut target\ffi\rust-smoke.json
    }
}
else {
    Write-Host "PULADO: rust ffi plugin smoke" -ForegroundColor Yellow
    Write-Host ""
}

if (-not $SkipNativeFfiSmoke) {
    Run-Step "native ffi smoke" {
        if (-not $hasGo) {
            throw "Go nao encontrado no PATH. Use -SkipNativeFfiSmoke ou instale Go."
        }
        if ($IncludeJavaNativeSmoke) {
            if (-not $javaHomeReady) {
                throw "JAVA_HOME invalido/ausente para smoke Java nativo. Configure JAVA_HOME ou remova -IncludeJavaNativeSmoke."
            }
            powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -IncludeJava -JsonOut target\ffi\native-smoke.json
        }
        else {
            powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -JsonOut target\ffi\native-smoke.json
        }
    }
}
else {
    Write-Host "PULADO: native ffi smoke" -ForegroundColor Yellow
    Write-Host ""
}

if ((-not $SkipRustFfiSmoke) -and (-not $SkipNativeFfiSmoke)) {
    Run-Step "verify ffi smoke summaries" {
        if ($IncludeJavaNativeSmoke) {
            powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1 -RequireJava
        }
        else {
            powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1
        }
    }
    Run-Step "export ffi validation matrix" {
        powershell -ExecutionPolicy Bypass -File .\scripts\export-ffi-validation-matrix.ps1 -Out target\ffi\ffi-validation-matrix.json
    }
    Run-Step "verify ffi validation matrix" {
        if ($IncludeJavaNativeSmoke) {
            powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix -RequireJava
        }
        else {
            powershell -ExecutionPolicy Bypass -File .\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix
        }
    }
    Run-Step "export ffi validation report" {
        powershell -ExecutionPolicy Bypass -File .\scripts\export-ffi-validation-report.ps1 -Out target\ffi\ffi-validation-report.md
    }
    Run-Step "test ffi validation report contract" {
        powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-report-contract.ps1
    }
    Run-Step "test ffi validation matrix contract" {
        powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-matrix-contract.ps1
    }
    Run-Step "test release package contract" {
        powershell -ExecutionPolicy Bypass -File .\scripts\test-release-package-contract.ps1
    }
}
else {
    Write-Host "PULADO: verify ffi smoke summaries" -ForegroundColor Yellow
    Write-Host ""
}

if ($allPassed) {
    Write-Host "=== Validacao completa passou ===" -ForegroundColor Green
    exit 0
}
else {
    Write-Host "=== Validacao completa falhou ===" -ForegroundColor Red
    exit 1
}
