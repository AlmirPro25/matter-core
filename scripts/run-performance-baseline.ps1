param(
    [string]$CliPath,
    [switch]$BuildRelease,
    [int]$Iterations = 30,
    [int]$StartupIterations = 5,
    [string[]]$BenchmarkFiles,
    [string]$OutDir = "target\performance",
    [string]$BaselineJson,
    [double]$DriftTolerancePercent = 20,
    [switch]$EnforceDrift,
    [switch]$NoMarkdown,
    [switch]$NoHistory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if ($Iterations -lt 1) {
    throw "Iterations must be >= 1"
}
if ($StartupIterations -lt 1) {
    throw "StartupIterations must be >= 1"
}
if ($DriftTolerancePercent -lt 0) {
    throw "DriftTolerancePercent must be >= 0"
}

function Resolve-RepoPath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $repoRoot $Path
}

function Get-DefaultCliPath {
    $releasePath = "F:\Users\almir\Desktop\matter_target\release\matter-cli.exe"
    $debugPath = "F:\Users\almir\Desktop\matter_target\debug\matter-cli.exe"
    if (Test-Path $releasePath -PathType Leaf) {
        return $releasePath
    }
    if (Test-Path $debugPath -PathType Leaf) {
        return $debugPath
    }
    return $null
}

function Convert-NsToMs {
    param([double]$Ns)
    return [Math]::Round($Ns / 1000000.0, 6)
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$Percentile
    )

    if ($Values.Count -eq 0) {
        return 0
    }
    $sorted = @($Values | Sort-Object)
    $index = [Math]::Ceiling(($Percentile / 100.0) * $sorted.Count) - 1
    if ($index -lt 0) {
        $index = 0
    }
    if ($index -ge $sorted.Count) {
        $index = $sorted.Count - 1
    }
    return [double]$sorted[$index]
}

function Measure-StartupCommand {
    param(
        [string]$Cli,
        [string]$Name,
        [string[]]$CommandArgs,
        [int]$Runs
    )

    $times = New-Object System.Collections.Generic.List[Double]
    for ($i = 0; $i -lt $Runs; $i++) {
        $elapsed = (Measure-Command {
            & $Cli @CommandArgs > $null
        }).TotalMilliseconds
        if ($LASTEXITCODE -ne 0) {
            throw "Startup command failed: $Name"
        }
        $times.Add([double]$elapsed)
    }

    $average = ($times | Measure-Object -Average).Average
    return [ordered]@{
        name = $Name
        args = $CommandArgs
        iterations = $Runs
        median_ms = [Math]::Round((Get-Percentile -Values $times.ToArray() -Percentile 50), 3)
        average_ms = [Math]::Round($average, 3)
        p95_ms = [Math]::Round((Get-Percentile -Values $times.ToArray() -Percentile 95), 3)
        min_ms = [Math]::Round((($times | Measure-Object -Minimum).Minimum), 3)
        max_ms = [Math]::Round((($times | Measure-Object -Maximum).Maximum), 3)
        runs_ms = @($times | ForEach-Object { [Math]::Round($_, 3) })
    }
}

function Invoke-MatterBenchmark {
    param(
        [string]$Cli,
        [string]$BenchmarkPath,
        [int]$Runs
    )

    $output = & $Cli benchmark-json $BenchmarkPath --iterations $Runs
    if ($LASTEXITCODE -ne 0) {
        throw "benchmark-json failed for $BenchmarkPath"
    }
    $json = $output | ConvertFrom-Json
    $stats = $json.bytecode.stats
    $name = Split-Path -Leaf $BenchmarkPath

    return [ordered]@{
        name = $name
        path = $BenchmarkPath
        iterations = $Runs
        status = [string]$json.bytecode.status
        median_ns = [double]$stats.median_ns
        average_ns = [double]$stats.average_ns
        p95_ns = [double]$stats.p95_ns
        min_ns = [double]$stats.min_ns
        max_ns = [double]$stats.max_ns
        median_ms = Convert-NsToMs ([double]$stats.median_ns)
        average_ms = Convert-NsToMs ([double]$stats.average_ns)
        p95_ms = Convert-NsToMs ([double]$stats.p95_ns)
        min_ms = Convert-NsToMs ([double]$stats.min_ns)
        max_ms = Convert-NsToMs ([double]$stats.max_ns)
        native_status = [string]$json.native.status
        native_reason = if ($json.native.PSObject.Properties.Name -contains "reason") { [string]$json.native.reason } else { $null }
    }
}

function Get-BaselineIndex {
    param([object]$Baseline)

    $index = @{}
    foreach ($item in $Baseline.benchmarks) {
        $index[[string]$item.name] = $item
    }
    return $index
}

