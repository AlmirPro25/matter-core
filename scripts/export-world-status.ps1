param(
    [string]$Out = "target\world\world-status.json",
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
    $raw = & $CliPath world-status-json 2>&1
}
else {
    $raw = & cargo run -q -p matter-cli -- world-status-json 2>&1
}

if ($LASTEXITCODE -ne 0) {
    throw "world-status-json failed: $raw"
}

$outPath = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $RepoRoot $Out }
$outDir = Split-Path -Parent $outPath
if ($outDir) {
    New-Item -ItemType Directory -Force $outDir | Out-Null
}

Set-Content -Path $outPath -Value $raw -Encoding UTF8

& powershell -ExecutionPolicy Bypass -File ".\scripts\test-world-status-contract.ps1" -StatusPath $outPath
if ($LASTEXITCODE -ne 0) {
    throw "Exported world status failed contract validation"
}

Write-Host "World status exported to $outPath"
