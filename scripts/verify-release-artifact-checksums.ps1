param(
    [string]$JsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path $JsonPath -PathType Leaf)) {
    throw "Release checksum JSON not found: $JsonPath"
}
if (-not (Test-Path $Sha256Path -PathType Leaf)) {
    throw "Release SHA256SUMS file not found: $Sha256Path"
}

$checksumDoc = Get-Content $JsonPath -Raw | ConvertFrom-Json
if ($checksumDoc.format -ne "matter-release-artifact-checksums-v1") {
    throw "Unexpected release checksum format: $($checksumDoc.format)"
}

$artifacts = @($checksumDoc.artifacts)
if ($checksumDoc.artifact_count -ne $artifacts.Count) {
    throw "Checksum artifact_count does not match artifact list"
}
if ($artifacts.Count -eq 0) {
    throw "Checksum document contains no artifacts"
}

$shaLines = Get-Content $Sha256Path | Where-Object { $_.Trim() }
$shaByPath = @{}
foreach ($line in $shaLines) {
    if ($line -notmatch '^([a-fA-F0-9]{64})\s+(.+)$') {
        throw "Invalid SHA256SUMS line: $line"
    }
    $shaByPath[$matches[2]] = $matches[1].ToLowerInvariant()
}

foreach ($artifact in $artifacts) {
    $path = [string]$artifact.path
    if (-not $path) {
        throw "Checksum artifact missing path"
    }
    if ([System.IO.Path]::IsPathRooted($path)) {
        throw "Checksum artifact path must be relative: $path"
    }
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Checksum artifact not found: $path"
    }

    $item = Get-Item -LiteralPath $path
    if ([int64]$artifact.size_bytes -ne $item.Length) {
        throw "Checksum artifact size mismatch for $path"
    }

    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    $expected = ([string]$artifact.sha256).ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "Checksum JSON hash mismatch for $path"
    }
    if (-not $shaByPath.ContainsKey($path)) {
        throw "SHA256SUMS missing artifact path: $path"
    }
    if ($shaByPath[$path] -ne $expected) {
        throw "SHA256SUMS hash does not match JSON for $path"
    }
}

[ordered]@{
    ok = $true
    json = [System.IO.Path]::GetFullPath($JsonPath)
    sha256 = [System.IO.Path]::GetFullPath($Sha256Path)
    checked_artifacts = @($artifacts).Count
    artifacts = @($artifacts | ForEach-Object { $_.path })
} | ConvertTo-Json -Depth 4
