param(
    [string]$CliPath,
    [string]$Version = "0.1.0-beta",
    [switch]$SkipReleaseBuild,
    [switch]$SkipCargoTests,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$results = New-Object System.Collections.Generic.List[object]

function Invoke-GateStep {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    $started = Get-Date
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "Beta gate step failed: $Name (exit code $LASTEXITCODE)"
    }

    $duration = [math]::Round(((Get-Date) - $started).TotalSeconds, 3)
    $results.Add([ordered]@{
        name = $Name
        ok = $true
        seconds = $duration
    }) | Out-Null
}

if (-not $SkipReleaseBuild) {
    Invoke-GateStep "build release package" {
        $args = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\build-release-package.ps1")
        if ($CliPath) {
            $args += @("-CliPath", $CliPath)
        }
        & powershell @args
    }
}

Invoke-GateStep "build windows setup exe" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\build-windows-setup-exe.ps1"
}

Invoke-GateStep "build download site" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\build-download-site.ps1" -Version $Version -Channel "beta"
}

Invoke-GateStep "export beta release notes" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-beta-release-notes.ps1" -Tag ("v{0}" -f $Version)
}

Invoke-GateStep "test release package contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-package-contract.ps1"
}

Invoke-GateStep "test release checksum contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-artifact-checksums-contract.ps1"
}

Invoke-GateStep "test release zip installer contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-zip-installer-contract.ps1"
}

Invoke-GateStep "test windows setup exe contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-windows-setup-exe-contract.ps1"
}

Invoke-GateStep "test download site contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-download-site-contract.ps1"
}

Invoke-GateStep "test beta readiness contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-readiness-contract.ps1"
}

Invoke-GateStep "test beta feedback contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-feedback-contract.ps1"
}

Invoke-GateStep "test beta release notes contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-release-notes-contract.ps1"
}

Invoke-GateStep "test beta site workflow contract" {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-beta-site-workflow-contract.ps1"
}

if (-not $SkipCargoTests) {
    Invoke-GateStep "cargo test" {
        cargo test -q
    }
}

$release = Get-Content "site\release.json" -Raw | ConvertFrom-Json
$zip = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1

$summary = [ordered]@{
    ok = $true
    version = $release.version
    channel = $release.channel
    status = $release.status
    production_ready = [bool]$release.production_ready
    zip = if ($zip) {
        [ordered]@{
            name = $zip.name
            size_bytes = [int64]$zip.size_bytes
            sha256 = [string]$zip.sha256
        }
    } else {
        $null
    }
    steps = @($results.ToArray())
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 8
}
else {
    Write-Host ""
    Write-Host "Matter Core beta gate passed." -ForegroundColor Green
    Write-Host ("Version: {0}" -f $summary.version) -ForegroundColor Green
    Write-Host ("Status: {0}" -f $summary.status) -ForegroundColor Green
    Write-Host ("Zip SHA-256: {0}" -f $summary.zip.sha256) -ForegroundColor Green
}
