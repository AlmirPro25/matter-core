param(
    [string]$ReleaseNotesPath = "docs\releases\BETA_RELEASE_BODY.md",
    [string]$SiteReleasePath = "site\release.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

foreach ($required in @($ReleaseNotesPath, $SiteReleasePath)) {
    if (-not (Test-Path $required -PathType Leaf)) {
        throw "Beta release notes contract missing required file: $required"
    }
}

$release = Get-Content $SiteReleasePath -Raw | ConvertFrom-Json
$notes = Get-Content $ReleaseNotesPath -Raw
$zip = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1

foreach ($requiredText in @(
    "Matter Core $($release.version) Windows x64 Beta",
    "Status: **$($release.status)**",
    "Channel: **$($release.channel)**",
    "Production ready: **False**",
    "matter-core-windows-x64.zip",
    "install-release-zip.ps1",
    "release-checksums.json",
    "SHA256SUMS.txt",
    $release.install_command,
    "~~~powershell",
    "matter capabilities-json",
    "diagnose-local-install.ps1",
    "uninstall-local.ps1",
    $zip.sha256,
    "Beta feedback",
    "This beta does **not** claim production readiness",
    "Signed .msi installer"
)) {
    if (-not $notes.Contains($requiredText)) {
        throw "Beta release notes missing required content: $requiredText"
    }
}

foreach ($forbiddenText in @(
    '$(@{',
    '$sizeMb',
    '`powershell',
    "`nelease-checksums.json"
)) {
    if ($notes.Contains($forbiddenText)) {
        throw "Beta release notes contain unresolved or malformed Markdown content: $forbiddenText"
    }
}

if ($notes.Contains("production guarantee") -and -not $notes.Contains("does **not** claim production readiness")) {
    throw "Beta release notes contain an unsafe production implication"
}

[ordered]@{
    ok = $true
    checked = @(
        "release notes exist",
        "release notes match site metadata",
        "install diagnose uninstall instructions present",
        "artifact hashes present",
        "feedback path present",
        "production limits present"
    )
} | ConvertTo-Json -Depth 4
