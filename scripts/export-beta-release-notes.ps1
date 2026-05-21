param(
    [string]$SiteReleasePath = "site\release.json",
    [string]$Out = "docs\releases\BETA_RELEASE_BODY.md",
    [string]$Tag = "v0.1.0-beta"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not (Test-Path $SiteReleasePath -PathType Leaf)) {
    throw "Site release metadata not found: $SiteReleasePath"
}

$release = Get-Content $SiteReleasePath -Raw | ConvertFrom-Json
if ($release.channel -ne "beta") {
    throw "Refusing to export beta release notes for non-beta channel: $($release.channel)"
}
if ($release.production_ready) {
    throw "Refusing to export beta release notes when production_ready=true"
}

$zip = @($release.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1
if (-not $zip) {
    throw "Release metadata missing matter-core-windows-x64.zip"
}

$installer = @($release.artifacts) | Where-Object { $_.name -eq "install-release-zip.ps1" } | Select-Object -First 1
$checksums = @($release.artifacts) | Where-Object { $_.name -eq "release-checksums.json" } | Select-Object -First 1
$sha256 = @($release.artifacts) | Where-Object { $_.name -eq "SHA256SUMS.txt" } | Select-Object -First 1

foreach ($artifact in @($installer, $checksums, $sha256)) {
    if (-not $artifact) {
        throw "Release metadata missing required beta artifact"
    }
}

$sizeMb = [math]::Round(([double]$zip.size_bytes / 1MB), 2)
$bodyLines = @(
    "# Matter Core $($release.version) Windows x64 Beta",
    "",
    "Status: **$($release.status)**  ",
    "Channel: **$($release.channel)**  ",
    "Production ready: **$($release.production_ready)**  ",
    "Tag: **$Tag**",
    "",
    "Matter Core beta is a controlled Windows x64 test release for developers who want to try the experimental Matter runtime, CLI, bytecode VM, JSON automation commands, and guarded reflection.",
    "",
    "## Download",
    "",
    "Download these files into the same folder:",
    "",
    "- matter-core-windows-x64.zip",
    "- install-release-zip.ps1",
    "- release-checksums.json",
    "- SHA256SUMS.txt",
    "",
    "## Install",
    "",
    "Run from PowerShell in the download folder:",
    "",
    "~~~powershell",
    [string]$release.install_command,
    "~~~",
    "",
    "Then open a new PowerShell window and run:",
    "",
    "~~~powershell",
    "matter run examples\first_run.matter",
    "matter capabilities-json",
    "~~~",
    "",
    "## Diagnose",
    "",
    "~~~powershell",
    'powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\diagnose-local-install.ps1"',
    "~~~",
    "",
    "## Uninstall",
    "",
    "~~~powershell",
    'powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\uninstall-local.ps1"',
    "~~~",
    "",
    "## Artifact Integrity",
    "",
    "- Package: $($zip.name)",
    "- Size: $sizeMb MB",
    "- SHA-256: $($zip.sha256)",
    "",
    "Installer artifact:",
    "",
    "- $($installer.name) SHA-256: $($installer.sha256)",
    "",
    "Checksum artifacts:",
    "",
    "- $($checksums.name) SHA-256: $($checksums.sha256)",
    "- $($sha256.name) SHA-256: $($sha256.sha256)",
    "",
    "## Feedback",
    "",
    "Use the GitHub issue template **Beta feedback**.",
    "",
    "Include:",
    "",
    "- Windows version.",
    "- PowerShell version.",
    "- Commands run.",
    "- Diagnosis output.",
    "- Any SmartScreen, antivirus, PATH, permission, or unzip warning.",
    "",
    "## Beta Limits",
    "",
    "This beta does **not** claim production readiness.",
    "",
    "Not included yet:",
    "",
    "- Signed .msi installer.",
    "- Code signing certificate.",
    "- Auto-update.",
    "- Linux/macOS installers.",
    "- General production support guarantee."
)

$body = $bodyLines -join [Environment]::NewLine

$parent = Split-Path -Parent $Out
if ($parent) {
    New-Item -ItemType Directory -Force $parent | Out-Null
}
Set-Content -Path $Out -Value $body -Encoding UTF8

[ordered]@{
    ok = $true
    out = [System.IO.Path]::GetFullPath($Out)
    tag = $Tag
    version = $release.version
    sha256 = $zip.sha256
} | ConvertTo-Json -Depth 4
