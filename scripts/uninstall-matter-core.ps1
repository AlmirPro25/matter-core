# Phase 4: Uninstall only Matter-owned files (manifest-driven).
param(
    [string]$InstallRoot = "",
    [switch]$RemoveProjects,
    [switch]$RemoveFromPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $InstallRoot) {
    if ($env:MATTER_HOME) { $InstallRoot = $env:MATTER_HOME }
    else { $InstallRoot = Join-Path $env:LOCALAPPDATA "Matter" }
}

if (-not (Test-Path -LiteralPath $InstallRoot)) {
    Write-Host "Nothing to uninstall at $InstallRoot" -ForegroundColor Yellow
    exit 0
}
$InstallRoot = (Resolve-Path -LiteralPath $InstallRoot).Path
$manifestPath = Join-Path $InstallRoot ".matter-install-manifest.json"

$binDir = Join-Path $InstallRoot "bin"

if (Test-Path -LiteralPath $manifestPath) {
    $man = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    foreach ($rel in $man.owned_files) {
        $path = Join-Path $InstallRoot ($rel -replace '/', '\')
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force -Recurse -ErrorAction SilentlyContinue
        }
    }
    if (Test-Path -LiteralPath $manifestPath) {
        Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
    }
    Write-Host "Removed owned files from manifest" -ForegroundColor Green
} else {
    # Fallback: remove only known Matter product dirs (never wipe arbitrary InstallRoot content blindly except bin)
    foreach ($d in @("bin", "examples", "schemas", "docs", "scripts")) {
        $p = Join-Path $InstallRoot $d
        if (Test-Path -LiteralPath $p) {
            Remove-Item -LiteralPath $p -Recurse -Force
        }
    }
    foreach ($f in @("README.md", "INSTALL.txt", "LICENSE", "MANIFEST.json", "SHA256SUMS")) {
        $p = Join-Path $InstallRoot $f
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
    }
    Write-Host "Removed known Matter product files (no manifest)" -ForegroundColor Yellow
}

if ($RemoveProjects) {
    $p = Join-Path $InstallRoot "projects"
    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force }
    Write-Host "Removed projects (explicit -RemoveProjects)" -ForegroundColor Yellow
} else {
    $p = Join-Path $InstallRoot "projects"
    if (Test-Path -LiteralPath $p) {
        Write-Host "Preserved projects: $p" -ForegroundColor Cyan
    }
}

# Remove empty install root if nothing left
$left = Get-ChildItem -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue
if (-not $left -or $left.Count -eq 0) {
    Remove-Item -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue
}

if ($RemoveFromPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -and $userPath -like "*$binDir*") {
        $parts = $userPath.Split(';') | Where-Object { $_ -and ($_ -ne $binDir) }
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ';'), "User")
        Write-Host "Removed bin from User PATH" -ForegroundColor Green
    }
}

Write-Host "Uninstall complete: $InstallRoot" -ForegroundColor Green
exit 0
