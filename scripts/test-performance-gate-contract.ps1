param(
    [string]$GatePath = "scripts\run-performance-gate.ps1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not (Test-Path $GatePath -PathType Leaf)) {
    throw "Performance gate script not found: $GatePath"
}

$gate = Get-Content $GatePath -Raw

foreach ($requiredText in @(
    "run-performance-baseline.ps1",
    "test-performance-baseline-contract.ps1",
    "export-performance-trend-report.ps1",
    "test-performance-trend-contract.ps1",
    "performance-baseline.json",
    "performance-history.ndjson",
    "performance-trend-report.json",
    "EnforceDrift",
    "DriftTolerancePercent",
    "WarnBenchmarkP95Ms",
    "FailStartupP95Ms",
    "if (-not `$summary.ok)",
    "exit 2"
)) {
    if (-not $gate.Contains($requiredText)) {
        throw "Performance gate missing required content: $requiredText"
    }
}

$orderedMarkers = @(
    "run performance baseline",
    "test performance baseline contract",
    "export performance trend report",
    "test performance trend contract"
)

$lastIndex = -1
foreach ($marker in $orderedMarkers) {
    $index = $gate.IndexOf($marker)
    if ($index -lt 0) {
        throw "Performance gate missing step marker: $marker"
    }
    if ($index -le $lastIndex) {
        throw "Performance gate step order is invalid at marker: $marker"
    }
    $lastIndex = $index
}

[ordered]@{
    ok = $true
    gate = $GatePath
    checked = @(
        "baseline step included",
        "baseline contract included",
        "trend export included",
        "trend contract included",
        "drift options exposed",
        "threshold options exposed",
        "failed summary exits non-zero",
        "step order is valid"
    )
} | ConvertTo-Json -Depth 4
