# Phase 4: Install portable Matter Core package to any path (no admin required by default).
# Preserves <InstallRoot>\projects and <InstallRoot>\user on reinstall/update.
param(
    [string]$PackageRoot = "",
    [string]$InstallRoot = "",
    [switch]$SkipPath,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-DefaultInstallRoot {
    if ($env:MATTER_HOME -and $env:MATTER_HOME.Trim()) {
        return $env:MATTER_HOME.Trim()
    }
    return (Join-Path $env:LOCALAPPDATA "Matter")
}

if (-not $PackageRoot) {
    # When run from inside a package: scripts/ is under package root
    $here = $PSScriptRoot
    if (Test-Path -LiteralPath (Join-Path $here "..\bin\matter-cli.exe")) {
        $PackageRoot = (Resolve-Path -LiteralPath (Join-Path $here "..")).Path
    } else {
        throw "Pass -PackageRoot to the extracted package directory."
    }
} else {
    $PackageRoot = (Resolve-Path -LiteralPath $PackageRoot).Path
}

if (-not $InstallRoot) {
    $InstallRoot = Resolve-DefaultInstallRoot
}

$pkgBin = Join-Path $PackageRoot "bin\matter-cli.exe"
if (-not (Test-Path -LiteralPath $pkgBin)) {
    # also allow package root = flat layout with matter-cli.exe
    if (Test-Path -LiteralPath (Join-Path $PackageRoot "matter-cli.exe")) {
        $pkgBin = Join-Path $PackageRoot "matter-cli.exe"
    } else {
        throw "matter-cli.exe not found under package: $PackageRoot"
    }
}

Write-Host "Package: $PackageRoot" -ForegroundColor Cyan
Write-Host "Install: $InstallRoot" -ForegroundColor Cyan

# Preserve user data
$preserve = @("projects", "user", "workspace")
$backup = Join-Path $env:TEMP ("matter-preserve-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Force -Path $backup | Out-Null
if (Test-Path -LiteralPath $InstallRoot) {
    foreach ($d in $preserve) {
        $p = Join-Path $InstallRoot $d
        if (Test-Path -LiteralPath $p) {
            Copy-Item -LiteralPath $p -Destination (Join-Path $backup $d) -Recurse -Force
            Write-Host "  preserved $d" -ForegroundColor Gray
        }
    }
}

$binDir = Join-Path $InstallRoot "bin"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

# Copy package contents except scripts may re-copy
$owned = New-Object System.Collections.Generic.List[string]

function Copy-Owned([string]$Src, [string]$DstRel) {
    $dst = Join-Path $InstallRoot $DstRel
    $parent = Split-Path -Parent $dst
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    if (Test-Path -LiteralPath $Src -PathType Container) {
        Copy-Item -LiteralPath $Src -Destination $dst -Recurse -Force
        Get-ChildItem -LiteralPath $dst -Recurse -File | ForEach-Object {
            $rel = $_.FullName.Substring($InstallRoot.Length).TrimStart('\', '/')
            $script:owned.Add(($rel -replace '\\', '/'))
        }
    } else {
        Copy-Item -LiteralPath $Src -Destination $dst -Force
        $script:owned.Add(($DstRel -replace '\\', '/'))
    }
}

# Prefer structured package layout
$pkgBinDir = Join-Path $PackageRoot "bin"
if (Test-Path -LiteralPath $pkgBinDir -PathType Container) {
    Get-ChildItem -LiteralPath $pkgBinDir -File | ForEach-Object {
        $dest = Join-Path $binDir $_.Name
        Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
        $owned.Add(("bin/" + $_.Name))
    }
} else {
    Copy-Item -LiteralPath $pkgBin -Destination (Join-Path $binDir "matter-cli.exe") -Force
    Copy-Item -LiteralPath $pkgBin -Destination (Join-Path $binDir "matter.exe") -Force
    $owned.Add("bin/matter-cli.exe"); $owned.Add("bin/matter.exe")
}
if (-not (Test-Path -LiteralPath (Join-Path $binDir "matter-cli.exe"))) {
    throw "Install failed: bin\matter-cli.exe missing after copy from $PackageRoot"
}

foreach ($dir in @("examples", "schemas", "docs", "scripts")) {
    $src = Join-Path $PackageRoot $dir
    if (Test-Path -LiteralPath $src) {
        Copy-Owned $src $dir
    }
}
foreach ($f in @("README.md", "INSTALL.txt", "LICENSE", "MANIFEST.json", "SHA256SUMS")) {
    $src = Join-Path $PackageRoot $f
    if (Test-Path -LiteralPath $src) {
        Copy-Owned $src $f
    }
}

# Restore preserved user dirs
foreach ($d in $preserve) {
    $b = Join-Path $backup $d
    if (Test-Path -LiteralPath $b) {
        $dest = Join-Path $InstallRoot $d
        if (Test-Path -LiteralPath $dest) { Remove-Item -LiteralPath $dest -Recurse -Force }
        Copy-Item -LiteralPath $b -Destination $dest -Recurse -Force
        Write-Host "  restored $d" -ForegroundColor Gray
    }
}
Remove-Item -LiteralPath $backup -Recurse -Force -ErrorAction SilentlyContinue

# Ensure projects dir exists for users
$proj = Join-Path $InstallRoot "projects"
if (-not (Test-Path -LiteralPath $proj)) {
    New-Item -ItemType Directory -Force -Path $proj | Out-Null
}

$installManifest = [pscustomobject]@{
    product       = "matter-core"
    edition       = "language-only"
    installed_at  = (Get-Date).ToString("o")
    install_root  = $InstallRoot
    package_root  = $PackageRoot
    owned_files   = ($owned | Select-Object -Unique | Sort-Object)
    preserve_dirs = $preserve
}
$imPath = Join-Path $InstallRoot ".matter-install-manifest.json"
$installManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $imPath -Encoding utf8
$owned.Add(".matter-install-manifest.json")

if (-not $SkipPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    if ($userPath -notlike "*$binDir*") {
        $newPath = if ($userPath) { "$userPath;$binDir" } else { $binDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "User PATH updated (+ bin)" -ForegroundColor Green
    }
    if ($env:PATH -notlike "*$binDir*") {
        $env:PATH = "$binDir;" + $env:PATH
    }
}

$matter = Join-Path $binDir "matter-cli.exe"
& $matter --version
if ($LASTEXITCODE -ne 0) { throw "Installed CLI failed --version" }
& $matter core-status-json 1>$null 2>$null
if ($LASTEXITCODE -ne 0) { throw "Installed CLI failed core-status-json" }

Write-Host "Install OK: $InstallRoot" -ForegroundColor Green
Write-Host "  $matter" -ForegroundColor Gray
exit 0
