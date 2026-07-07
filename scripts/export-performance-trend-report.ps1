param(
    [string]$HistoryJsonl = "target\performance\performance-history.ndjson",
    [string]$JsonOut = "target\performance\performance-trend-report.json",
    [string]$MdOut = "target\performance\performance-trend-report.md",
    [int]$Window = 20,
    [double]$WarnBenchmarkP95Ms = 75,
    [double]$FailBenchmarkP95Ms = 150,
    [double]$WarnStartupP95Ms = 150,
    [double]$FailStartupP95Ms = 300
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Resolve-RepoPath {
    param([string]$PathValue)
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return $PathValue
    }
    return Join-Path $repoRoot $PathValue
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) {
        throw "Cannot compute percentile on empty set"
    }
    if ($sorted.Count -eq 1) {
        return [double]$sorted[0]
    }

    $rank = ($Percentile / 100.0) * ($sorted.Count - 1)
    $low = [int][Math]::Floor($rank)
    $high = [int][Math]::Ceiling($rank)
    if ($low -eq $high) {
        return [double]$sorted[$low]
    }
    $weight = $rank - $low
    return [double]($sorted[$low] + (($sorted[$high] - $sorted[$low]) * $weight))
}

function Get-Stats {
    param([double[]]$Values)

    $latest = [double]$Values[-1]
    $median = Get-Percentile -Values $Values -Percentile 50
    $p95 = Get-Percentile -Values $Values -Percentile 95
    return [ordered]@{
        samples = $Values.Count
        latest_ms = [Math]::Round($latest, 6)
        median_ms = [Math]::Round($median, 6)
        p95_ms = [Math]::Round($p95, 6)
        min_ms = [Math]::Round((($Values | Measure-Object -Minimum).Minimum), 6)
        max_ms = [Math]::Round((($Values | Measure-Object -Maximum).Maximum), 6)
    }
}

