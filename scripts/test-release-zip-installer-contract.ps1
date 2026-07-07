param(
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$ChecksumJsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$installRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_zip_installer_contract_" + [guid]::NewGuid().ToString("N"))

try {
    foreach ($required in @($ZipPath, $ChecksumJsonPath, $Sha256Path)) {
        if (-not (Test-Path $required -PathType Leaf)) {
            throw "Required release artifact not found: $required"
        }
    }

    $installer = Join-Path $PSScriptRoot "install-release-zip.ps1"
    if (-not (Test-Path $installer -PathType Leaf)) {
        throw "Release zip installer not found: $installer"
    }

    & powershell -ExecutionPolicy Bypass -File $installer `
        -ZipPath $ZipPath `
        -ChecksumJsonPath $ChecksumJsonPath `
        -Sha256Path $Sha256Path `
        -InstallDir $installRoot `
        -NoPath
    if ($LASTEXITCODE -ne 0) {
        throw "Release zip installer failed with exit code $LASTEXITCODE"
    }

    $matterExe = Join-Path $installRoot "bin\matter.exe"
    $installManifest = Join-Path $installRoot "INSTALL_MANIFEST.json"
    $worldStatus = Join-Path $installRoot "target\world\world-status.json"
    $frontierStatus = Join-Path $installRoot "target\frontier\frontier-status.json"
    $diagnoser = Join-Path $installRoot "scripts\diagnose-local-install.ps1"
    foreach ($required in @($matterExe, $installManifest, $worldStatus, $frontierStatus, $diagnoser)) {
        if (-not (Test-Path $required -PathType Leaf)) {
            throw "Release zip installer missing installed file: $required"
        }
    }

    $manifest = Get-Content $installManifest -Raw | ConvertFrom-Json
    if ($manifest.schema -ne "matter.release.install.v1") {
        throw "Installed manifest has unexpected schema: $($manifest.schema)"
    }

    $capabilities = & $matterExe capabilities-json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter capabilities-json failed with exit code $LASTEXITCODE"
    }
    $capabilitiesJson = $capabilities | ConvertFrom-Json
    if (-not $capabilitiesJson.ok) {
        throw "Installed matter capabilities-json did not report ok=true"
    }

    $world = & $matterExe world-status-json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter world-status-json failed with exit code $LASTEXITCODE"
    }
    $worldJson = $world | ConvertFrom-Json
    if (-not $worldJson.ok -or $worldJson.summary.mode -ne "logical_world_partition") {
        throw "Installed matter world-status-json failed runtime contract"
    }

    $frontier = & $matterExe frontier-status-json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter frontier-status-json failed with exit code $LASTEXITCODE"
    }
    $frontierJson = $frontier | ConvertFrom-Json
    if (-not $frontierJson.summary.all_non_stub -or -not $frontierJson.summary.all_simulated -or $frontierJson.summary.any_hardware) {
        throw "Installed matter frontier-status-json failed reality contract"
    }

    $diagnosis = & powershell -NoProfile -ExecutionPolicy Bypass -File $diagnoser -InstallDir $installRoot -AllowMissingPath -Json
    if ($LASTEXITCODE -ne 0) {
        throw "Release zip installer diagnosis failed with exit code $LASTEXITCODE"
    }
    $diagnosisJson = $diagnosis | ConvertFrom-Json
    if (-not $diagnosisJson.ok) {
        throw "Release zip installer diagnosis did not report ok=true"
    }

    $uninstaller = Join-Path $installRoot "scripts\uninstall-local.ps1"
    if (-not (Test-Path $uninstaller -PathType Leaf)) {
        throw "Release zip installer missing uninstaller: $uninstaller"
    }
    & powershell -ExecutionPolicy Bypass -File $uninstaller -InstallDir $installRoot -NoPath -NoPause
    if ($LASTEXITCODE -ne 0) {
        throw "Release zip installer uninstaller failed with exit code $LASTEXITCODE"
    }
    if (Test-Path $installRoot) {
        throw "Release zip installer uninstaller did not remove install root: $installRoot"
    }

    [ordered]@{
        ok = $true
        checked = @(
            "zip checksum verified before install",
            "zip installer installed matter.exe",
            "zip installer wrote install manifest",
            "zip installer installed world status artifact",
            "zip installer installed frontier status artifact",
            "installed CLI capabilities-json works",
            "installed CLI world-status-json works",
            "installed CLI frontier-status-json works",
            "installed diagnosis passes",
            "installed uninstaller removes install root"
        )
    } | ConvertTo-Json -Depth 4
}
finally {
    if (Test-Path $installRoot) {
        Remove-Item -LiteralPath $installRoot -Recurse -Force
    }
}
