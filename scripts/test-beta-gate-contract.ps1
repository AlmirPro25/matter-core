param(
    [string]$GatePath = "scripts\beta-gate.ps1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not (Test-Path $GatePath -PathType Leaf)) {
    throw "Beta gate script not found: $GatePath"
}

$gate = Get-Content $GatePath -Raw

foreach ($requiredText in @(
    "build-release-package.ps1",
    "build-windows-setup-exe.ps1",
    "build-download-site.ps1",
    "export-beta-release-notes.ps1",
    "test-release-package-contract.ps1",
    "test-release-artifact-checksums-contract.ps1",
    "test-release-zip-installer-contract.ps1",
    "test-windows-setup-exe-contract.ps1",
    "test-download-site-contract.ps1",
    "test-beta-readiness-contract.ps1",
    "test-beta-feedback-contract.ps1",
    "test-beta-release-notes-contract.ps1",
    "test-beta-site-workflow-contract.ps1",
    "cargo test -q",
    "site\release.json",
    "production_ready"
)) {
    if (-not $gate.Contains($requiredText)) {
        throw "Beta gate missing required content: $requiredText"
    }
}

$orderedMarkers = @(
    "build release package",
    "build windows setup exe",
    "build download site",
    "export beta release notes",
    "test release package contract",
    "test release checksum contract",
    "test release zip installer contract",
    "test windows setup exe contract",
    "test download site contract",
    "test beta readiness contract",
    "test beta feedback contract",
    "test beta release notes contract",
    "test beta site workflow contract"
)

$lastIndex = -1
foreach ($marker in $orderedMarkers) {
    $index = $gate.IndexOf($marker)
    if ($index -lt 0) {
        throw "Beta gate missing step marker: $marker"
    }
    if ($index -le $lastIndex) {
        throw "Beta gate step order is invalid at marker: $marker"
    }
    $lastIndex = $index
}

[ordered]@{
    ok = $true
    gate = $GatePath
    checked = @(
        "release build included",
        "download site build included",
        "package checksum install site beta workflow contracts included",
        "cargo tests included",
        "step order is valid"
    )
} | ConvertTo-Json -Depth 4
