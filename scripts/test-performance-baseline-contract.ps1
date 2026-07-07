param(
    [string]$CliPath,
    [string]$OutDir = "target\performance-contract"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Assert-NumberPositive {
    param(
        [object]$Value,
        [string]$Message
    )

    Assert-True ($null -ne $Value) $Message
    Assert-True ([double]$Value -gt 0) $Message
}

if (-not $CliPath) {
    $releasePath = "F:\Users\almir\Desktop\matter_target\release\matter-cli.exe"
    $debugPath = "F:\Users\almir\Desktop\matter_target\debug\matter-cli.exe"
    if (Test-Path $releasePath -PathType Leaf) {
        $CliPath = $releasePath
    } elseif (Test-Path $debugPath -PathType Leaf) {
        $CliPath = $debugPath
    } else {
        throw "CliPath was not provided and no default matter-cli.exe was found"
    }
}

$outRoot = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $repoRoot $OutDir }
if (Test-Path $outRoot) {
    Remove-Item -LiteralPath $outRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $outRoot -Force | Out-Null

$scriptPath = Join-Path $PSScriptRoot "run-performance-baseline.ps1"
$benchmarks = @(
    "benchmarks\fibonacci_iterative.matter",
    "benchmarks\sum_array.matter"
)

& $scriptPath -CliPath $CliPath -Iterations 3 -StartupIterations 2 -BenchmarkFiles $benchmarks -OutDir $outRoot > $null
if ($LASTEXITCODE -ne 0) {
    throw "run-performance-baseline.ps1 failed"
}

$jsonPath = Join-Path $outRoot "performance-baseline.json"
$mdPath = Join-Path $outRoot "performance-baseline.md"
$historyPath = Join-Path $outRoot "performance-history.ndjson"

if (-not (Test-Path $jsonPath -PathType Leaf)) {
    throw "Missing performance-baseline.json"
}
if (-not (Test-Path $mdPath -PathType Leaf)) {
    throw "Missing performance-baseline.md"
}
if (-not (Test-Path $historyPath -PathType Leaf)) {
    throw "Missing performance-history.ndjson"
}

$report = Get-Content -Path $jsonPath -Raw | ConvertFrom-Json
$schemaRef = $report.PSObject.Properties['$schema'].Value
Assert-True ($schemaRef -eq "schemas/performance-baseline.schema.json") "Unexpected performance schema reference"
Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "Performance schema reference must be repo-relative"
Assert-True (Test-Path (Join-Path $repoRoot $schemaRef) -PathType Leaf) "Performance schema file is missing"
Assert-True ($report.schema_version -eq 1) "Unexpected performance schema version"
Assert-True ($report.kind -eq "performance_baseline") "Unexpected performance artifact kind"
Assert-True ($report.ok -eq $true) "Performance baseline must report ok=true"
if ($report.benchmarks.Count -ne 2) {
    throw "Expected 2 benchmarks, got $($report.benchmarks.Count)"
}
if ($report.startup.Count -lt 3) {
    throw "Expected startup command timings"
}
foreach ($benchmark in $report.benchmarks) {
    Assert-True ($benchmark.status -eq "ok") "Benchmark status must be ok for $($benchmark.name)"
    Assert-NumberPositive $benchmark.median_ns "Invalid benchmark median_ns for $($benchmark.name)"
    Assert-NumberPositive $benchmark.p95_ns "Invalid benchmark p95_ns for $($benchmark.name)"
    Assert-NumberPositive $benchmark.median_ms "Invalid benchmark median_ms for $($benchmark.name)"
    Assert-NumberPositive $benchmark.p95_ms "Invalid benchmark p95_ms for $($benchmark.name)"
}
foreach ($startup in $report.startup) {
    Assert-NumberPositive $startup.median_ms "Invalid startup median_ms for $($startup.name)"
    Assert-True ($startup.runs_ms.Count -eq $startup.iterations) "Startup runs_ms count mismatch for $($startup.name)"
}
Assert-True (@($report.startup.name) -contains "frontier-sim-quality-json") "Missing frontier-sim-quality-json startup timing"
if (-not ($report.cli.sha256 -match "^[a-f0-9]{64}$")) {
    throw "Invalid CLI sha256"
}
Assert-True ($report.summary.benchmark_count -eq $report.benchmarks.Count) "Summary benchmark count mismatch"
Assert-True ($report.summary.startup_command_count -eq $report.startup.Count) "Summary startup count mismatch"

$driftOut = Join-Path $outRoot "drift"
& $scriptPath -CliPath $CliPath -Iterations 2 -StartupIterations 1 -BenchmarkFiles $benchmarks -OutDir $driftOut -BaselineJson $jsonPath -DriftTolerancePercent 100000 -EnforceDrift > $null
if ($LASTEXITCODE -ne 0) {
    throw "Self-baseline drift gate should pass with large tolerance"
}

[ordered]@{
    ok = $true
    schema = $schemaRef
    benchmark_count = $report.benchmarks.Count
    startup_count = $report.startup.Count
    out_dir = $outRoot
} | ConvertTo-Json -Depth 4
