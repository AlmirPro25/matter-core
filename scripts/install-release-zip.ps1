param(
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$ChecksumJsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt",
    [string]$InstallDir = "$env:LOCALAPPDATA\Matter",
    [switch]$NoPath,
    [switch]$SkipChecksum,
    [switch]$SkipPostInstallCheck,
    [switch]$KeepExtracted
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$tempExtractRoot = $null

function Resolve-ExistingFile {
    param(
        [string]$Path,
        [string]$Description
    )

    if (-not (Test-Path $Path -PathType Leaf)) {
        throw "$Description not found: $Path"
    }
    return (Resolve-Path $Path).Path
}

function Assert-ZipChecksum {
    param(
        [string]$ZipFullPath,
        [string]$JsonFullPath,
        [string]$Sha256FullPath
    )

    $zipName = Split-Path $ZipFullPath -Leaf
    $actualHash = (Get-FileHash -LiteralPath $ZipFullPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $actualSize = (Get-Item -LiteralPath $ZipFullPath).Length

    $checksumJson = Get-Content $JsonFullPath -Raw | ConvertFrom-Json
    $artifacts = @($checksumJson.artifacts)
    $jsonEntry = $artifacts | Where-Object {
        $_.name -eq $zipName -or (Split-Path ([string]$_.path) -Leaf) -eq $zipName
    } | Select-Object -First 1

    if (-not $jsonEntry) {
        throw "Checksum JSON does not contain artifact: $zipName"
    }
    if ([string]$jsonEntry.sha256 -ne $actualHash) {
        throw "Release zip SHA-256 mismatch. Expected $($jsonEntry.sha256), got $actualHash"
    }
    if ([int64]$jsonEntry.size_bytes -ne $actualSize) {
        throw "Release zip size mismatch. Expected $($jsonEntry.size_bytes), got $actualSize"
    }

    $shaLines = Get-Content $Sha256FullPath
    $shaEntry = $shaLines | Where-Object {
        $lineParts = $_ -split '\s+', 2
        $lineParts.Count -ge 2 -and (Split-Path $lineParts[1] -Leaf) -eq $zipName
    } | Select-Object -First 1
    if (-not $shaEntry) {
        throw "SHA256SUMS does not contain artifact: $zipName"
    }

    $parts = $shaEntry -split '\s+', 2
    if ($parts.Count -lt 2) {
        throw "Invalid SHA256SUMS line for artifact: $shaEntry"
    }

    $shaHash = $parts[0].ToLowerInvariant()
    if ($shaHash -ne $actualHash) {
        throw "SHA256SUMS hash mismatch. Expected $shaHash, got $actualHash"
    }
}

try {
    $zipFullPath = Resolve-ExistingFile $ZipPath "Release zip"

    if (-not $SkipChecksum) {
        $jsonFullPath = Resolve-ExistingFile $ChecksumJsonPath "Release checksum JSON"
        $sha256FullPath = Resolve-ExistingFile $Sha256Path "Release SHA256SUMS"
        Assert-ZipChecksum -ZipFullPath $zipFullPath -JsonFullPath $jsonFullPath -Sha256FullPath $sha256FullPath
    }

    $tempExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_zip_install_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force $tempExtractRoot | Out-Null
    Expand-Archive -Path $zipFullPath -DestinationPath $tempExtractRoot -Force

    $directPackageRoot = Join-Path $tempExtractRoot "matter-core-windows-x64"
    $packageRoot = if (Test-Path $directPackageRoot -PathType Container) { $directPackageRoot } else { $tempExtractRoot }
    $installer = Join-Path $packageRoot "scripts\install-release-local.ps1"

    if (-not (Test-Path $installer -PathType Leaf)) {
        throw "Release installer not found in extracted package: scripts\install-release-local.ps1"
    }

    $installArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", $installer,
        "-InstallDir", $InstallDir
    )
    if ($NoPath) {
        $installArgs += "-NoPath"
    }
    if ($SkipPostInstallCheck) {
        $installArgs += "-SkipPostInstallCheck"
    }

    & powershell @installArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Release installer failed with exit code $LASTEXITCODE"
    }

    [ordered]@{
        ok = $true
        zip_path = $zipFullPath
        checksum_verified = -not $SkipChecksum
        install_dir = [System.IO.Path]::GetFullPath($InstallDir)
        extracted_package_root = $packageRoot
    } | ConvertTo-Json -Depth 4
}
finally {
    if ($tempExtractRoot -and -not $KeepExtracted -and (Test-Path $tempExtractRoot)) {
        Remove-Item -LiteralPath $tempExtractRoot -Recurse -Force
    }
}
