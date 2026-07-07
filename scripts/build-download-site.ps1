param(
    [string]$SiteRoot = "site",
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$ChecksumJsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt",
    [string]$StatusTriadHealthPath = "target\validation\status-triad-beta-health.json",
    [string]$StatusTriadTrendPath = "target\validation\status-triad-beta-trend-report.json",
    [string]$StatusTriadHistoryPath = "target\validation\status-triad-beta-history.ndjson",
    [string]$Version = "0.1.0-beta",
    [string]$Channel = "beta"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

foreach ($path in @($ZipPath, $ChecksumJsonPath, $Sha256Path, "dist\matter-core-beta-setup.exe", "scripts\install-release-zip.ps1", "scripts\install-matter-beta.cmd", "site\BETA_NOTES.md", "site\TESTER_GUIDE.md")) {
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Required download site input not found: $path"
    }
}

$resolvedHealthPath = $StatusTriadHealthPath
if (-not (Test-Path $resolvedHealthPath -PathType Leaf)) {
    if ($StatusTriadHealthPath -eq "target\validation\status-triad-beta-health.json" -and (Test-Path "target\validation\status-triad-health.json" -PathType Leaf)) {
        $resolvedHealthPath = "target\validation\status-triad-health.json"
    }
    else {
        throw "Required status triad health input not found: $StatusTriadHealthPath"
    }
}

$resolvedTrendPath = $StatusTriadTrendPath
if (-not (Test-Path $resolvedTrendPath -PathType Leaf)) {
    if ($StatusTriadTrendPath -eq "target\validation\status-triad-beta-trend-report.json" -and (Test-Path "target\validation\status-triad-trend-report.json" -PathType Leaf)) {
        $resolvedTrendPath = "target\validation\status-triad-trend-report.json"
    }
    else {
        throw "Required status triad trend input not found: $StatusTriadTrendPath"
    }
}

$resolvedHistoryPath = $StatusTriadHistoryPath
if (-not (Test-Path $resolvedHistoryPath -PathType Leaf)) {
    if ($StatusTriadHistoryPath -eq "target\validation\status-triad-beta-history.ndjson" -and (Test-Path "target\validation\status-triad-history.ndjson" -PathType Leaf)) {
        $resolvedHistoryPath = "target\validation\status-triad-history.ndjson"
    }
    else {
        throw "Required status triad history input not found: $StatusTriadHistoryPath"
    }
}

$sitePath = [System.IO.Path]::GetFullPath($SiteRoot)
$downloadsPath = Join-Path $sitePath "downloads"
New-Item -ItemType Directory -Force $downloadsPath | Out-Null

Copy-Item -LiteralPath $ZipPath -Destination (Join-Path $downloadsPath "matter-core-windows-x64.zip") -Force
Copy-Item -LiteralPath "dist\matter-core-beta-setup.exe" -Destination (Join-Path $downloadsPath "matter-core-beta-setup.exe") -Force
Copy-Item -LiteralPath $ChecksumJsonPath -Destination (Join-Path $downloadsPath "release-checksums.json") -Force
Copy-Item -LiteralPath $Sha256Path -Destination (Join-Path $downloadsPath "SHA256SUMS.txt") -Force
Copy-Item -LiteralPath "scripts\install-release-zip.ps1" -Destination (Join-Path $downloadsPath "install-release-zip.ps1") -Force
Copy-Item -LiteralPath "scripts\install-matter-beta.cmd" -Destination (Join-Path $downloadsPath "install-matter-beta.cmd") -Force
Copy-Item -LiteralPath $resolvedHealthPath -Destination (Join-Path $downloadsPath "status-triad-health.json") -Force
Copy-Item -LiteralPath $resolvedTrendPath -Destination (Join-Path $downloadsPath "status-triad-trend-report.json") -Force
Copy-Item -LiteralPath $resolvedHistoryPath -Destination (Join-Path $downloadsPath "status-triad-history.ndjson") -Force

$checksums = Get-Content $ChecksumJsonPath -Raw | ConvertFrom-Json
$artifact = @($checksums.artifacts) | Where-Object { $_.name -eq "matter-core-windows-x64.zip" } | Select-Object -First 1
if (-not $artifact) {
    throw "Checksum JSON missing matter-core-windows-x64.zip"
}
$triadHealth = Get-Content -Path $resolvedHealthPath -Raw | ConvertFrom-Json
$triadTrend = Get-Content -Path $resolvedTrendPath -Raw | ConvertFrom-Json

