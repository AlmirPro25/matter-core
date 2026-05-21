param(
    [string]$SiteRoot = "site",
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$ChecksumJsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

foreach ($required in @(
    $ZipPath,
    $ChecksumJsonPath,
    $Sha256Path,
    "scripts\install-release-zip.ps1",
    "scripts\diagnose-local-install.ps1",
    "scripts\uninstall-local.ps1",
    "docs\releases\BETA_READINESS.md",
    ".github\workflows\beta-site.yml",
    ".github\ISSUE_TEMPLATE\beta_feedback.yml",
    "scripts\beta-gate.ps1",
    "scripts\test-beta-gate-contract.ps1",
    "scripts\test-beta-feedback-contract.ps1",
    "scripts\export-beta-release-notes.ps1",
    "scripts\test-beta-release-notes-contract.ps1",
    "scripts\test-beta-site-workflow-contract.ps1",
    "docs\releases\BETA_RELEASE_BODY.md",
    (Join-Path $SiteRoot "index.html"),
    (Join-Path $SiteRoot "release.json"),
    (Join-Path $SiteRoot "BETA_NOTES.md"),
    (Join-Path $SiteRoot "TESTER_GUIDE.md")
)) {
    if (-not (Test-Path $required -PathType Leaf)) {
        throw "Beta readiness missing required file: $required"
    }
}

$checksums = Get-Content $ChecksumJsonPath -Raw | ConvertFrom-Json
if ($checksums.format -ne "matter-release-artifact-checksums-v1") {
    throw "Beta readiness checksum JSON has unexpected format"
}

$zipHash = (Get-FileHash -LiteralPath $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
$zipSize = (Get-Item $ZipPath).Length
$checksumZip = @($checksums.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1
if (-not $checksumZip) {
    throw "Beta readiness checksum JSON missing zip artifact"
}
if ($checksumZip.sha256 -ne $zipHash) {
    throw "Beta readiness checksum hash does not match zip"
}
if ([int64]$checksumZip.size_bytes -ne $zipSize) {
    throw "Beta readiness checksum size does not match zip"
}

$siteRelease = Get-Content (Join-Path $SiteRoot "release.json") -Raw | ConvertFrom-Json
if ($siteRelease.channel -ne "beta") {
    throw "Beta readiness site release channel must be beta"
}
if ($siteRelease.status -ne "beta-ready") {
    throw "Beta readiness site release status must be beta-ready"
}
if ($siteRelease.production_ready) {
    throw "Beta readiness must not claim production readiness"
}

$betaReadiness = Get-Content "docs\releases\BETA_READINESS.md" -Raw
foreach ($requiredText in @(
    "Status: beta-ready",
    "Not included yet",
    "The beta should not claim production readiness",
    "Static download site serves the zip",
    "GitHub Pages workflow builds and validates the beta site before deploy",
    "Beta gate contract passes",
    "Feedback channel is defined",
    "Release body is generated from beta metadata"
)) {
    if (-not $betaReadiness.Contains($requiredText)) {
        throw "Beta readiness doc missing required content: $requiredText"
    }
}

$betaNotes = Get-Content (Join-Path $SiteRoot "BETA_NOTES.md") -Raw
foreach ($requiredText in @(
    "Matter Core beta",
    "Install",
    "Diagnose",
    "Uninstall",
    "This is a beta"
)) {
    if (-not $betaNotes.Contains($requiredText)) {
        throw "Beta notes missing required content: $requiredText"
    }
}

[ordered]@{
    ok = $true
    checked = @(
        "release zip exists",
        "checksums match zip",
        "site release metadata is beta-ready",
        "beta readiness doc is honest",
        "beta notes include install diagnose uninstall"
    )
} | ConvertTo-Json -Depth 4
