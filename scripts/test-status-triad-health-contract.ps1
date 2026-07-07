param(
    [string]$HealthJson = "target\validation\status-triad-health.json"
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

$healthPath = $HealthJson
if (-not [System.IO.Path]::IsPathRooted($healthPath)) {
    $healthPath = Join-Path $repoRoot $healthPath
}

Assert-True (Test-Path $healthPath -PathType Leaf) "Status triad health JSON not found: $healthPath"

$payload = Get-Content -Path $healthPath -Raw | ConvertFrom-Json

Assert-True ($null -ne $payload.ok) "Health JSON missing ok"
Assert-True (@("pass", "warn", "fail") -contains [string]$payload.status) "Health JSON status must be pass/warn/fail"
Assert-True ([string]::IsNullOrWhiteSpace([string]$payload.generated_at) -eq $false) "Health JSON missing generated_at"
Assert-True ($payload.thresholds.warn_p95_ms -gt 0) "Health JSON warn threshold must be > 0"
Assert-True ($payload.thresholds.fail_p95_ms -gt 0) "Health JSON fail threshold must be > 0"
Assert-True ($payload.summary.max_p95_ms -ge 0) "Health JSON max_p95_ms must be >= 0"
Assert-True ($payload.summary.window_samples -ge 1) "Health JSON window_samples must be >= 1"
Assert-True ($payload.summary.total_samples -ge 1) "Health JSON total_samples must be >= 1"

foreach ($name in @("core", "world", "frontier")) {
    $node = $payload.triad.$name
    Assert-True ($null -ne $node) "Health JSON missing triad node: $name"
    Assert-True ($node.latest_ms -ge 0) "$name latest_ms must be >= 0"
    Assert-True ($node.median_ms -ge 0) "$name median_ms must be >= 0"
    Assert-True ($node.p95_ms -ge 0) "$name p95_ms must be >= 0"
}

$computedMax = [Math]::Max([double]$payload.triad.core.p95_ms, [Math]::Max([double]$payload.triad.world.p95_ms, [double]$payload.triad.frontier.p95_ms))
Assert-True ([Math]::Abs($computedMax - [double]$payload.summary.max_p95_ms) -lt 0.001) "Health JSON max_p95_ms mismatch"

[ordered]@{
    ok = $true
    status = $payload.status
    health = $healthPath
    checked = @(
        "required fields",
        "threshold fields",
        "triad metrics",
        "max p95 consistency"
    )
} | ConvertTo-Json -Depth 4
