# Phase 4: Update Matter Core binaries/docs without deleting user projects.
param(
    [Parameter(Mandatory = $true)][string]$PackageRoot,
    [string]$InstallRoot = "",
    [switch]$SkipPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $InstallRoot) {
    if ($env:MATTER_HOME) { $InstallRoot = $env:MATTER_HOME }
    else { $InstallRoot = Join-Path $env:LOCALAPPDATA "Matter" }
}

if (-not (Test-Path -LiteralPath $InstallRoot)) {
    throw "Install root does not exist: $InstallRoot (run install-matter-core.ps1 first)"
}

# Seed a marker project to ensure preservation if tests need it
$proj = Join-Path $InstallRoot "projects"
New-Item -ItemType Directory -Force -Path $proj | Out-Null

$installScript = Join-Path $PSScriptRoot "install-matter-core.ps1"
if (-not (Test-Path -LiteralPath $installScript)) {
    $installScript = Join-Path $PackageRoot "scripts\install-matter-core.ps1"
}
$args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $installScript,
    "-PackageRoot", $PackageRoot, "-InstallRoot", $InstallRoot)
if ($SkipPath) { $args += "-SkipPath" }
& powershell @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Verify projects still exist
if (-not (Test-Path -LiteralPath $proj)) {
    throw "Update wiped projects directory"
}
Write-Host "Update OK (projects preserved): $InstallRoot" -ForegroundColor Green
exit 0
