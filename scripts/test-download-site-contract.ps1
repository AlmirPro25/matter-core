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
    (Join-Path $sitePath "downloads\status-triad-health.json"),
    (Join-Path $sitePath "downloads\status-triad-trend-report.json"),
    (Join-Path $sitePath "downloads\status-triad-history.ndjson"),
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
    "downloads/status-triad-health.json",
    "downloads/status-triad-trend-report.json",
    "downloads/status-triad-history.ndjson",
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
if (-not $release.PSObject.Properties["runtime_health_summary"]) {
    throw "Download site release metadata missing runtime_health_summary"
}
$runtimeHealthThresholds = $null
if ($release.PSObject.Properties["runtime_health_thresholds"]) {
    $runtimeHealthThresholds = $release.PSObject.Properties["runtime_health_thresholds"].Value
}
$runtimeHealth = $release.PSObject.Properties["runtime_health_summary"].Value
if (@("pass", "warn", "fail") -notcontains [string]$runtimeHealth.status) {
    throw "Download site runtime_health_summary.status must be pass, warn, or fail"
}
if ([double]$runtimeHealth.max_p95_ms -lt 0) {
    throw "Download site runtime_health_summary.max_p95_ms must be >= 0"
}
if ([int]$runtimeHealth.window_samples -lt 1) {
    throw "Download site runtime_health_summary.window_samples must be >= 1"
}
if ([int]$runtimeHealth.total_samples -lt 1) {
    throw "Download site runtime_health_summary.total_samples must be >= 1"
}
if ([string]$runtimeHealth.source -ne "downloads/status-triad-health.json") {
    throw "Download site runtime_health_summary.source mismatch"
}
if ($runtimeHealthThresholds) {
    if ([double]$runtimeHealthThresholds.warn_p95_ms -le 0 -or [double]$runtimeHealthThresholds.fail_p95_ms -le 0) {
        throw "Download site runtime_health_thresholds must be > 0"
    }
}
if (-not $release.PSObject.Properties["runtime_trend_summary"]) {
    throw "Download site release metadata missing runtime_trend_summary"
}
$runtimeTrend = $release.PSObject.Properties["runtime_trend_summary"].Value
if ([int]$runtimeTrend.window_samples -lt 1 -or [int]$runtimeTrend.total_samples -lt 1) {
    throw "Download site runtime_trend_summary sample counts are invalid"
}
foreach ($metric in @("core_p95_ms", "world_p95_ms", "frontier_p95_ms")) {
    if ([double]$runtimeTrend.$metric -lt 0) {
        throw "Download site runtime_trend_summary.$metric must be >= 0"
    }
}
if ([string]$runtimeTrend.source -ne "downloads/status-triad-trend-report.json") {
    throw "Download site runtime_trend_summary.source mismatch"
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

$healthEntry = @($release.artifacts) | Where-Object { $_.name -eq "status-triad-health.json" } | Select-Object -First 1
if (-not $healthEntry) {
    throw "Download site release metadata missing status triad health artifact"
}
$healthPath = Join-Path $sitePath $healthEntry.path
$healthHash = (Get-FileHash -LiteralPath $healthPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($healthHash -ne $healthEntry.sha256) {
    throw "Download site triad health hash mismatch"
}
$healthPayload = Get-Content -Path $healthPath -Raw | ConvertFrom-Json
if (@("pass", "warn", "fail") -notcontains [string]$healthPayload.status) {
    throw "Download site triad health status must be pass, warn, or fail"
}
if ([string]$runtimeHealth.status -ne [string]$healthPayload.status) {
    throw "Download site runtime_health_summary.status does not match triad health artifact"
}
$trendEntry = @($release.artifacts) | Where-Object { $_.name -eq "status-triad-trend-report.json" } | Select-Object -First 1
if (-not $trendEntry) {
    throw "Download site release metadata missing status triad trend artifact"
}
$trendPath = Join-Path $sitePath $trendEntry.path
$trendHash = (Get-FileHash -LiteralPath $trendPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($trendHash -ne $trendEntry.sha256) {
    throw "Download site triad trend hash mismatch"
}
$trendPayload = Get-Content -Path $trendPath -Raw | ConvertFrom-Json
if ([double]$runtimeTrend.core_p95_ms -ne [double]$trendPayload.triad.core.p95_ms) {
    throw "Download site runtime_trend_summary.core_p95_ms mismatch"
}
$historyEntry = @($release.artifacts) | Where-Object { $_.name -eq "status-triad-history.ndjson" } | Select-Object -First 1
if (-not $historyEntry) {
    throw "Download site release metadata missing status triad history artifact"
}
$historyPath = Join-Path $sitePath $historyEntry.path
$historyHash = (Get-FileHash -LiteralPath $historyPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($historyHash -ne $historyEntry.sha256) {
    throw "Download site triad history hash mismatch"
}
$historyLines = @(Get-Content -Path $historyPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if ($historyLines.Count -lt 1) {
    throw "Download site triad history must contain at least one sample line"
}

# Fallback readiness: if artifact is unavailable, runtime_health_summary still provides valid status data.
$runtimeHealthStatus = [string]$runtimeHealth.status
$runtimeHealthMaxP95 = [double]$runtimeHealth.max_p95_ms
$runtimeHealthWindowSamples = [int]$runtimeHealth.window_samples
$runtimeHealthTotalSamples = [int]$runtimeHealth.total_samples

$healthBackupPath = "$healthPath.bak"
Copy-Item -LiteralPath $healthPath -Destination $healthBackupPath -Force
try {
    Remove-Item -LiteralPath $healthPath -Force
    if (-not [string]::IsNullOrWhiteSpace($runtimeHealthStatus) -and $runtimeHealthMaxP95 -ge 0 -and $runtimeHealthWindowSamples -ge 1 -and $runtimeHealthTotalSamples -ge 1) {
        # pass
    }
    else {
        throw "Download site runtime_health_summary cannot support fallback mode"
    }
}
finally {
    if (Test-Path $healthBackupPath -PathType Leaf) {
        Move-Item -LiteralPath $healthBackupPath -Destination $healthPath -Force
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