function Parse-HistoryEntries {
    param([string]$RawText)

    $entries = New-Object System.Collections.ArrayList
    foreach ($line in ($RawText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        [void]$entries.Add(($line | ConvertFrom-Json))
    }
    return @($entries.ToArray())
}

function Add-Sample {
    param(
        [hashtable]$Map,
        [string]$Name,
        [double]$Value
    )

    if (-not $Map.ContainsKey($Name)) {
        $Map[$Name] = New-Object System.Collections.Generic.List[Double]
    }
    $Map[$Name].Add($Value)
}

function Get-WorstStatus {
    param(
        [double]$Value,
        [double]$Warn,
        [double]$Fail
    )

    if ($Value -gt $Fail) { return "fail" }
    if ($Value -gt $Warn) { return "warn" }
    return "pass"
}

if ($Window -lt 1) {
    throw "Window must be >= 1"
}
if ($WarnBenchmarkP95Ms -le 0 -or $FailBenchmarkP95Ms -le 0 -or $WarnStartupP95Ms -le 0 -or $FailStartupP95Ms -le 0) {
    throw "Thresholds must be > 0"
}
if ($WarnBenchmarkP95Ms -gt $FailBenchmarkP95Ms) {
    throw "WarnBenchmarkP95Ms must be <= FailBenchmarkP95Ms"
}
if ($WarnStartupP95Ms -gt $FailStartupP95Ms) {
    throw "WarnStartupP95Ms must be <= FailStartupP95Ms"
}

$historyPath = Resolve-RepoPath $HistoryJsonl
if (-not (Test-Path $historyPath -PathType Leaf)) {
    throw "Performance history not found: $historyPath"
}

$entries = @(Parse-HistoryEntries -RawText (Get-Content -Path $historyPath -Raw))
if ($entries.Count -eq 0) {
    throw "Performance history has no entries: $historyPath"
}

$windowSize = [Math]::Min($Window, $entries.Count)
$windowEntries = if ($windowSize -eq $entries.Count) { $entries } else { $entries[($entries.Count - $windowSize)..($entries.Count - 1)] }

$benchmarkSamples = @{}
$startupSamples = @{}

foreach ($entry in $windowEntries) {
    foreach ($benchmark in @($entry.benchmarks)) {
        Add-Sample -Map $benchmarkSamples -Name ([string]$benchmark.name) -Value ([double]$benchmark.median_ms)
    }
    foreach ($startup in @($entry.startup)) {
        Add-Sample -Map $startupSamples -Name ([string]$startup.name) -Value ([double]$startup.median_ms)
    }
}

$benchmarkStats = New-Object System.Collections.ArrayList
$maxBenchmarkP95 = 0.0
foreach ($name in @($benchmarkSamples.Keys | Sort-Object)) {
    $stats = Get-Stats -Values $benchmarkSamples[$name].ToArray()
    $status = Get-WorstStatus -Value ([double]$stats.p95_ms) -Warn $WarnBenchmarkP95Ms -Fail $FailBenchmarkP95Ms
    if ([double]$stats.p95_ms -gt $maxBenchmarkP95) {
        $maxBenchmarkP95 = [double]$stats.p95_ms
    }
    [void]$benchmarkStats.Add([ordered]@{
        name = $name
        status = $status
        stats = $stats
    })
}

$startupStats = New-Object System.Collections.ArrayList
$maxStartupP95 = 0.0
foreach ($name in @($startupSamples.Keys | Sort-Object)) {
    $stats = Get-Stats -Values $startupSamples[$name].ToArray()
    $status = Get-WorstStatus -Value ([double]$stats.p95_ms) -Warn $WarnStartupP95Ms -Fail $FailStartupP95Ms
    if ([double]$stats.p95_ms -gt $maxStartupP95) {
        $maxStartupP95 = [double]$stats.p95_ms
    }
    [void]$startupStats.Add([ordered]@{
        name = $name
        status = $status
        stats = $stats
    })
}

$benchmarkHealth = Get-WorstStatus -Value $maxBenchmarkP95 -Warn $WarnBenchmarkP95Ms -Fail $FailBenchmarkP95Ms
$startupHealth = Get-WorstStatus -Value $maxStartupP95 -Warn $WarnStartupP95Ms -Fail $FailStartupP95Ms
$overallStatus = if (@($benchmarkHealth, $startupHealth) -contains "fail") { "fail" } elseif (@($benchmarkHealth, $startupHealth) -contains "warn") { "warn" } else { "pass" }

$report = [ordered]@{
    '$schema' = "schemas/performance-trend-report.schema.json"
    schema_version = 1
    kind = "performance_trend_report"
    ok = ($overallStatus -ne "fail")
    status = $overallStatus
    generated_at = (Get-Date).ToString("o")
    history = $historyPath
    total_samples = $entries.Count
    window_samples = $windowSize
    thresholds = [ordered]@{
        warn_benchmark_p95_ms = $WarnBenchmarkP95Ms
        fail_benchmark_p95_ms = $FailBenchmarkP95Ms
        warn_startup_p95_ms = $WarnStartupP95Ms
        fail_startup_p95_ms = $FailStartupP95Ms
    }
    summary = [ordered]@{
        max_benchmark_p95_ms = [Math]::Round($maxBenchmarkP95, 6)
        max_startup_p95_ms = [Math]::Round($maxStartupP95, 6)
        benchmark_health = $benchmarkHealth
        startup_health = $startupHealth
        benchmark_count = $benchmarkStats.Count
        startup_command_count = $startupStats.Count
    }
    benchmarks = @($benchmarkStats.ToArray())
    startup = @($startupStats.ToArray())
}

$jsonPath = Resolve-RepoPath $JsonOut
$jsonDir = Split-Path -Parent $jsonPath
if ($jsonDir) {
    New-Item -ItemType Directory -Path $jsonDir -Force | Out-Null
}
$report | ConvertTo-Json -Depth 8 | Set-Content -Path $jsonPath -Encoding UTF8

$mdPath = Resolve-RepoPath $MdOut
$mdDir = Split-Path -Parent $mdPath
if ($mdDir) {
    New-Item -ItemType Directory -Path $mdDir -Force | Out-Null
}

$lines = New-Object System.Collections.Generic.List[String]
$lines.Add("# Matter Core Performance Trend Report")
$lines.Add("")
$lines.Add("- Generated: $($report.generated_at)")
$lines.Add("- Status: $($report.status)")
$lines.Add("- History: $historyPath")
$lines.Add("- Total samples: $($report.total_samples)")
$lines.Add("- Window samples: $($report.window_samples)")
$lines.Add("")
$lines.Add("## Benchmark Medians")
$lines.Add("")
$lines.Add("| Benchmark | Status | Latest | Median | P95 | Min | Max | Samples |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|---:|")
foreach ($item in $report.benchmarks) {
    $s = $item.stats
    $lines.Add("| $($item.name) | $($item.status) | $($s.latest_ms) ms | $($s.median_ms) ms | $($s.p95_ms) ms | $($s.min_ms) ms | $($s.max_ms) ms | $($s.samples) |")
}
$lines.Add("")
$lines.Add("## CLI Startup Medians")
$lines.Add("")
$lines.Add("| Command | Status | Latest | Median | P95 | Min | Max | Samples |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|---:|")
foreach ($item in $report.startup) {
    $s = $item.stats
    $lines.Add("| $($item.name) | $($item.status) | $($s.latest_ms) ms | $($s.median_ms) ms | $($s.p95_ms) ms | $($s.min_ms) ms | $($s.max_ms) ms | $($s.samples) |")
}

Set-Content -Path $mdPath -Value ($lines -join "`n") -Encoding UTF8

[ordered]@{
    ok = $true
    status = $overallStatus
    json = $jsonPath
    md = $mdPath
    window_samples = $windowSize
} | ConvertTo-Json -Depth 4
