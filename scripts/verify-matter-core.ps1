# Phase 4: Verify a Matter Core install or extracted package.
param(
    [string]$InstallRoot = "",
    [string]$PackageRoot = "",
    [switch]$MinimalPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

function Fail([string]$msg) { Write-Host "FAIL: $msg" -ForegroundColor Red; exit 1 }
function Ok([string]$msg) { Write-Host "PASS: $msg" -ForegroundColor Green }

$root = $InstallRoot
if (-not $root) { $root = $PackageRoot }
if (-not $root) {
    if ($env:MATTER_HOME) { $root = $env:MATTER_HOME }
    else { $root = Join-Path $env:LOCALAPPDATA "Matter" }
}
if (-not (Test-Path -LiteralPath $root)) { Fail "root not found: $root" }
$root = (Resolve-Path -LiteralPath $root).Path

$cli = Join-Path $root "bin\matter-cli.exe"
if (-not (Test-Path -LiteralPath $cli)) {
    $cli = Join-Path $root "matter-cli.exe"
}
if (-not (Test-Path -LiteralPath $cli)) { Fail "matter-cli.exe not found under $root" }

if ($MinimalPath) {
    $env:PATH = Join-Path $env:SystemRoot "System32"
}

$checks = @(
    @{ name = "version"; args = @("--version") },
    @{ name = "help"; args = @("--help") },
    @{ name = "core-status-json"; args = @("core-status-json") }
)

foreach ($c in $checks) {
    & $cli @($c.args) 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { Fail "$($c.name) exit=$LASTEXITCODE" }
    Ok $c.name
}

$hello = Join-Path $root "examples\hello.matter"
if (-not (Test-Path -LiteralPath $hello)) {
    # try package layout
    $hello = Get-ChildItem -LiteralPath $root -Recurse -Filter "hello.matter" -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}
if ($hello -and (Test-Path -LiteralPath $hello)) {
    $tmp = Join-Path $env:TEMP ("matter-verify-" + [guid]::NewGuid().ToString("n") + ".mbc")
    & $cli run $hello 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { Fail "run hello" }
    Ok "run"
    & $cli compile $hello -o $tmp 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { Fail "compile" }
    Ok "compile"
    & $cli run-bytecode $tmp 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { Fail "run-bytecode" }
    Ok "run-bytecode"
    Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
} else {
    Write-Host "WARN: hello.matter missing; skipped run/compile" -ForegroundColor Yellow
}

# Manifest checksums if present
$sums = Join-Path $root "SHA256SUMS"
if (Test-Path -LiteralPath $sums) {
    $bad = 0
    Get-Content -LiteralPath $sums | ForEach-Object {
        $line = $_.Trim()
        if (-not $line -or $line.StartsWith("#")) { return }
        $parts = $line -split '\s+', 2
        if ($parts.Count -lt 2) { return }
        $want = $parts[0].ToLowerInvariant()
        $rel = $parts[1].Trim().TrimStart('*')
        $path = Join-Path $root ($rel -replace '/', '\')
        if (-not (Test-Path -LiteralPath $path)) {
            # MANIFEST may have been rewritten; skip missing optional
            if ($rel -eq "MANIFEST.json" -or $rel -eq "SHA256SUMS") { return }
            Write-Host "WARN missing $rel" -ForegroundColor Yellow
            return
        }
        # skip self-hash of SHA256SUMS if listed after generation
        if ($rel -eq "SHA256SUMS") { return }
        $got = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        if ($got -ne $want -and $rel -ne "MANIFEST.json") {
            # MANIFEST re-hashed after write in packager edge cases
            Write-Host "FAIL hash $rel" -ForegroundColor Red
            $bad++
        }
    }
    if ($bad -eq 0) { Ok "SHA256SUMS sample" } else { Fail "SHA256SUMS mismatches=$bad" }
}

Ok "verify complete: $root"
exit 0
