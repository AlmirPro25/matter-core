param(
    [string]$SetupPath = "dist\matter-core-beta-setup.exe",
    [string]$SiteReleasePath = "site\release.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$installRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_setup_exe_contract_" + [guid]::NewGuid().ToString("N"))

try {
    if (-not (Test-Path $SetupPath -PathType Leaf)) {
        throw "Setup exe not found: $SetupPath"
    }
    if (-not (Test-Path $SiteReleasePath -PathType Leaf)) {
        throw "Site release metadata not found: $SiteReleasePath"
    }

    $release = Get-Content $SiteReleasePath -Raw | ConvertFrom-Json
    $setupEntry = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-beta-setup.exe" } | Select-Object -First 1
    if (-not $setupEntry) {
        throw "Site release metadata missing setup exe artifact"
    }

    $setupFullPath = (Resolve-Path $SetupPath).Path
    $actualHash = (Get-FileHash -LiteralPath $setupFullPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $actualSize = (Get-Item -LiteralPath $setupFullPath).Length
    if ($actualHash -ne [string]$setupEntry.sha256) {
        throw "Setup exe hash mismatch between file and site metadata"
    }
    if ($actualSize -ne [int64]$setupEntry.size_bytes) {
        throw "Setup exe size mismatch between file and site metadata"
    }
    if ($setupEntry.signed) {
        throw "Beta setup metadata must not claim signed=true until real code signing is applied"
    }

    $previousCi = $env:CI
    $env:CI = "true"
    & $setupFullPath -InstallDir $installRoot -NoPath
    if ($LASTEXITCODE -ne 0) {
        throw "Setup exe install failed with exit code $LASTEXITCODE"
    }

    $matterExe = Join-Path $installRoot "bin\matter.exe"
    $agentDemo = Join-Path $installRoot "examples\agent_policy_demo.matter"
    $diagnoser = Join-Path $installRoot "scripts\diagnose-local-install.ps1"
    $uninstaller = Join-Path $installRoot "scripts\uninstall-local.ps1"
    foreach ($required in @($matterExe, $agentDemo, $diagnoser, $uninstaller)) {
        if (-not (Test-Path $required -PathType Leaf)) {
            throw "Setup exe install missing expected file: $required"
        }
    }

    $demoOutput = & $matterExe run $agentDemo
    if ($LASTEXITCODE -ne 0) {
        throw "Installed setup exe demo failed with exit code $LASTEXITCODE"
    }
    $demoText = ($demoOutput -join "`n")
    if (-not $demoText.Contains("Matter Core agent policy demo") -or -not $demoText.Contains("decision")) {
        throw "Installed setup exe demo output missing expected text"
    }

    $diagnosis = & powershell -NoProfile -ExecutionPolicy Bypass -File $diagnoser -InstallDir $installRoot -AllowMissingPath -Json
    if ($LASTEXITCODE -ne 0) {
        throw "Setup exe diagnosis failed with exit code $LASTEXITCODE"
    }
    $diagnosisJson = $diagnosis | ConvertFrom-Json
    if (-not $diagnosisJson.ok) {
        throw "Setup exe diagnosis did not report ok=true"
    }

    & powershell -NoProfile -ExecutionPolicy Bypass -File $uninstaller -InstallDir $installRoot -NoPath -NoPause
    if ($LASTEXITCODE -ne 0) {
        throw "Setup exe uninstaller failed with exit code $LASTEXITCODE"
    }
    if (Test-Path $installRoot) {
        throw "Setup exe uninstaller did not remove install root: $installRoot"
    }

    [ordered]@{
        ok = $true
        setup = $setupFullPath
        sha256 = $actualHash
        signed = $false
        checked = @(
            "setup exe exists",
            "site metadata matches setup exe",
            "setup exe does not claim signing",
            "setup exe installs Matter",
            "installed agent policy demo runs",
            "installed diagnosis passes",
            "installed uninstaller removes install root"
        )
    } | ConvertTo-Json -Depth 4
}
finally {
    if (Test-Path Variable:previousCi) {
        $env:CI = $previousCi
    }
    if (Test-Path $installRoot) {
        Remove-Item -LiteralPath $installRoot -Recurse -Force
    }
}
