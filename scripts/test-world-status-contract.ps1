param(
    [string]$StatusPath,
    [string]$CliPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

if ($StatusPath) {
    if (-not (Test-Path $StatusPath -PathType Leaf)) {
        throw "World status file not found: $StatusPath"
    }
    $raw = Get-Content $StatusPath -Raw
}
elseif ($CliPath) {
    if (-not (Test-Path $CliPath -PathType Leaf)) {
        throw "CLI not found: $CliPath"
    }
    $raw = & $CliPath world-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "world-status-json failed: $raw"
    }
}
else {
    $raw = & cargo run -q -p matter-cli -- world-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "world-status-json failed: $raw"
    }
}

$payload = $raw | ConvertFrom-Json
$schemaRef = $payload.PSObject.Properties['$schema'].Value

Assert-True ($schemaRef -eq "schemas/world-status.schema.json") "Unexpected world status schema reference"
Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "World status schema reference must be repo-relative"
Assert-True (Test-Path (Join-Path $RepoRoot $schemaRef)) "World status schema file is missing"
Assert-True ($payload.schema_version -eq 1) "Unexpected world status schema version"
Assert-True ($payload.kind -eq "world_status") "Unexpected world status kind"
Assert-True ($payload.ok -eq $true) "World status must report ok=true"
Assert-True ($payload.summary.mode -eq "logical_world_partition") "Unexpected world status mode"
Assert-True ($payload.summary.degraded -eq $true) "World status should show degraded=true for overloaded sample"
Assert-True ($payload.summary.hot_cells -eq 1) "World status sample should produce one hot cell"
Assert-True ($payload.summary.cell_count -eq 2) "World status sample should produce two cells"
Assert-True ($payload.summary.entities -eq 4) "World status sample should seed four entities"
Assert-True ($payload.summary.sample_visible_count -eq 2) "World status sample should expose two visible entities for p1"
Assert-True ($payload.summary.sample_hidden_count -eq 1) "World status sample should hide one entity for p1"

[ordered]@{
    ok = $true
    schema = $schemaRef
    checked = @(
        "schema reference",
        "schema file exists",
        "summary mode",
        "overload/degraded evidence",
        "partition counts",
        "interest visibility counts"
    )
} | ConvertTo-Json -Depth 4