function Compare-PerformanceDrift {
    param(
        [object[]]$Benchmarks,
        [string]$BaselinePath,
        [double]$TolerancePercent
    )

    if (-not $BaselinePath) {
        return [ordered]@{
            enabled = $false
            enforced = [bool]$EnforceDrift
            tolerance_percent = $TolerancePercent
            baseline_path = $null
            failures = @()
            comparisons = @()
        }
    }

    $resolved = Resolve-RepoPath $BaselinePath
    if (-not (Test-Path $resolved -PathType Leaf)) {
        throw "Baseline JSON not found: $resolved"
    }

    $baseline = Get-Content -Path $resolved -Raw | ConvertFrom-Json
    $baselineIndex = Get-BaselineIndex -Baseline $baseline
    $comparisons = New-Object System.Collections.ArrayList
    $failures = New-Object System.Collections.ArrayList

    foreach ($candidate in $Benchmarks) {
        if (-not $baselineIndex.ContainsKey($candidate.name)) {
            continue
        }
        $base = $baselineIndex[$candidate.name]
        $baselineMedianNs = [double]$base.median_ns
        if ($baselineMedianNs -le 0) {
            continue
        }
        $allowedNs = $baselineMedianNs * (1.0 + ($TolerancePercent / 100.0))
        $deltaPercent = (([double]$candidate.median_ns - $baselineMedianNs) / $baselineMedianNs) * 100.0
        $passed = ([double]$candidate.median_ns -le $allowedNs)
        $comparison = [ordered]@{
            name = $candidate.name
            baseline_median_ms = Convert-NsToMs $baselineMedianNs
            candidate_median_ms = [double]$candidate.median_ms
            delta_percent = [Math]::Round($deltaPercent, 3)
            allowed_median_ms = Convert-NsToMs $allowedNs
            passed = $passed
        }
        [void]$comparisons.Add($comparison)
        if (-not $passed) {
            [void]$failures.Add($comparison)
        }
    }

    return [ordered]@{
        enabled = $true
        enforced = [bool]$EnforceDrift
        tolerance_percent = $TolerancePercent
        baseline_path = [string]$resolved
        failures = @($failures.ToArray())
        comparisons = @($comparisons.ToArray())
    }
}

function Render-MarkdownReport {
    param([object]$Report)

    $lines = New-Object System.Collections.Generic.List[String]
    $lines.Add("# Matter Core Performance Baseline")
    $lines.Add("")
    $lines.Add("- Generated at: $($Report.generated_at)")
    $lines.Add("- CLI: ``" + $Report.cli.path + "``")
    $lines.Add("- CLI SHA-256: ``" + $Report.cli.sha256 + "``")
    $lines.Add("- Benchmark iterations: $($Report.iterations)")
    $lines.Add("- Startup iterations: $($Report.startup_iterations)")
    $lines.Add("")
    $lines.Add("## Bytecode VM Benchmarks")
    $lines.Add("")
    $lines.Add("| Benchmark | Median | Average | P95 | Min | Max | Native |")
    $lines.Add("|---|---:|---:|---:|---:|---:|---|")
    foreach ($b in $Report.benchmarks) {
        $lines.Add("| $($b.name) | $($b.median_ms) ms | $($b.average_ms) ms | $($b.p95_ms) ms | $($b.min_ms) ms | $($b.max_ms) ms | $($b.native_status) |")
    }
    $lines.Add("")
    $lines.Add("## CLI Startup Commands")
    $lines.Add("")
    $lines.Add("| Command | Median | Average | P95 | Min | Max |")
    $lines.Add("|---|---:|---:|---:|---:|---:|")
    foreach ($s in $Report.startup) {
        $lines.Add("| $($s.name) | $($s.median_ms) ms | $($s.average_ms) ms | $($s.p95_ms) ms | $($s.min_ms) ms | $($s.max_ms) ms |")
    }
    $lines.Add("")
    $lines.Add("## Drift")
    $lines.Add("")
    if (-not $Report.drift.enabled) {
        $lines.Add("No baseline comparison was requested.")
    } else {
        $lines.Add("- Baseline: ``" + $Report.drift.baseline_path + "``")
        $lines.Add("- Tolerance: $($Report.drift.tolerance_percent)%")
        $lines.Add("- Failures: $($Report.drift.failures.Count)")
        $lines.Add("")
        $lines.Add("| Benchmark | Baseline Median | Candidate Median | Delta | Allowed | Result |")
        $lines.Add("|---|---:|---:|---:|---:|---|")
        foreach ($c in $Report.drift.comparisons) {
            $result = if ($c.passed) { "pass" } else { "fail" }
            $lines.Add("| $($c.name) | $($c.baseline_median_ms) ms | $($c.candidate_median_ms) ms | $($c.delta_percent)% | $($c.allowed_median_ms) ms | $result |")
        }
    }
    $lines.Add("")
    $lines.Add("## Summary")
    $lines.Add("")
    $lines.Add("- Slowest benchmark by median: $($Report.summary.slowest_benchmark.name) ($($Report.summary.slowest_benchmark.median_ms) ms)")
    $lines.Add("- Highest P95 benchmark: $($Report.summary.highest_p95_benchmark.name) ($($Report.summary.highest_p95_benchmark.p95_ms) ms)")
    $lines.Add("- Native benchmark enabled count: $($Report.summary.native_enabled_count)")
    return ($lines -join "`n")
}

if ($BuildRelease) {
    cargo build -p matter-cli --release
    if ($LASTEXITCODE -ne 0) {
        throw "Release build failed"
    }
}