$siteRelease = [ordered]@{
    generated_at = $checksums.generated_at
    format = "matter-download-site-release-v1"
    version = $Version
    channel = $Channel
    status = "beta-ready"
    production_ready = $false
    install_command = ".\matter-core-beta-setup.exe"
    runtime_health_summary = [ordered]@{
        status = [string]$triadHealth.status
        max_p95_ms = [double]$triadHealth.summary.max_p95_ms
        window_samples = [int]$triadHealth.summary.window_samples
        total_samples = [int]$triadHealth.summary.total_samples
        source = "downloads/status-triad-health.json"
    }
    runtime_health_thresholds = [ordered]@{
        warn_p95_ms = [double]$triadHealth.thresholds.warn_p95_ms
        fail_p95_ms = [double]$triadHealth.thresholds.fail_p95_ms
    }
    runtime_trend_summary = [ordered]@{
        window_samples = [int]$triadTrend.window_samples
        total_samples = [int]$triadTrend.total_samples
        core_p95_ms = [double]$triadTrend.triad.core.p95_ms
        world_p95_ms = [double]$triadTrend.triad.world.p95_ms
        frontier_p95_ms = [double]$triadTrend.triad.frontier.p95_ms
        source = "downloads/status-triad-trend-report.json"
    }
    artifacts = @(
        [ordered]@{
            name = $artifact.name
            path = "downloads/matter-core-windows-x64.zip"
            size_bytes = [int64]$artifact.size_bytes
            sha256 = [string]$artifact.sha256
        },
        [ordered]@{
            name = "matter-core-beta-setup.exe"
            path = "downloads/matter-core-beta-setup.exe"
            size_bytes = (Get-Item (Join-Path $downloadsPath "matter-core-beta-setup.exe")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "matter-core-beta-setup.exe") -Algorithm SHA256).Hash.ToLowerInvariant()
            signed = $false
        },
        [ordered]@{
            name = "install-release-zip.ps1"
            path = "downloads/install-release-zip.ps1"
            size_bytes = (Get-Item (Join-Path $downloadsPath "install-release-zip.ps1")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "install-release-zip.ps1") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "install-matter-beta.cmd"
            path = "downloads/install-matter-beta.cmd"
            size_bytes = (Get-Item (Join-Path $downloadsPath "install-matter-beta.cmd")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "install-matter-beta.cmd") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "release-checksums.json"
            path = "downloads/release-checksums.json"
            size_bytes = (Get-Item (Join-Path $downloadsPath "release-checksums.json")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "release-checksums.json") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "SHA256SUMS.txt"
            path = "downloads/SHA256SUMS.txt"
            size_bytes = (Get-Item (Join-Path $downloadsPath "SHA256SUMS.txt")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "SHA256SUMS.txt") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "status-triad-health.json"
            path = "downloads/status-triad-health.json"
            size_bytes = (Get-Item (Join-Path $downloadsPath "status-triad-health.json")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "status-triad-health.json") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "status-triad-trend-report.json"
            path = "downloads/status-triad-trend-report.json"
            size_bytes = (Get-Item (Join-Path $downloadsPath "status-triad-trend-report.json")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "status-triad-trend-report.json") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "status-triad-history.ndjson"
            path = "downloads/status-triad-history.ndjson"
            size_bytes = (Get-Item (Join-Path $downloadsPath "status-triad-history.ndjson")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $downloadsPath "status-triad-history.ndjson") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "TESTER_GUIDE.md"
            path = "TESTER_GUIDE.md"
            size_bytes = (Get-Item (Join-Path $sitePath "TESTER_GUIDE.md")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $sitePath "TESTER_GUIDE.md") -Algorithm SHA256).Hash.ToLowerInvariant()
        },
        [ordered]@{
            name = "BETA_NOTES.md"
            path = "BETA_NOTES.md"
            size_bytes = (Get-Item (Join-Path $sitePath "BETA_NOTES.md")).Length
            sha256 = (Get-FileHash -LiteralPath (Join-Path $sitePath "BETA_NOTES.md") -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    )
}

$siteRelease | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $sitePath "release.json") -Encoding UTF8

[ordered]@{
    ok = $true
    site_root = $sitePath
    downloads = @($siteRelease.artifacts | ForEach-Object { $_.path })
} | ConvertTo-Json -Depth 4
