param(
    [string]$CliPath,
    [string]$Version = "0.1.0-beta",
    [switch]$SkipReleaseBuild,
    [switch]$SkipCargoTests,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$results = New-Object System.Collections.Generic.List[object]

function Invoke-GateStep {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    $started = Get-Date
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "Beta gate step failed: $Name (exit code $LASTEXITCODE)"
    }

    $duration = [math]::Round(((Get-Date) - $started).TotalSeconds, 3)
    $results.Add([ordered]@{
        name = $Name
        ok = $true
        seconds = $duration
    }) | Out-Null
}

if (-not $SkipReleaseBuild) {
    Invoke-GateStep "build release package" {
        $args = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\build-release-package.ps1")
        if ($CliPath) {
            $args += @("-CliPath", $CliPath)
        }
        & powershell @args
    }
}

Invoke-GateStep "build windows setup exe" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\build-windows-setup-exe.ps1"
}

Invoke-GateStep "build download site" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\build-download-site.ps1" -Version $Version -Channel "beta"
}

Invoke-GateStep "export beta release notes" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-beta-release-notes.ps1" -Tag ("v{0}" -f $Version)
}

Invoke-GateStep "test release package contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-package-contract.ps1"
}

Invoke-GateStep "test status triad contract" {
    $args = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\test-status-triad-contract.ps1")
    if ($CliPath) {
        $args += @("-CliPath", $CliPath)
    }
    $args += @(
        "-OutJson", "target\validation\status-triad-beta-latest.json",
        "-HistoryJsonl", "target\validation\status-triad-beta-history.ndjson"
    )
    if ($env:MATTER_STATUS_TRIAD_ENFORCE -eq "1") {
        $args += "-EnforceLatencyBudget"
        if ($env:MATTER_STATUS_TRIAD_MAX_CORE_MS) {
            $args += @("-MaxCoreMs", $env:MATTER_STATUS_TRIAD_MAX_CORE_MS)
        }
        if ($env:MATTER_STATUS_TRIAD_MAX_WORLD_MS) {
            $args += @("-MaxWorldMs", $env:MATTER_STATUS_TRIAD_MAX_WORLD_MS)
        }
        if ($env:MATTER_STATUS_TRIAD_MAX_FRONTIER_MS) {
            $args += @("-MaxFrontierMs", $env:MATTER_STATUS_TRIAD_MAX_FRONTIER_MS)
        }
        if ($env:MATTER_STATUS_TRIAD_DRIFT_TOLERANCE_PERCENT) {
            $args += @("-EnforceLatencyDrift", "-DriftTolerancePercent", $env:MATTER_STATUS_TRIAD_DRIFT_TOLERANCE_PERCENT)
        }
    }
    & powershell @args
}

Invoke-GateStep "test status triad history contract" {
    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\test-status-triad-history-contract.ps1",
        "-HistoryJsonl", "target\validation\status-triad-beta-history.ndjson",
        "-MinSamples", "1"
    )
    if ($env:MATTER_STATUS_TRIAD_HISTORY_MAX_MEDIAN_MS) {
        $args += @("-MaxMedianMs", $env:MATTER_STATUS_TRIAD_HISTORY_MAX_MEDIAN_MS)
    }
    if ($env:MATTER_STATUS_TRIAD_HISTORY_MAX_P95_MS) {
        $args += @("-MaxP95Ms", $env:MATTER_STATUS_TRIAD_HISTORY_MAX_P95_MS)
    }
    if ($env:MATTER_STATUS_TRIAD_HISTORY_MAX_REGRESSION_PERCENT) {
        $args += @("-EnforceRegression", "-MaxRegressionPercent", $env:MATTER_STATUS_TRIAD_HISTORY_MAX_REGRESSION_PERCENT)
    }
    & powershell @args
}

Invoke-GateStep "export status triad trend report" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-status-triad-trend-report.ps1" `
        -HistoryJsonl "target\validation\status-triad-beta-history.ndjson" `
        -JsonOut "target\validation\status-triad-beta-trend-report.json" `
        -MdOut "target\validation\status-triad-beta-trend-report.md"
}

Invoke-GateStep "export status triad health" {
    $warn = if ($env:MATTER_STATUS_TRIAD_HEALTH_WARN_P95_MS) { $env:MATTER_STATUS_TRIAD_HEALTH_WARN_P95_MS } else { "30000" }
    $fail = if ($env:MATTER_STATUS_TRIAD_HEALTH_FAIL_P95_MS) { $env:MATTER_STATUS_TRIAD_HEALTH_FAIL_P95_MS } else { "45000" }
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-status-triad-health.ps1" `
        -TriadLatestJson "target\validation\status-triad-beta-latest.json" `
        -TriadTrendJson "target\validation\status-triad-beta-trend-report.json" `
        -HealthOut "target\validation\status-triad-beta-health.json" `
        -WarnP95Ms $warn `
        -FailP95Ms $fail
}

