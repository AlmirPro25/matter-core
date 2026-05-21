param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Matter",
    [switch]$NoPath,
    [switch]$SkipPostInstallCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$packageRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$binarySource = Join-Path $packageRoot "matter-cli.exe"

if (-not (Test-Path $binarySource -PathType Leaf)) {
    throw "Release package binary not found: $binarySource"
}

$binDir = Join-Path $InstallDir "bin"
$examplesDir = Join-Path $InstallDir "examples"
$docsDir = Join-Path $InstallDir "docs"
$scriptsDir = Join-Path $InstallDir "scripts"
$matterExe = Join-Path $binDir "matter.exe"
$matterCliExe = Join-Path $binDir "matter-cli.exe"
$pathModified = $false

Write-Host "Matter release local installer" -ForegroundColor Cyan
Write-Host "Installing to: $InstallDir" -ForegroundColor Cyan

New-Item -ItemType Directory -Force $binDir | Out-Null
New-Item -ItemType Directory -Force $examplesDir | Out-Null
New-Item -ItemType Directory -Force $docsDir | Out-Null
New-Item -ItemType Directory -Force $scriptsDir | Out-Null

Copy-Item -LiteralPath $binarySource -Destination $matterExe -Force
Copy-Item -LiteralPath $binarySource -Destination $matterCliExe -Force

foreach ($entry in @("README.md", "LANGUAGE_TOUR.md", "USER_ONBOARDING.md")) {
    $source = Join-Path $packageRoot $entry
    if (Test-Path $source -PathType Leaf) {
        Copy-Item -LiteralPath $source -Destination (Join-Path $InstallDir $entry) -Force
    }
}

foreach ($dir in @("examples", "docs", "schemas")) {
    $source = Join-Path $packageRoot $dir
    if (Test-Path $source -PathType Container) {
        Copy-Item -LiteralPath $source -Destination $InstallDir -Recurse -Force
    }
}

$uninstallSource = Join-Path $packageRoot "scripts\uninstall-local.ps1"
if (Test-Path $uninstallSource -PathType Leaf) {
    Copy-Item -LiteralPath $uninstallSource -Destination (Join-Path $scriptsDir "uninstall-local.ps1") -Force
}

$diagnoseSource = Join-Path $packageRoot "scripts\diagnose-local-install.ps1"
if (Test-Path $diagnoseSource -PathType Leaf) {
    Copy-Item -LiteralPath $diagnoseSource -Destination (Join-Path $scriptsDir "diagnose-local-install.ps1") -Force
}

if (-not $NoPath) {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @()
    if ($currentPath) {
        $pathEntries = $currentPath -split ';' | Where-Object { $_ }
    }

    if ($pathEntries -notcontains $binDir) {
        $newPath = if ($currentPath) { "$currentPath;$binDir" } else { $binDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $pathModified = $true
        Write-Host "Added to user PATH: $binDir" -ForegroundColor Green
    }
    else {
        Write-Host "User PATH already contains: $binDir" -ForegroundColor Gray
    }
}

if (-not $SkipPostInstallCheck) {
    $capabilitiesOutput = & $matterExe capabilities-json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter capabilities-json failed with exit code $LASTEXITCODE"
    }
    $capabilities = $capabilitiesOutput | ConvertFrom-Json
    if (-not $capabilities.ok) {
        throw "Installed matter capabilities-json did not report ok=true"
    }
}

$sourceHash = (Get-FileHash -LiteralPath $binarySource -Algorithm SHA256).Hash.ToLowerInvariant()
$matterHash = (Get-FileHash -LiteralPath $matterExe -Algorithm SHA256).Hash.ToLowerInvariant()
$matterCliHash = (Get-FileHash -LiteralPath $matterCliExe -Algorithm SHA256).Hash.ToLowerInvariant()

$manifest = [ordered]@{
    schema = "matter.release.install.v1"
    installed_at = (Get-Date).ToString("o")
    install_dir = [System.IO.Path]::GetFullPath($InstallDir)
    package_root = $packageRoot.Path
    version = "0.1.0"
    path_modified = $pathModified
    post_install_check = if ($SkipPostInstallCheck) { "skipped" } else { "capabilities-json" }
    source_binary = [ordered]@{
        path = "matter-cli.exe"
        sha256 = $sourceHash
    }
    installed_binaries = @(
        [ordered]@{
            path = "bin\matter.exe"
            sha256 = $matterHash
        },
        [ordered]@{
            path = "bin\matter-cli.exe"
            sha256 = $matterCliHash
        }
    )
    commands = @(
        "matter --help",
        "matter run examples\first_run.matter",
        "matter capabilities-json"
    )
}

$manifest | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $InstallDir "INSTALL_MANIFEST.json") -Encoding UTF8

$info = @"
Matter Core release install
Installed at: $InstallDir

Commands:
  matter --help
  matter run "$examplesDir\first_run.matter"
  matter capabilities-json

Diagnose:
  powershell -ExecutionPolicy Bypass -File "$scriptsDir\diagnose-local-install.ps1" -InstallDir "$InstallDir"

Uninstall:
  powershell -ExecutionPolicy Bypass -File "$scriptsDir\uninstall-local.ps1" -InstallDir "$InstallDir"
"@

Set-Content -Path (Join-Path $InstallDir "INFO.txt") -Value $info -Encoding UTF8

Write-Host ""
Write-Host "Matter installed for the current user." -ForegroundColor Green
Write-Host "Open a new terminal and run: matter --help" -ForegroundColor Yellow
