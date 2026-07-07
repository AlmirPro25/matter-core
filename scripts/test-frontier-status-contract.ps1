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
        throw "Frontier status file not found: $StatusPath"
    }
    $raw = Get-Content $StatusPath -Raw
}
elseif ($CliPath) {
    if (-not (Test-Path $CliPath -PathType Leaf)) {
        throw "CLI not found: $CliPath"
    }
    $raw = & $CliPath frontier-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "frontier-status-json failed: $raw"
    }
}
else {
    $raw = & cargo run -q -p matter-cli -- frontier-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "frontier-status-json failed: $raw"
    }
}

$payload = $raw | ConvertFrom-Json
$schemaRef = $payload.PSObject.Properties['$schema'].Value

Assert-True ($schemaRef -eq "schemas/frontier-status.schema.json") "Unexpected frontier status schema reference"
Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "Frontier status schema reference must be repo-relative"
Assert-True (Test-Path (Join-Path $RepoRoot $schemaRef)) "Frontier status schema file is missing"
Assert-True ($payload.schema_version -eq 1) "Unexpected frontier status schema version"
Assert-True ($payload.kind -eq "frontier_status") "Unexpected frontier status kind"
Assert-True ($payload.summary.all_non_stub -eq $true) "Frontier backends must report non-stub"
Assert-True ($payload.summary.all_simulated -eq $true) "Frontier backends must report simulated"
Assert-True ($payload.summary.any_hardware -eq $false) "Frontier status must not claim hardware"

foreach ($backend in @("quantum", "photonic", "neuromorphic", "wetware")) {
    $status = $payload.backends.$backend
    Assert-True ($null -ne $status) "Missing frontier backend status: $backend"
    Assert-True ($status.backend -eq $backend) "Backend name mismatch for $backend"
    Assert-True ($status.stub -eq $false) "$backend must not report stub=true"
    Assert-True ($status.hardware -eq $false) "$backend must not report hardware=true"
    Assert-True ($status.simulated -eq $true) "$backend must report simulated=true"
    Assert-True ([string]::IsNullOrWhiteSpace($status.model) -eq $false) "$backend must report a model"
    Assert-True ([string]::IsNullOrWhiteSpace($status.mode) -eq $false) "$backend must report a mode"
    Assert-True (@($status.capabilities).Count -gt 0) "$backend must report capabilities"
}

[ordered]@{
    ok = $true
    schema = $schemaRef
    checked = @(
        "schema reference",
        "schema file exists",
        "summary flags",
        "required backends",
        "backend reality flags",
        "backend capabilities"
    )
} | ConvertTo-Json -Depth 4
