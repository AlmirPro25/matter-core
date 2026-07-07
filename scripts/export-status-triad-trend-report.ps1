param(
    [string]$HistoryJsonl = "target\validation\status-triad-history.ndjson",
    [string]$JsonOut = "target\validation\status-triad-trend-report.json",
    [string]$MdOut = "target\validation\status-triad-trend-report.md",
    [int]$Window = 20
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

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
    $count = $Values.Count
    $latest = [double]$Values[-1]
    $median = Get-Percentile -Values $Values -Percentile 50
    $p95 = Get-Percentile -Values $Values -Percentile 95
    return [ordered]@{
        samples = $count
        latest_ms = [Math]::Round($latest, 3)
        median_ms = [Math]::Round($median, 3)
        p95_ms = [Math]::Round($p95, 3)
    }
}

function Parse-HistoryEntries {
    param([string]$RawText)

    $entries = New-Object System.Collections.Generic.List[object]
    $lineModeFailed = $false
    foreach ($line in ($RawText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $entries.Add(($line | ConvertFrom-Json)) | Out-Null
        }
        catch {
            $lineModeFailed = $true
            break
        }
    }
    if (-not $lineModeFailed -and $entries.Count -gt 0) {
        return @($entries.ToArray())
    }

    # Fallback for legacy pretty-printed concatenated JSON objects.
    $entries = New-Object System.Collections.Generic.List[object]
    $buffer = ""
    $depth = 0
    $inString = $false
    $escaped = $false

    foreach ($ch in $RawText.ToCharArray()) {
        $buffer += $ch

        if ($escaped) {
            $escaped = $false
            continue
        }
        if ($ch -eq '\') {
            $escaped = $true
            continue
        }
        if ($ch -eq '"') {
            $inString = -not $inString
            continue
        }
        if ($inString) {
            continue
        }

        if ($ch -eq '{') { $depth++ }
        elseif ($ch -eq '}') { $depth-- }

        if ($depth -eq 0 -and -not [string]::IsNullOrWhiteSpace($buffer)) {
            $candidate = $buffer.Trim()
            if ($candidate.StartsWith("{") -and $candidate.EndsWith("}")) {
                try {
                    $entries.Add(($candidate | ConvertFrom-Json)) | Out-Null
                    $buffer = ""
                }
                catch {
                    # keep buffering
                }
            }
        }
    }

    return @($entries.ToArray())
}

$historyPath = $HistoryJsonl
if (-not [System.IO.Path]::IsPathRooted($historyPath)) {
    $historyPath = Join-Path $repoRoot $historyPath
}
if (-not (Test-Path $historyPath -PathType Leaf)) {
    throw "Status triad history not found: $historyPath"
}

$raw = Get-Content -Path $historyPath -Raw
$entries = Parse-HistoryEntries -RawText $raw
if ($entries.Count -eq 0) {
    throw "Status triad history has no entries: $historyPath"
}

$windowSize = [Math]::Min($Window, $entries.Count)
$windowEntries = if ($windowSize -eq $entries.Count) { $entries } else { $entries[($entries.Count - $windowSize)..($entries.Count - 1)] }

$core = @()
$world = @()
$frontier = @()
foreach ($entry in $windowEntries) {
    $obs = $entry.latency_budget.observed_ms
    if ($null -eq $obs) {
        throw "History entry missing latency_budget.observed_ms"
    }
    $core += [double]$obs.core
    $world += [double]$obs.world
    $frontier += [double]$obs.frontier
}

$coreStats = Get-Stats -Values $core
$worldStats = Get-Stats -Values $world
$frontierStats = Get-Stats -Values $frontier

$trend = [ordered]@{
    ok = $true
    generated_at = (Get-Date).ToString("o")
    history = $historyPath
    total_samples = $entries.Count
    window_samples = $windowSize
    triad = [ordered]@{
        core = $coreStats
        world = $worldStats
        frontier = $frontierStats
    }
}

$jsonPath = $JsonOut
if (-not [System.IO.Path]::IsPathRooted($jsonPath)) {
    $jsonPath = Join-Path $repoRoot $jsonPath
}
$jsonDir = Split-Path -Parent $jsonPath
if ($jsonDir) {
    New-Item -ItemType Directory -Path $jsonDir -Force | Out-Null
}
$trend | ConvertTo-Json -Depth 6 | Set-Content -Path $jsonPath -Encoding UTF8

$mdPath = $MdOut
if (-not [System.IO.Path]::IsPathRooted($mdPath)) {
    $mdPath = Join-Path $repoRoot $mdPath
}
$mdDir = Split-Path -Parent $mdPath
if ($mdDir) {
    New-Item -ItemType Directory -Path $mdDir -Force | Out-Null
}

$md = @"
# Status Triad Trend Report

- Generated: $($trend.generated_at)
- History: $historyPath
- Total samples: $($trend.total_samples)
- Window samples: $($trend.window_samples)

| Metric | Latest (ms) | Median (ms) | P95 (ms) | Samples |
|---|---:|---:|---:|---:|
| core | $($coreStats.latest_ms) | $($coreStats.median_ms) | $($coreStats.p95_ms) | $($coreStats.samples) |
| world | $($worldStats.latest_ms) | $($worldStats.median_ms) | $($worldStats.p95_ms) | $($worldStats.samples) |
| frontier | $($frontierStats.latest_ms) | $($frontierStats.median_ms) | $($frontierStats.p95_ms) | $($frontierStats.samples) |
"@

Set-Content -Path $mdPath -Value $md -Encoding UTF8

[ordered]@{
    ok = $true
    json = $jsonPath
    md = $mdPath
    window_samples = $windowSize
} | ConvertTo-Json -Depth 4
