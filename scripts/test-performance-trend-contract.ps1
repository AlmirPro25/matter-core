param(
    [string]$TrendJson = "target\performance\performance-trend-report.json"
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

function Assert-PositiveMetric {
    param(
        [object]$Value,
        [string]$Message
    )
    Assert-True ($null -ne $Value) $Message
    Assert-True ([double]$Value -gt 0) $Message
}

$trendPath = if ([System.IO.Path]::IsPathRooted($TrendJson)) { $TrendJson } else { Join-Path $repoRoot $TrendJson }
Assert-True (Test-Path $trendPath -PathType Leaf) "Performance trend JSON not found: $trendPath"

$payload = Get-Content -Path $trendPath -Raw | ConvertFrom-Json
$schemaRef = $payload.PSObject.Properties['$schema'].Value

Assert-True ($schemaRef -eq "schemas/performance-trend-report.schema.json") "Unexpected performance trend schema reference"
Assert-True ($payload.schema_version -eq 1) "Unexpected performance trend schema version"
Assert-True ($payload.kind -eq "performance_trend_report") "Unexpected performance trend kind"
Assert-True (@("pass", "warn", "fail") -contains [string]$payload.status) "Performance trend status must be pass/warn/fail"
Assert-True ($payload.total_samples -ge 1) "Performance trend total_samples must be >= 1"
Assert-True ($payload.window_samples -ge 1) "Performance trend window_samples must be >= 1"
Assert-True ($payload.summary.benchmark_count -eq $payload.benchmarks.Count) "Benchmark count mismatch"
Assert-True ($payload.summary.startup_command_count -eq $payload.startup.Count) "Startup count mismatch"
Assert-PositiveMetric $payload.summary.max_benchmark_p95_ms "max_benchmark_p95_ms must be > 0"
Assert-PositiveMetric $payload.summary.max_startup_p95_ms "max_startup_p95_ms must be > 0"

foreach ($benchmark in @($payload.benchmarks)) {
    Assert-True (@("pass", "warn", "fail") -contains [string]$benchmark.status) "Invalid benchmark status for $($benchmark.name)"
    Assert-PositiveMetric $benchmark.stats.latest_ms "Invalid latest_ms for $($benchmark.name)"
    Assert-PositiveMetric $benchmark.stats.median_ms "Invalid median_ms for $($benchmark.name)"
    Assert-PositiveMetric $benchmark.stats.p95_ms "Invalid p95_ms for $($benchmark.name)"
    Assert-True ($benchmark.stats.samples -ge 1) "Invalid sample count for $($benchmark.name)"
}

foreach ($startup in @($payload.startup)) {
    Assert-True (@("pass", "warn", "fail") -contains [string]$startup.status) "Invalid startup status for $($startup.name)"
    Assert-PositiveMetric $startup.stats.latest_ms "Invalid startup latest_ms for $($startup.name)"
    Assert-PositiveMetric $startup.stats.median_ms "Invalid startup median_ms for $($startup.name)"
    Assert-PositiveMetric $startup.stats.p95_ms "Invalid startup p95_ms for $($startup.name)"
    Assert-True ($startup.stats.samples -ge 1) "Invalid startup sample count for $($startup.name)"
}

[ordered]@{
    ok = $true
    schema = $schemaRef
    status = $payload.status
    benchmark_count = $payload.benchmarks.Count
    startup_count = $payload.startup.Count
    trend = $trendPath
} | ConvertTo-Json -Depth 4
