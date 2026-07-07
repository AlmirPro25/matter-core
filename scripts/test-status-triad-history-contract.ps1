param(
    [string]$HistoryJsonl = "target\validation\status-triad-history.ndjson",
    [int]$MinSamples = 1,
    [double]$MaxP95Ms = 30000,
    [double]$MaxMedianMs = 30000,
    [switch]$EnforceRegression,
    [double]$MaxRegressionPercent = 35
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

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    $count = $sorted.Count
    Assert-True ($count -gt 0) "Cannot compute percentile on empty sample set"

    if ($count -eq 1) {
        return [double]$sorted[0]
    }

    $rank = ($Percentile / 100.0) * ($count - 1)
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

    $median = Get-Percentile -Values $Values -Percentile 50
    $p95 = Get-Percentile -Values $Values -Percentile 95
    $latest = [double]$Values[-1]
    return [ordered]@{
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
                    # keep buffering; malformed chunk may need more input
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

Assert-True (Test-Path $historyPath -PathType Leaf) "Status triad history not found: $historyPath"

$rawHistory = Get-Content -Path $historyPath -Raw
$entries = Parse-HistoryEntries -RawText $rawHistory

Assert-True ($entries.Count -ge $MinSamples) "Status triad history requires at least $MinSamples samples, found $($entries.Count)"

$coreSamples = @()
$worldSamples = @()
$frontierSamples = @()
foreach ($entry in $entries) {
    $obs = $entry.latency_budget.observed_ms
    Assert-True ($null -ne $obs) "History entry missing latency_budget.observed_ms"
    $coreSamples += [double]$obs.core
    $worldSamples += [double]$obs.world
    $frontierSamples += [double]$obs.frontier
}

$coreStats = Get-Stats -Values $coreSamples
$worldStats = Get-Stats -Values $worldSamples
$frontierStats = Get-Stats -Values $frontierSamples

foreach ($pair in @(
    @{ Name = "core"; Stats = $coreStats },
    @{ Name = "world"; Stats = $worldStats },
    @{ Name = "frontier"; Stats = $frontierStats }
)) {
    Assert-True ($pair.Stats.median_ms -le $MaxMedianMs) "Median latency exceeds threshold for $($pair.Name): $($pair.Stats.median_ms)ms > $MaxMedianMs ms"
    Assert-True ($pair.Stats.p95_ms -le $MaxP95Ms) "P95 latency exceeds threshold for $($pair.Name): $($pair.Stats.p95_ms)ms > $MaxP95Ms ms"
}

$regression = $null
if ($EnforceRegression -and $entries.Count -ge 2) {
    $previous = $entries[$entries.Count - 2].latency_budget.observed_ms
    $current = $entries[$entries.Count - 1].latency_budget.observed_ms

    $regression = [ordered]@{
        max_percent = $MaxRegressionPercent
        checks = @()
    }

    foreach ($name in @("core", "world", "frontier")) {
        $prev = [double]$previous.$name
        $curr = [double]$current.$name
        Assert-True ($prev -gt 0) "Previous latency must be > 0 for regression check ($name)"
        $deltaPercent = (($curr - $prev) / $prev) * 100.0
        $regression.checks += [ordered]@{
            name = $name
            previous_ms = [Math]::Round($prev, 3)
            current_ms = [Math]::Round($curr, 3)
            delta_percent = [Math]::Round($deltaPercent, 3)
        }
        Assert-True ($deltaPercent -le $MaxRegressionPercent) "Regression exceeded for ${name}: $([Math]::Round($deltaPercent, 3))% > $MaxRegressionPercent%"
    }
}

[ordered]@{
    ok = $true
    history = $historyPath
    samples = $entries.Count
    thresholds = [ordered]@{
        max_median_ms = $MaxMedianMs
        max_p95_ms = $MaxP95Ms
        min_samples = $MinSamples
    }
    triad = [ordered]@{
        core = $coreStats
        world = $worldStats
        frontier = $frontierStats
    }
    regression = if ($regression) { $regression } else { $null }
} | ConvertTo-Json -Depth 6
