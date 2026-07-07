param(
    [string]$CliPath,
    [switch]$BuildRelease,
    [int]$Iterations = 30,
    [int]$StartupIterations = 5,
    [string]$OutDir = "target\performance",
    [string]$BaselineJson,
    [switch]$EnforceDrift,
    [double]$DriftTolerancePercent = 20,
    [int]$TrendWindow = 20,
    [double]$WarnBenchmarkP95Ms = 75,
    [double]$FailBenchmarkP95Ms = 150,
    [double]$WarnStartupP95Ms = 150,
    [double]$FailStartupP95Ms = 300,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$results = New-Object System.Collections.ArrayList

function Resolve-RepoPath {
    param([string]$PathValue)
    if (-not $PathValue) {
        return $null
    }
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return $PathValue
    }
    return Join-Path $repoRoot $PathValue
}

function Invoke-GateStep {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    $started = Get-Date
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "Performance gate step failed: $Name (exit code $LASTEXITCODE)"
    }
    $duration = [Math]::Round(((Get-Date) - $started).TotalSeconds, 3)
    [void]$results.Add([ordered]@{
        name = $Name
        ok = $true
        seconds = $duration
    })
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

if ($Iterations -lt 1) {
    throw "Iterations must be >= 1"
}
if ($StartupIterations -lt 1) {
    throw "StartupIterations must be >= 1"
}
if ($TrendWindow -lt 1) {
    throw "TrendWindow must be >= 1"
}

if (-not $CliPath) {
    $CliPath = Get-DefaultCliPath
}
if (-not $CliPath -and -not $BuildRelease) {
    throw "CliPath was not provided and no default matter-cli.exe was found"
}

$outRoot = Resolve-RepoPath $OutDir
New-Item -ItemType Directory -Path $outRoot -Force | Out-Null

$currentBaselineJson = Join-Path $outRoot "performance-baseline.json"
$trendJson = Join-Path $outRoot "performance-trend-report.json"
$trendMd = Join-Path $outRoot "performance-trend-report.md"
$historyJsonl = Join-Path $outRoot "performance-history.ndjson"

$effectiveBaselineJson = $BaselineJson
if ($BaselineJson) {
    $resolvedBaseline = Resolve-RepoPath $BaselineJson
    if ((Test-Path $resolvedBaseline -PathType Leaf) -and (([System.IO.Path]::GetFullPath($resolvedBaseline)) -eq ([System.IO.Path]::GetFullPath($currentBaselineJson)))) {
        $snapshot = Join-Path $outRoot "performance-baseline.previous.json"
        Copy-Item -LiteralPath $resolvedBaseline -Destination $snapshot -Force
        $effectiveBaselineJson = $snapshot
    }
}

Invoke-GateStep "run performance baseline" {
    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\run-performance-baseline.ps1",
        "-Iterations", "$Iterations",
        "-StartupIterations", "$StartupIterations",
        "-OutDir", $outRoot
    )
    if ($BuildRelease) {
        $args += "-BuildRelease"
    }
    if ($CliPath) {
        $args += @("-CliPath", $CliPath)
    }
    if ($effectiveBaselineJson) {
        $args += @("-BaselineJson", $effectiveBaselineJson, "-DriftTolerancePercent", "$DriftTolerancePercent")
    }
    if ($EnforceDrift) {
        $args += "-EnforceDrift"
    }
    & powershell @args > $null
}

Invoke-GateStep "test performance baseline contract" {
    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\test-performance-baseline-contract.ps1",
        "-OutDir", (Join-Path $outRoot "contract-smoke")
    )
    if ($CliPath) {
        $args += @("-CliPath", $CliPath)
    }
    & powershell @args > $null
}

Invoke-GateStep "export performance trend report" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-performance-trend-report.ps1" `
        -HistoryJsonl $historyJsonl `
        -JsonOut $trendJson `
        -MdOut $trendMd `
        -Window $TrendWindow `
        -WarnBenchmarkP95Ms $WarnBenchmarkP95Ms `
        -FailBenchmarkP95Ms $FailBenchmarkP95Ms `
        -WarnStartupP95Ms $WarnStartupP95Ms `
        -FailStartupP95Ms $FailStartupP95Ms > $null
}

Invoke-GateStep "test performance trend contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-performance-trend-contract.ps1" `
        -TrendJson $trendJson > $null
}

$baseline = Get-Content -Path $currentBaselineJson -Raw | ConvertFrom-Json
$trend = Get-Content -Path $trendJson -Raw | ConvertFrom-Json

$summary = [ordered]@{
    ok = ($baseline.ok -and ($trend.status -ne "fail"))
    status = [string]$trend.status
    generated_at = (Get-Date).ToString("o")
    artifacts = [ordered]@{
        baseline_json = $currentBaselineJson
        baseline_md = (Join-Path $outRoot "performance-baseline.md")
        history = $historyJsonl
        trend_json = $trendJson
        trend_md = $trendMd
    }
    baseline = [ordered]@{
        benchmark_count = [int]$baseline.summary.benchmark_count
        startup_command_count = [int]$baseline.summary.startup_command_count
        slowest_benchmark = $baseline.summary.slowest_benchmark
    }
    trend = [ordered]@{
        total_samples = [int]$trend.total_samples
        window_samples = [int]$trend.window_samples
        max_benchmark_p95_ms = [double]$trend.summary.max_benchmark_p95_ms
        max_startup_p95_ms = [double]$trend.summary.max_startup_p95_ms
    }
    steps = @($results.ToArray())
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 8
}
else {
    Write-Host ""
    if ($summary.ok) {
        Write-Host "Matter Core performance gate passed." -ForegroundColor Green
    } else {
        Write-Host "Matter Core performance gate failed." -ForegroundColor Red
    }
    Write-Host ("Status: {0}" -f $summary.status)
    Write-Host ("Benchmarks: {0}" -f $summary.baseline.benchmark_count)
    Write-Host ("Startup commands: {0}" -f $summary.baseline.startup_command_count)
}

if (-not $summary.ok) {
    exit 2
}
