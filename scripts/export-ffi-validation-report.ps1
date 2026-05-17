param(
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json",
    [string]$Out = "target\ffi\ffi-validation-report.md"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

if (-not (Test-Path $MatrixPath)) {
    throw "Missing FFI validation matrix: $MatrixPath"
}

$matrix = Get-Content $MatrixPath -Raw | ConvertFrom-Json

if ($matrix.PSObject.Properties['$schema'].Value -ne "schemas/ffi-validation-matrix.schema.json") {
    throw "FFI validation matrix has unexpected schema reference"
}

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# FFI Validation Report")
$lines.Add("")
$lines.Add(("Generated: {0}" -f $matrix.generated_at))
$lines.Add("")
$lines.Add(('Schema: `{0}`' -f $matrix.PSObject.Properties['$schema'].Value))
$lines.Add("")
$lines.Add(("Rule: {0}" -f $matrix.rule))
$lines.Add("")
$lines.Add("| Bridge | Crate | Status | Production claim allowed | Blocker |")
$lines.Add("| --- | --- | --- | --- | --- |")

foreach ($bridge in @($matrix.bridges)) {
    $allowed = if ($bridge.production_claim_allowed) { "true" } else { "false" }
    $blocker = ($bridge.production_blocker -replace "\|", "\|")
    $lines.Add(('| `{0}` | `{1}` | `{2}` | `{3}` | {4} |' -f $bridge.id, $bridge.crate, $bridge.status, $allowed, $blocker))
}

$lines.Add("")
$lines.Add("## Evidence")
$lines.Add("")

foreach ($bridge in @($matrix.bridges)) {
    $lines.Add(("### {0}" -f $bridge.id))
    $lines.Add("")
    foreach ($item in @($bridge.evidence)) {
        $lines.Add("- $item")
    }
    $lines.Add("")
}

$outDir = Split-Path -Parent $Out
if ($outDir) {
    New-Item -ItemType Directory -Force $outDir | Out-Null
}

$lines | Set-Content -Path $Out -Encoding UTF8

[ordered]@{
    ok = $true
    matrix = $MatrixPath
    report = $Out
    bridges = @($matrix.bridges).Count
} | ConvertTo-Json -Depth 4
