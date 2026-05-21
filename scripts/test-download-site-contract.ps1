param(
    [string]$SiteRoot = "site"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$sitePath = [System.IO.Path]::GetFullPath($SiteRoot)
$indexPath = Join-Path $sitePath "index.html"
$releasePath = Join-Path $sitePath "release.json"

foreach ($required in @(
    $indexPath,
    (Join-Path $sitePath "assets\styles.css"),
    (Join-Path $sitePath "assets\app.js"),
    (Join-Path $sitePath "BETA_NOTES.md"),
    (Join-Path $sitePath "TESTER_GUIDE.md"),
    $releasePath,
    (Join-Path $sitePath "downloads\matter-core-windows-x64.zip"),
    (Join-Path $sitePath "downloads\matter-core-beta-setup.exe"),
    (Join-Path $sitePath "downloads\install-matter-beta.cmd"),
    (Join-Path $sitePath "downloads\install-release-zip.ps1"),
    (Join-Path $sitePath "downloads\release-checksums.json"),
    (Join-Path $sitePath "downloads\SHA256SUMS.txt")
)) {
    if (-not (Test-Path $required -PathType Leaf)) {
        throw "Download site missing required file: $required"
    }
}

$html = Get-Content $indexPath -Raw
foreach ($requiredText in @(
    "downloads/matter-core-windows-x64.zip",
    "downloads/matter-core-beta-setup.exe",
    "downloads/install-matter-beta.cmd",
    "downloads/install-release-zip.ps1",
    "downloads/release-checksums.json",
    "downloads/SHA256SUMS.txt",
    "BETA_NOTES.md",
    "TESTER_GUIDE.md",
    "Guia do tester",
    "Beta Windows x64",
    "Status beta",
    "install-release-zip.ps1",
    "install-matter-beta.cmd",
    "matter-core-beta-setup.exe",
    "artifact-signature",
    "matter capabilities-json"
)) {
    if (-not $html.Contains($requiredText)) {
        throw "Download site HTML missing required content: $requiredText"
    }
}

$release = Get-Content $releasePath -Raw | ConvertFrom-Json
if ($release.format -ne "matter-download-site-release-v1") {
    throw "Download site release metadata has unexpected format"
}
if ($release.channel -ne "beta") {
    throw "Download site release metadata must use beta channel"
}
if ($release.status -ne "beta-ready") {
    throw "Download site release metadata must report beta-ready status"
}
if ($release.production_ready) {
    throw "Download site release metadata must not claim production readiness"
}
if (-not $release.version) {
    throw "Download site release metadata missing version"
}

$zipEntry = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1
if (-not $zipEntry) {
    throw "Download site release metadata missing zip artifact"
}

$setupEntry = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-beta-setup.exe" } | Select-Object -First 1
if (-not $setupEntry) {
    throw "Download site release metadata missing setup artifact"
}
if ($setupEntry.signed) {
    throw "Download site setup metadata must not claim signed=true in beta"
}

$zipPath = Join-Path $sitePath $zipEntry.path
$zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
$zipSize = (Get-Item $zipPath).Length
if ($zipHash -ne $zipEntry.sha256) {
    throw "Download site zip hash mismatch"
}
if ($zipSize -ne [int64]$zipEntry.size_bytes) {
    throw "Download site zip size mismatch"
}

$setupPath = Join-Path $sitePath $setupEntry.path
$setupHash = (Get-FileHash -LiteralPath $setupPath -Algorithm SHA256).Hash.ToLowerInvariant()
$setupSize = (Get-Item $setupPath).Length
if ($setupHash -ne $setupEntry.sha256) {
    throw "Download site setup hash mismatch"
}
if ($setupSize -ne [int64]$setupEntry.size_bytes) {
    throw "Download site setup size mismatch"
}

$checksums = Get-Content (Join-Path $sitePath "downloads\release-checksums.json") -Raw | ConvertFrom-Json
$checksumZip = @($checksums.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1
if (-not $checksumZip) {
    throw "Download site checksum JSON missing zip artifact"
}
if ($checksumZip.sha256 -ne $zipHash) {
    throw "Download site checksum JSON hash does not match copied zip"
}

foreach ($docName in @("BETA_NOTES.md", "TESTER_GUIDE.md")) {
    $docEntry = @($release.artifacts) | Where-Object { $_.name -eq $docName } | Select-Object -First 1
    if (-not $docEntry) {
        throw "Download site release metadata missing documentation artifact: $docName"
    }

    $docPath = Join-Path $sitePath $docEntry.path
    $docHash = (Get-FileHash -LiteralPath $docPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($docHash -ne $docEntry.sha256) {
        throw "Download site documentation hash mismatch: $docName"
    }
}

[ordered]@{
    ok = $true
    site_root = $sitePath
    checked = @(
        "site files exist",
        "download links exist",
        "release metadata parses",
        "zip hash matches metadata",
        "checksum JSON matches zip"
    )
} | ConvertTo-Json -Depth 4
