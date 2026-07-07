param(
    [string]$MapPath = "docs\technical\FRONTIER_SIMULATION_REFINEMENT_MAP.md"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Needle
    )
    if (-not $Text.Contains($Needle)) {
        throw "Refinement map missing required content: $Needle"
    }
}

$path = if ([System.IO.Path]::IsPathRooted($MapPath)) { $MapPath } else { Join-Path $repoRoot $MapPath }
if (-not (Test-Path $path -PathType Leaf)) {
    throw "Frontier simulation refinement map not found: $path"
}

$doc = Get-Content -Path $path -Raw

foreach ($required in @(
    "Frontier Simulation Refinement Map",
    "## Current Surface",
    "## Quality Ladder",
    "## API Refinement Map",
    "## Unified Contract",
    "## Implementation Order",
    "## Claim Boundaries",
    "quantum",
    "neuromorphic",
    "photonic",
    "wetware",
    "bell_stats(shots, seed)",
    "lif_threshold_probe",
    "truth_table",
    "stimulate_seeded",
    "frontier-sim-quality-json",
    "schemas/frontier-simulation-quality.schema.json",
    "real quantum speedup",
    "hardware-equivalent results"
)) {
    Assert-Contains -Text $doc -Needle $required
}

[ordered]@{
    ok = $true
    map = $path
    checked = @(
        "current surface",
        "quality ladder",
        "api refinement map",
        "unified contract",
        "implementation order",
        "claim boundaries",
        "all frontier modules"
    )
} | ConvertTo-Json -Depth 4
