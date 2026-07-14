# Phase 5: package twice and compare content hashes (not necessarily ZIP bytes).
param([string]$CliPath = "")

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Out = Join-Path $Root "target\validation\production_readiness_v2"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
if (-not $CliPath) {
    $CliPath = Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
}

function Build-Package([string]$destName) {
    $dest = Join-Path $Root "dist\$destName"
    if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
    # Invoke packager; swallow stdout so only explicit return is captured
    $null = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Root "scripts\package-matter-core.ps1") `
        -CliPath $CliPath -SkipBuild -Version "0.1.0-repro" -OutDir $dest 2>&1
    if ($LASTEXITCODE -ne 0) { throw "package failed for $destName" }
    if (-not (Test-Path -LiteralPath $dest)) { throw "package dir missing: $dest" }
    return $dest
}

# Two package directory builds (same CLI binary) — compare file set + per-file SHA256
$a = Build-Package "repro-a"
$b = Build-Package "repro-b"

function Get-FileMap([string]$root) {
    $map = @{}
    Get-ChildItem -LiteralPath $root -Recurse | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
        $rel = $_.FullName.Substring($root.Length).TrimStart('\', '/').Replace('\', '/')
        # Skip MANIFEST timestamps / SHA256SUMS that embed rebuild times
        if ($rel -eq "MANIFEST.json" -or $rel -eq "SHA256SUMS") { return }
        $h = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash
        $map[$rel] = @{ sha256 = $h; bytes = $_.Length }
    }
    return $map
}

$mapA = Get-FileMap $a
$mapB = Get-FileMap $b
$keysA = @($mapA.Keys | Sort-Object)
$keysB = @($mapB.Keys | Sort-Object)
$setEqual = ($keysA -join "|") -eq ($keysB -join "|")
$contentEqual = $true
$diffs = @()
if ($setEqual) {
    foreach ($k in $keysA) {
        if ($mapA[$k].sha256 -ne $mapB[$k].sha256) {
            $contentEqual = $false
            $diffs += $k
        }
    }
} else {
    $contentEqual = $false
    $diffs += "file-set-mismatch"
}

# ZIP byte identity (often differs due to timestamps in zip central directory)
$zipA = Join-Path $Root "dist\matter-core-0.1.0-repro-windows-x64.zip"
# packager always writes versioned zip name from -Version
$zipA = Join-Path $Root "dist\matter-core-0.1.0-repro-windows-x64.zip"
# Our package script uses $Version in name; second build overwrites same zip.
# So capture zip hash after first build by re-packaging with distinct version labels already done.
# Compare only directory content for logical repro; zip from sequential overwrites.
$zipPath = Join-Path $Root "dist\matter-core-0.1.0-repro-windows-x64.zip"
$zipHash = if (Test-Path $zipPath) { (Get-FileHash -Algorithm SHA256 $zipPath).Hash } else { $null }

$verdict = if ($contentEqual -and $setEqual) { "PASS_LOGICAL" } else { "FAIL" }
$report = [pscustomobject]@{
    at = (Get-Date).ToString("o")
    verdict = $verdict
    file_set_equal = $setEqual
    content_sha_equal = $contentEqual
    diffs = $diffs
    files_a = $keysA.Count
    files_b = $keysB.Count
    zip_note = "ZIP central directory timestamps make byte-identical ZIPs unreliable on Windows Compress/ZipFile without fixed entry times; logical content (per-file SHA-256 excluding MANIFEST/SHA256SUMS timestamps) is the RC criterion."
    last_zip_sha256 = $zipHash
    dirs = @{ a = $a; b = $b }
}
$report | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $Out "repro-compare.json") -Encoding utf8
Write-Host "REPRO verdict=$verdict content_equal=$contentEqual"
if ($verdict -eq "FAIL") { exit 1 }
exit 0
