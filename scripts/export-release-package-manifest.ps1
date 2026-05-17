param(
    [string]$PackageRoot = "dist\matter-core-windows-x64",
    [string]$Out,
    [switch]$SkipBinary
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path $PackageRoot -PathType Container)) {
    throw "Release package root not found: $PackageRoot"
}

$resolvedPackageRoot = Resolve-Path $PackageRoot
if (-not $Out) {
    $Out = Join-Path $resolvedPackageRoot "target\ffi\release-package-manifest.json"
}

$outParent = Split-Path -Parent $Out
if ($outParent) {
    New-Item -ItemType Directory -Force $outParent | Out-Null
}

$resolvedOut = if (Test-Path $Out -PathType Leaf) {
    (Resolve-Path $Out).Path
}
else {
    [System.IO.Path]::GetFullPath($Out)
}

$rootPath = $resolvedPackageRoot.Path.TrimEnd('\', '/')
$files = Get-ChildItem $rootPath -Recurse -File |
    Where-Object { [System.IO.Path]::GetFullPath($_.FullName) -ne $resolvedOut } |
    Sort-Object FullName

if (-not $SkipBinary) {
    $binaryPath = Join-Path $rootPath "matter-cli.exe"
    if (-not (Test-Path $binaryPath -PathType Leaf)) {
        throw "Release package missing binary: matter-cli.exe"
    }
}

$entries = foreach ($file in $files) {
    $fullPath = [System.IO.Path]::GetFullPath($file.FullName)
    $relativePath = $fullPath.Substring($rootPath.Length).TrimStart('\', '/').Replace('/', '\')
    if ([System.IO.Path]::IsPathRooted($relativePath)) {
        throw "Manifest entry resolved to absolute path: $relativePath"
    }

    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName
    [ordered]@{
        path        = $relativePath
        size_bytes  = $file.Length
        sha256      = $hash.Hash.ToLowerInvariant()
    }
}

[ordered]@{
    generated_at       = (Get-Date).ToString("o")
    package_root_name  = Split-Path -Leaf $rootPath
    format             = "matter-release-package-manifest-v1"
    file_count         = @($entries).Count
    files              = @($entries)
} | ConvertTo-Json -Depth 6 | Set-Content -Path $Out -Encoding UTF8

Write-Host "Release package manifest exported to $Out"
