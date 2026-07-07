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
        throw "Core status file not found: $StatusPath"
    }
    $raw = Get-Content $StatusPath -Raw
}
elseif ($CliPath) {
    if (-not (Test-Path $CliPath -PathType Leaf)) {
        throw "CLI not found: $CliPath"
    }
    $raw = & $CliPath core-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "core-status-json failed: $raw"
    }
}
else {
    $raw = & cargo run -q -p matter-cli -- core-status-json 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "core-status-json failed: $raw"
    }
}

$payload = $raw | ConvertFrom-Json
$schemaRef = $payload.PSObject.Properties['$schema'].Value

Assert-True ($schemaRef -eq "schemas/core-status.schema.json") "Unexpected core status schema reference"
Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "Core status schema reference must be repo-relative"
Assert-True (Test-Path (Join-Path $RepoRoot $schemaRef)) "Core status schema file is missing"
Assert-True ($payload.schema_version -eq 1) "Unexpected core status schema version"
Assert-True ($payload.kind -eq "core_status") "Unexpected core status kind"
Assert-True ($payload.ok -eq $true) "Core status must report ok=true"
Assert-True ($payload.summary.core_loop_validated -eq $true) "Core loop must be validated"
Assert-True ($payload.summary.pipeline -eq "source_to_bytecode_to_vm_to_runtime") "Unexpected pipeline label"
Assert-True ($payload.summary.bytecode -eq "MBC1") "Core status must declare MBC1"
Assert-True ($payload.summary.execution_controlled -eq $true) "Core execution must be controlled"
Assert-True ($payload.summary.introspection_available -eq $true) "Core introspection must be available"
Assert-True ($payload.summary.production_ready -eq $false) "Core status must not claim production-ready"

$checks = @($payload.checks)
foreach ($name in @("parse", "compile", "reflection", "reflexive_guard", "run", "event_dispatch", "output_capture")) {
    $match = $checks | Where-Object { $_.name -eq $name } | Select-Object -First 1
    Assert-True ($null -ne $match) "Missing core status check: $name"
    Assert-True ($match.passed -eq $true) "Core status check failed: $name"
}

$output = @($payload.evidence.output)
Assert-True ($output -contains "Matter Core") "Core status output must include Matter Core"
Assert-True ($output -contains "8") "Core status output must include fib result"
Assert-True ($output -contains "event: boot") "Core status output must include event output"
Assert-True ($payload.evidence.bytecode.summary.instructions -gt 0) "Core status must include bytecode instruction evidence"
Assert-True ($payload.evidence.reflection.total_statements -gt 0) "Core status must include AST reflection evidence"

[ordered]@{
    ok = $true
    schema = $schemaRef
    checked = @(
        "schema reference",
        "schema file exists",
        "core loop flags",
        "required checks",
        "captured output",
        "bytecode evidence",
        "reflection evidence"
    )
} | ConvertTo-Json -Depth 4
