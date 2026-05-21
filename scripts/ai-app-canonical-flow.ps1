param(
    [string]$ProgramPath = "examples\first_run.matter",
    [string]$OutDir = "target\ai-flow",
    [int]$BenchmarkIterations = 20,
    [long]$MaxMedianNs = 20000000,
    [long]$MaxP95Ns = 50000000,
    [string]$CliPath = "",
    [switch]$SkipBenchmarkGate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function New-DirIfMissing {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Invoke-MatterJsonCommand {
    param(
        [Parameter(Mandatory = $true)][string[]]$Args,
        [Parameter(Mandatory = $true)][string]$OutPath
    )

    if ($CliPath) {
        & $CliPath @Args | Set-Content -LiteralPath $OutPath -Encoding UTF8
    }
    else {
        & cargo run -q -p matter-cli -- @Args | Set-Content -LiteralPath $OutPath -Encoding UTF8
    }
}

$programFullPath = Resolve-Path -LiteralPath $ProgramPath
New-DirIfMissing -Path $OutDir

$timestamp = (Get-Date).ToString("s")
$runJson = Join-Path $OutDir "run.json"
$checkJson = Join-Path $OutDir "check.json"
$reflectJson = Join-Path $OutDir "reflect.json"
$guardJson = Join-Path $OutDir "guard.json"
$perfJson = Join-Path $OutDir "perf.json"
$benchmarkJson = Join-Path $OutDir "benchmark.json"
$gateJson = Join-Path $OutDir "benchmark-gate.json"
$summaryJson = Join-Path $OutDir "summary.json"
$summaryMd = Join-Path $OutDir "summary.md"

Write-Host "[1/7] check-json"
Invoke-MatterJsonCommand -Args @("check-json", $programFullPath) -OutPath $checkJson

Write-Host "[2/7] reflect-json"
Invoke-MatterJsonCommand -Args @("reflect-json", $programFullPath) -OutPath $reflectJson

Write-Host "[3/7] reflexive-guard-json"
Invoke-MatterJsonCommand -Args @("reflexive-guard-json", $programFullPath) -OutPath $guardJson

Write-Host "[4/7] run-json"
Invoke-MatterJsonCommand -Args @("run-json", $programFullPath) -OutPath $runJson

Write-Host "[5/7] perf-diagnose-json"
Invoke-MatterJsonCommand -Args @("perf-diagnose-json", $programFullPath) -OutPath $perfJson

Write-Host "[6/7] benchmark-json"
Invoke-MatterJsonCommand -Args @("benchmark-json", $programFullPath, "--iterations", "$BenchmarkIterations") -OutPath $benchmarkJson

$gateStatus = "skipped"
if (-not $SkipBenchmarkGate) {
    Write-Host "[7/7] benchmark-gate-json"
    Invoke-MatterJsonCommand -Args @(
        "benchmark-gate-json",
        $benchmarkJson,
        "--max-median-ns", "$MaxMedianNs",
        "--max-p95-ns", "$MaxP95Ns"
    ) -OutPath $gateJson
    $gateStatus = "executed"
}

$summary = [ordered]@{
    ok = $true
    generated_at = $timestamp
    program = $programFullPath.Path
    flow = @(
        "check-json",
        "reflect-json",
        "reflexive-guard-json",
        "run-json",
        "perf-diagnose-json",
        "benchmark-json",
        ($(if ($SkipBenchmarkGate) { "benchmark-gate-json (skipped)" } else { "benchmark-gate-json" }))
    )
    artifacts = [ordered]@{
        check = $checkJson
        reflect = $reflectJson
        guard = $guardJson
        run = $runJson
        perf = $perfJson
        benchmark = $benchmarkJson
        benchmark_gate = if ($SkipBenchmarkGate) { "" } else { $gateJson }
    }
    benchmark_gate = [ordered]@{
        status = $gateStatus
        max_median_ns = $MaxMedianNs
        max_p95_ns = $MaxP95Ns
    }
}

$summary | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $summaryJson -Encoding UTF8

$md = @(
    "# AI -> Matter -> App Flow Report",
    "",
    "- Generated at: $timestamp",
    "- Program: $($programFullPath.Path)",
    "- Flow status: ok",
    "",
    "## Steps",
    "1. check-json",
    "2. reflect-json",
    "3. reflexive-guard-json",
    "4. run-json",
    "5. perf-diagnose-json",
    "6. benchmark-json",
    "7. benchmark-gate-json ($gateStatus)",
    "",
    "## Artifacts",
    "- $checkJson",
    "- $reflectJson",
    "- $guardJson",
    "- $runJson",
    "- $perfJson",
    "- $benchmarkJson",
    ($(if (-not $SkipBenchmarkGate) { "- $gateJson" } else { "- benchmark gate skipped" }))
)

$md -join "`r`n" | Set-Content -LiteralPath $summaryMd -Encoding UTF8

Write-Host "Done. Summary: $summaryJson"
