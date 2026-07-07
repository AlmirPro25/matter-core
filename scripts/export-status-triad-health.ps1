param(
    [string]$TriadLatestJson = "target\validation\status-triad-latest.json",
    [string]$TriadTrendJson = "target\validation\status-triad-trend-report.json",
    [string]$HealthOut = "target\validation\status-triad-health.json",
    [double]$WarnP95Ms = 30000,
    [double]$FailP95Ms = 45000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Resolve-PathSafe {
    param([string]$PathValue)
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return $PathValue
    }
    return (Join-Path $repoRoot $PathValue)
}

function Assert-FileExists {
    param([string]$PathValue, [string]$Label)
    if (-not (Test-Path $PathValue -PathType Leaf)) {
        throw "$Label not found: $PathValue"
    }
}

$latestPath = Resolve-PathSafe $TriadLatestJson
$trendPath = Resolve-PathSafe $TriadTrendJson
$healthPath = Resolve-PathSafe $HealthOut

Assert-FileExists -PathValue $latestPath -Label "Triad latest JSON"
Assert-FileExists -PathValue $trendPath -Label "Triad trend JSON"

$latest = Get-Content -Path $latestPath -Raw | ConvertFrom-Json
$trend = Get-Content -Path $trendPath -Raw | ConvertFrom-Json

if (-not $latest.ok) { throw "Triad latest JSON reports ok=false" }
if (-not $trend.ok) { throw "Triad trend JSON reports ok=false" }

$triad = [ordered]@{
    core = [ordered]@{
        latest_ms = [double]$trend.triad.core.latest_ms
        median_ms = [double]$trend.triad.core.median_ms
        p95_ms = [double]$trend.triad.core.p95_ms
    }
    world = [ordered]@{
        latest_ms = [double]$trend.triad.world.latest_ms
        median_ms = [double]$trend.triad.world.median_ms
        p95_ms = [double]$trend.triad.world.p95_ms
    }
    frontier = [ordered]@{
        latest_ms = [double]$trend.triad.frontier.latest_ms
        median_ms = [double]$trend.triad.frontier.median_ms
        p95_ms = [double]$trend.triad.frontier.p95_ms
    }
}

$maxP95 = [Math]::Max($triad.core.p95_ms, [Math]::Max($triad.world.p95_ms, $triad.frontier.p95_ms))
$status = if ($maxP95 -gt $FailP95Ms) { "fail" } elseif ($maxP95 -gt $WarnP95Ms) { "warn" } else { "pass" }

$health = [ordered]@{
    ok = ($status -ne "fail")
    status = $status
    generated_at = (Get-Date).ToString("o")
    source = [ordered]@{
        latest = $latestPath
        trend = $trendPath
    }
    thresholds = [ordered]@{
        warn_p95_ms = $WarnP95Ms
        fail_p95_ms = $FailP95Ms
    }
    summary = [ordered]@{
        max_p95_ms = [Math]::Round($maxP95, 3)
        window_samples = [int]$trend.window_samples
        total_samples = [int]$trend.total_samples
    }
    triad = $triad
}

$healthDir = Split-Path -Parent $healthPath
if ($healthDir) {
    New-Item -ItemType Directory -Path $healthDir -Force | Out-Null
}
$health | ConvertTo-Json -Depth 8 | Set-Content -Path $healthPath -Encoding UTF8

$health | ConvertTo-Json -Depth 8
