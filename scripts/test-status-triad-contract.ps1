param(
    [string]$CliPath,
    [string]$CoreStatusPath,
    [string]$WorldStatusPath,
    [string]$FrontierStatusPath,
    [switch]$EnforceLatencyBudget,
    [double]$MaxCoreMs = 60000,
    [double]$MaxWorldMs = 60000,
    [double]$MaxFrontierMs = 60000,
    [switch]$EnforceLatencyDrift,
    [double]$DriftTolerancePercent = 50,
    [string]$DriftBaselineJson,
    [string]$OutJson,
    [string]$HistoryJsonl
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Invoke-StatusContract {
    param(
        [string]$ScriptName,
        [string]$StatusPath,
        [string]$CliPath
    )

    $scriptPath = Join-Path $PSScriptRoot $ScriptName
    if (-not (Test-Path $scriptPath -PathType Leaf)) {
        throw "Status contract script not found: $scriptPath"
    }

    $invocationArgs = @{}
    if ($StatusPath) {
        $invocationArgs["StatusPath"] = $StatusPath
    }
    elseif ($CliPath) {
        $invocationArgs["CliPath"] = $CliPath
    }

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $scriptPath @invocationArgs
    $stopwatch.Stop()
    if ($LASTEXITCODE -ne 0) {
        throw "$ScriptName failed with exit code $LASTEXITCODE"
    }

    return [ordered]@{
        payload = ($output | ConvertFrom-Json)
        elapsed_ms = [Math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
    }
}

function Assert-LatencyBudget {
    param(
        [string]$Name,
        [double]$ActualMs,
        [double]$MaxMs
    )

    if ($ActualMs -gt $MaxMs) {
        throw "Latency budget exceeded for ${Name}: ${ActualMs}ms > ${MaxMs}ms"
    }
}

function Get-DriftBaseline {
    param([string]$BaselinePath)

    if (-not $BaselinePath) {
        return $null
    }

    $resolved = $BaselinePath
    if (-not [System.IO.Path]::IsPathRooted($resolved)) {
        $resolved = Join-Path $repoRoot $resolved
    }
    if (-not (Test-Path $resolved -PathType Leaf)) {
        throw "Drift baseline file not found: $resolved"
    }

    $baseline = Get-Content -Path $resolved -Raw | ConvertFrom-Json
    if ($baseline.core_ms -le 0 -or $baseline.world_ms -le 0 -or $baseline.frontier_ms -le 0) {
        throw "Invalid drift baseline values in $resolved"
    }

    return [ordered]@{
        path = $resolved
        core_ms = [double]$baseline.core_ms
        world_ms = [double]$baseline.world_ms
        frontier_ms = [double]$baseline.frontier_ms
    }
}

function Assert-LatencyDrift {
    param(
        [string]$Name,
        [double]$ObservedMs,
        [double]$BaselineMs,
        [double]$TolerancePercent
    )

    $allowed = $BaselineMs * (1.0 + ($TolerancePercent / 100.0))
    if ($ObservedMs -gt $allowed) {
        throw "Latency drift exceeded for ${Name}: observed=${ObservedMs}ms baseline=${BaselineMs}ms allowed=${allowed}ms"
    }
}

$core = Invoke-StatusContract -ScriptName "test-core-status-contract.ps1" -StatusPath $CoreStatusPath -CliPath $CliPath
$world = Invoke-StatusContract -ScriptName "test-world-status-contract.ps1" -StatusPath $WorldStatusPath -CliPath $CliPath
$frontier = Invoke-StatusContract -ScriptName "test-frontier-status-contract.ps1" -StatusPath $FrontierStatusPath -CliPath $CliPath

if ($EnforceLatencyBudget) {
    Assert-LatencyBudget -Name "core-status-json" -ActualMs $core.elapsed_ms -MaxMs $MaxCoreMs
    Assert-LatencyBudget -Name "world-status-json" -ActualMs $world.elapsed_ms -MaxMs $MaxWorldMs
    Assert-LatencyBudget -Name "frontier-status-json" -ActualMs $frontier.elapsed_ms -MaxMs $MaxFrontierMs
}

$drift = $null
if ($EnforceLatencyDrift) {
    $baselinePath = $DriftBaselineJson
    if (-not $baselinePath) {
        $baselinePath = "scripts\status-triad-latency-baseline.json"
    }
    $drift = Get-DriftBaseline -BaselinePath $baselinePath
    Assert-LatencyDrift -Name "core-status-json" -ObservedMs $core.elapsed_ms -BaselineMs $drift.core_ms -TolerancePercent $DriftTolerancePercent
    Assert-LatencyDrift -Name "world-status-json" -ObservedMs $world.elapsed_ms -BaselineMs $drift.world_ms -TolerancePercent $DriftTolerancePercent
    Assert-LatencyDrift -Name "frontier-status-json" -ObservedMs $frontier.elapsed_ms -BaselineMs $drift.frontier_ms -TolerancePercent $DriftTolerancePercent
}

$result = [ordered]@{
    ok = $true
    generated_at = (Get-Date).ToString("o")
    checked = @(
        "core status contract",
        "world status contract",
        "frontier status contract"
    )
    latency_budget = [ordered]@{
        enforced = [bool]$EnforceLatencyBudget
        max_ms = [ordered]@{
            core = $MaxCoreMs
            world = $MaxWorldMs
            frontier = $MaxFrontierMs
        }
        observed_ms = [ordered]@{
            core = $core.elapsed_ms
            world = $world.elapsed_ms
            frontier = $frontier.elapsed_ms
        }
    }
    latency_drift = [ordered]@{
        enforced = [bool]$EnforceLatencyDrift
        tolerance_percent = $DriftTolerancePercent
        baseline = if ($drift) {
            [ordered]@{
                path = [string]$drift.path
                core_ms = $drift.core_ms
                world_ms = $drift.world_ms
                frontier_ms = $drift.frontier_ms
            }
        } else {
            $null
        }
    }
    schemas = [ordered]@{
        core = [string]$core.payload.schema
        world = [string]$world.payload.schema
        frontier = [string]$frontier.payload.schema
    }
}

$resultJson = $result | ConvertTo-Json -Depth 6

if ($OutJson) {
    $outPath = $OutJson
    if (-not [System.IO.Path]::IsPathRooted($outPath)) {
        $outPath = Join-Path $repoRoot $outPath
    }
    $outDir = Split-Path -Parent $outPath
    if ($outDir) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    Set-Content -Path $outPath -Value $resultJson -Encoding UTF8
}

if ($HistoryJsonl) {
    $historyPath = $HistoryJsonl
    if (-not [System.IO.Path]::IsPathRooted($historyPath)) {
        $historyPath = Join-Path $repoRoot $historyPath
    }
    $historyDir = Split-Path -Parent $historyPath
    if ($historyDir) {
        New-Item -ItemType Directory -Path $historyDir -Force | Out-Null
    }
    $historyLine = $result | ConvertTo-Json -Depth 6 -Compress
    Add-Content -Path $historyPath -Value $historyLine -Encoding UTF8
}

$resultJson