Invoke-GateStep "test status triad health contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-status-triad-health-contract.ps1" `
        -HealthJson "target\validation\status-triad-beta-health.json"
}

Invoke-GateStep "test frontier simulation quality contract" {
    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\test-frontier-simulation-quality-contract.ps1"
    )
    if ($CliPath) {
        $args += @("-CliPath", $CliPath)
    }
    & powershell @args
}

Invoke-GateStep "run performance gate" {
    $iterations = if ($env:MATTER_PERFORMANCE_ITERATIONS) { $env:MATTER_PERFORMANCE_ITERATIONS } else { "30" }
    $startupIterations = if ($env:MATTER_PERFORMANCE_STARTUP_ITERATIONS) { $env:MATTER_PERFORMANCE_STARTUP_ITERATIONS } else { "5" }
    $trendWindow = if ($env:MATTER_PERFORMANCE_TREND_WINDOW) { $env:MATTER_PERFORMANCE_TREND_WINDOW } else { "20" }
    $warnBenchmarkP95 = if ($env:MATTER_PERFORMANCE_WARN_BENCHMARK_P95_MS) { $env:MATTER_PERFORMANCE_WARN_BENCHMARK_P95_MS } else { "75" }
    $failBenchmarkP95 = if ($env:MATTER_PERFORMANCE_FAIL_BENCHMARK_P95_MS) { $env:MATTER_PERFORMANCE_FAIL_BENCHMARK_P95_MS } else { "150" }
    $warnStartupP95 = if ($env:MATTER_PERFORMANCE_WARN_STARTUP_P95_MS) { $env:MATTER_PERFORMANCE_WARN_STARTUP_P95_MS } else { "150" }
    $failStartupP95 = if ($env:MATTER_PERFORMANCE_FAIL_STARTUP_P95_MS) { $env:MATTER_PERFORMANCE_FAIL_STARTUP_P95_MS } else { "300" }

    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\run-performance-gate.ps1",
        "-Iterations", $iterations,
        "-StartupIterations", $startupIterations,
        "-OutDir", "target\performance-beta",
        "-TrendWindow", $trendWindow,
        "-WarnBenchmarkP95Ms", $warnBenchmarkP95,
        "-FailBenchmarkP95Ms", $failBenchmarkP95,
        "-WarnStartupP95Ms", $warnStartupP95,
        "-FailStartupP95Ms", $failStartupP95
    )
    if ($CliPath) {
        $args += @("-CliPath", $CliPath)
    }
    if ($env:MATTER_PERFORMANCE_BASELINE_JSON) {
        $args += @("-BaselineJson", $env:MATTER_PERFORMANCE_BASELINE_JSON)
    }
    if ($env:MATTER_PERFORMANCE_ENFORCE_DRIFT -eq "1") {
        $args += "-EnforceDrift"
        if ($env:MATTER_PERFORMANCE_DRIFT_TOLERANCE_PERCENT) {
            $args += @("-DriftTolerancePercent", $env:MATTER_PERFORMANCE_DRIFT_TOLERANCE_PERCENT)
        }
    }
    & powershell @args
}

Invoke-GateStep "test performance gate contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-performance-gate-contract.ps1"
}

Invoke-GateStep "test release checksum contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-artifact-checksums-contract.ps1"
}

Invoke-GateStep "test release zip installer contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-zip-installer-contract.ps1"
}

Invoke-GateStep "test windows setup exe contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-windows-setup-exe-contract.ps1"
}

Invoke-GateStep "test download site contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-download-site-contract.ps1"
}

Invoke-GateStep "test beta readiness contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-readiness-contract.ps1"
}

Invoke-GateStep "test beta feedback contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-feedback-contract.ps1"
}

Invoke-GateStep "test beta release notes contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-release-notes-contract.ps1"
}

Invoke-GateStep "test beta site workflow contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-site-workflow-contract.ps1"
}

if (-not $SkipCargoTests) {
    Invoke-GateStep "cargo test" {
        cargo test -q
    }
}

$release = Get-Content "site\release.json" -Raw | ConvertFrom-Json
$zip = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1

$summary = [ordered]@{
    ok = $true
    version = $release.version
    channel = $release.channel
    status = $release.status
    production_ready = [bool]$release.production_ready
    zip = if ($zip) {
        [ordered]@{
            name = $zip.name
            size_bytes = [int64]$zip.size_bytes
            sha256 = [string]$zip.sha256
        }
    } else {
        $null
    }
    steps = @($results.ToArray())
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 8
}
else {
    Write-Host ""
    Write-Host "Matter Core beta gate passed." -ForegroundColor Green
    Write-Host ("Version: {0}" -f $summary.version) -ForegroundColor Green
    Write-Host ("Status: {0}" -f $summary.status) -ForegroundColor Green
    Write-Host ("Zip SHA-256: {0}" -f $summary.zip.sha256) -ForegroundColor Green
}
