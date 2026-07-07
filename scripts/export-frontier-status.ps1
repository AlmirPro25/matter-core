param(
    [string]$Out = "target\frontier\frontier-status.json",
    [string]$CliPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

if ($CliPath) {
    if (-not (Test-Path $CliPath -PathType Leaf)) {
        throw "CLI not found: $CliPath"
    }
    $raw = & $CliPath frontier-status-json 2>&1
}
else {
    $raw = & cargo run -q -p matter-cli -- frontier-status-json 2>&1
}

if ($LASTEXITCODE -ne 0) {
    throw "frontier-status-json failed: $raw"
}

$outPath = if ([System.IO.Path]::IsPathRooted($Out)) {
    $Out
}
else {
    Join-Path $RepoRoot $Out
}

$outDir = Split-Path -Parent $outPath
if ($outDir) {
    New-Item -ItemType Directory -Force $outDir | Out-Null
}

Set-Content -Path $outPath -Value $raw -Encoding UTF8

& powershell -ExecutionPolicy Bypass -File ".\scripts\test-frontier-status-contract.ps1" -StatusPath $outPath
if ($LASTEXITCODE -ne 0) {
    throw "Exported frontier status failed contract validation"
}

Write-Host "Frontier status exported to $outPath"