if (-not $CliPath) {
    $CliPath = Get-DefaultCliPath
}
if (-not $CliPath) {
    throw "CliPath was not provided and no default matter-cli.exe was found"
}
$CliPath = Resolve-RepoPath $CliPath
if (-not (Test-Path $CliPath -PathType Leaf)) {
    throw "CLI executable not found: $CliPath"
}

if (-not $BenchmarkFiles -or $BenchmarkFiles.Count -eq 0) {
    $BenchmarkFiles = @(
        "benchmarks\fibonacci_iterative.matter",
        "benchmarks\sum_array.matter",
        "benchmarks\data_structures.matter",
        "benchmarks\nested_loops.matter",
        "benchmarks\function_calls.matter",
        "benchmarks\loop_intensive.matter",
        "benchmarks\fibonacci.matter",
        "benchmarks\backend_calls.matter",
        "benchmarks\stress_test.matter"
    )
}

$outRoot = Resolve-RepoPath $OutDir
New-Item -ItemType Directory -Path $outRoot -Force | Out-Null

$benchmarkResults = New-Object System.Collections.Generic.List[Object]
foreach ($file in $BenchmarkFiles) {
    $path = Resolve-RepoPath $file
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Benchmark file not found: $path"
    }
    Write-Host "Benchmarking $file ($Iterations iterations)" -ForegroundColor Cyan
    $benchmarkResults.Add((Invoke-MatterBenchmark -Cli $CliPath -BenchmarkPath $file -Runs $Iterations))
}

$startupCommands = @(
    @{ name = "run first_run"; args = @("run", "examples\first_run.matter") },
    @{ name = "check first_run"; args = @("check", "examples\first_run.matter") },
    @{ name = "reflect-json first_run"; args = @("reflect-json", "examples\first_run.matter") },
    @{ name = "core-status-json"; args = @("core-status-json") },
    @{ name = "world-status-json"; args = @("world-status-json") },
    @{ name = "frontier-status-json"; args = @("frontier-status-json") },
    @{ name = "frontier-sim-quality-json"; args = @("frontier-sim-quality-json") }
)

$startupResults = New-Object System.Collections.Generic.List[Object]
foreach ($command in $startupCommands) {
    Write-Host "Measuring startup: $($command["name"]) ($StartupIterations iterations)" -ForegroundColor Cyan
    $startupResults.Add((Measure-StartupCommand -Cli $CliPath -Name $command["name"] -CommandArgs $command["args"] -Runs $StartupIterations))
}

$benchmarkArray = @($benchmarkResults.ToArray())
$startupArray = @($startupResults.ToArray())
$drift = Compare-PerformanceDrift -Benchmarks $benchmarkArray -BaselinePath $BaselineJson -TolerancePercent $DriftTolerancePercent
$slowest = @($benchmarkArray | Sort-Object -Property median_ns -Descending | Select-Object -First 1)[0]
$highestP95 = @($benchmarkArray | Sort-Object -Property p95_ns -Descending | Select-Object -First 1)[0]
$nativeEnabled = @($benchmarkArray | Where-Object { $_.native_status -eq "ok" }).Count

$report = [ordered]@{
    '$schema' = "schemas/performance-baseline.schema.json"
    schema_version = 1
    kind = "performance_baseline"
    ok = (-not $drift.enabled) -or ($drift.failures.Count -eq 0) -or (-not $EnforceDrift)
    generated_at = (Get-Date).ToString("o")
    iterations = $Iterations
    startup_iterations = $StartupIterations
    cli = [ordered]@{
        path = [string]$CliPath
        sha256 = (Get-FileHash -Algorithm SHA256 -Path $CliPath).Hash.ToLowerInvariant()
        size_bytes = (Get-Item $CliPath).Length
    }
    benchmarks = $benchmarkArray
    startup = $startupArray
    drift = $drift
    summary = [ordered]@{
        benchmark_count = $benchmarkArray.Count
        startup_command_count = $startupArray.Count
        native_enabled_count = $nativeEnabled
        slowest_benchmark = [ordered]@{
            name = $slowest.name
            median_ms = $slowest.median_ms
        }
        highest_p95_benchmark = [ordered]@{
            name = $highestP95.name
            p95_ms = $highestP95.p95_ms
        }
    }
}

$jsonPath = Join-Path $outRoot "performance-baseline.json"
$json = $report | ConvertTo-Json -Depth 8
Set-Content -Path $jsonPath -Value $json -Encoding UTF8

if (-not $NoMarkdown) {
    $mdPath = Join-Path $outRoot "performance-baseline.md"
    Set-Content -Path $mdPath -Value (Render-MarkdownReport -Report $report) -Encoding UTF8
}

if (-not $NoHistory) {
    $historyPath = Join-Path $outRoot "performance-history.ndjson"
    Add-Content -Path $historyPath -Value ($report | ConvertTo-Json -Depth 8 -Compress) -Encoding UTF8
}

if ($drift.enabled -and $drift.failures.Count -gt 0 -and $EnforceDrift) {
    Write-Error "Performance drift gate failed with $($drift.failures.Count) failure(s). See $jsonPath"
    exit 2
}

$json
