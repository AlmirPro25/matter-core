param(
    [string[]]$ArtifactPaths = @("dist\matter-core-windows-x64.zip"),
    [string]$JsonOut = "dist\release-checksums.json",
    [string]$Sha256Out = "dist\SHA256SUMS.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$entries = foreach ($artifactPath in $ArtifactPaths) {
    if (-not (Test-Path $artifactPath -PathType Leaf)) {
        throw "Release artifact not found: $artifactPath"
    }

    $resolved = Resolve-Path $artifactPath
    $resolvedPath = [System.IO.Path]::GetFullPath($resolved.Path)
    $cwd = [System.IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\', '/')
    $displayPath = if ($resolvedPath.StartsWith($cwd, [System.StringComparison]::OrdinalIgnoreCase)) {
        $resolvedPath.Substring($cwd.Length).TrimStart('\', '/')
    }
    else {
        $artifactPath
    }
    $item = Get-Item -LiteralPath $resolved
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $resolved

    [ordered]@{
        path = $displayPath.Replace("/", "\")
        name = $item.Name
        size_bytes = $item.Length
        sha256 = $hash.Hash.ToLowerInvariant()
    }
}

$jsonParent = Split-Path -Parent $JsonOut
if ($jsonParent) {
    New-Item -ItemType Directory -Force $jsonParent | Out-Null
}

$shaParent = Split-Path -Parent $Sha256Out
if ($shaParent) {
    New-Item -ItemType Directory -Force $shaParent | Out-Null
}

[ordered]@{
    generated_at = (Get-Date).ToString("o")
    format = "matter-release-artifact-checksums-v1"
    artifact_count = @($entries).Count
    artifacts = @($entries)
} | ConvertTo-Json -Depth 5 | Set-Content -Path $JsonOut -Encoding UTF8

$lines = foreach ($entry in $entries) {
    "{0}  {1}" -f $entry.sha256, $entry.path
}
$lines | Set-Content -Path $Sha256Out -Encoding ASCII

[ordered]@{
    ok = $true
    json = [System.IO.Path]::GetFullPath($JsonOut)
    sha256 = [System.IO.Path]::GetFullPath($Sha256Out)
    artifacts = @($entries)
} | ConvertTo-Json -Depth 5
