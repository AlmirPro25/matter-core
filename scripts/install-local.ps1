# Matter Language Local Installer (no admin)
# Prefers D: install root when C: is low on space.
# Uses project-local GNU release binary (no F:).

param(
    [string]$CliPath = "",
    [string]$InstallRoot = "",
    [switch]$SkipBuild,
    [switch]$SkipPath,
    [switch]$FullDocs
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   Matter Language Installer v0.1.7    " -ForegroundColor Cyan
Write-Host "   (Local install - no admin)          " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Get-FreeBytes([string]$PathRoot) {
    try {
        $drive = (Get-Item $PathRoot).PSDrive
        if ($drive) { return [int64]$drive.Free }
    } catch {}
    # fallback: parse root like D:\
    $root = [System.IO.Path]::GetPathRoot($PathRoot)
    $vol = Get-PSDrive -Name $root.TrimEnd('\').TrimEnd(':') -ErrorAction SilentlyContinue
    if ($vol) { return [int64]$vol.Free }
    return [int64]0
}

function Resolve-InstallRoot {
    param([string]$Explicit)
    if ($Explicit) { return $Explicit }
    # Phase 4: portable default — no required C:/D:/F: drive letters.
    if ($env:MATTER_HOME -and $env:MATTER_HOME.Trim()) {
        return $env:MATTER_HOME.Trim()
    }
    return (Join-Path $env:LOCALAPPDATA "Matter")
}

function Resolve-ReleaseCli {
    param([string]$Explicit)

    if ($Explicit) {
        if (-not (Test-Path $Explicit -PathType Leaf)) {
            throw "CLI not found: $Explicit"
        }
        return (Resolve-Path $Explicit).Path
    }

    $candidates = @(
        "target\x86_64-pc-windows-gnu\release\matter-cli.exe",
        "target\release\matter-cli.exe"
    )
    foreach ($c in $candidates) {
        $p = Join-Path $repoRoot $c
        if (Test-Path $p -PathType Leaf) {
            return (Resolve-Path $p).Path
        }
    }
    return $null
}

# Build-host MinGW only (optional). Runtime install does not need it if -SkipBuild / CliPath set.
$mingwBin = $env:MATTER_MINGW_BIN
if (-not $mingwBin) {
    foreach ($c in @("D:\mingw64\mingw64\bin", "C:\mingw64\mingw64\bin")) {
        if (Test-Path (Join-Path $c "gcc.exe")) { $mingwBin = $c; break }
    }
}
if ($mingwBin -and (Test-Path (Join-Path $mingwBin "gcc.exe"))) {
    $env:PATH = "$mingwBin;" + $env:PATH
    $env:CC = Join-Path $mingwBin "gcc.exe"
    $env:CXX = Join-Path $mingwBin "g++.exe"
    $dt = Join-Path $mingwBin "dlltool.exe"
    if (Test-Path $dt) { $env:DLLTOOL = $dt }
    $env:CC_x86_64_pc_windows_gnu = $env:CC
    $env:CXX_x86_64_pc_windows_gnu = $env:CXX
}
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue

$installDir = Resolve-InstallRoot -Explicit $InstallRoot
$binDir = Join-Path $installDir "bin"

Write-Host "Install dir: $installDir" -ForegroundColor Green
Write-Host ""

Write-Host "[1/5] Preparing directories..." -ForegroundColor Yellow
if (Test-Path $installDir) {
    Write-Host "  - Removing previous install..." -ForegroundColor Gray
    Remove-Item -Path $installDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $binDir -Force | Out-Null
Write-Host "  OK" -ForegroundColor Green

Write-Host "[2/5] Resolving matter-cli release binary..." -ForegroundColor Yellow
$cli = Resolve-ReleaseCli -Explicit $CliPath
if (-not $cli -and -not $SkipBuild) {
    Write-Host "  - Building release (x86_64-pc-windows-gnu)..." -ForegroundColor Gray
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "build-matter-cli.ps1") -Release
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    $cli = Resolve-ReleaseCli -Explicit ""
}
if (-not $cli) {
    throw "Release matter-cli.exe not found. Run scripts\build-matter-cli.ps1 -Release first."
}
Write-Host "  - Using: $cli" -ForegroundColor Gray
Write-Host "  OK" -ForegroundColor Green

Write-Host "[3/5] Copying runtime files (slim)..." -ForegroundColor Yellow
Copy-Item -LiteralPath $cli -Destination (Join-Path $binDir "matter.exe") -Force
Copy-Item -LiteralPath $cli -Destination (Join-Path $binDir "matter-cli.exe") -Force

# Essential examples only (not entire tree of huge assets)
$examplesDir = Join-Path $installDir "examples"
New-Item -ItemType Directory -Path $examplesDir -Force | Out-Null
$essentialExamples = @(
    "hello.matter", "first_run.matter", "fibonacci.matter", "simple.matter",
    "functions.matter", "events.matter", "language_tour.matter", "README.md"
)
foreach ($name in $essentialExamples) {
    $src = Join-Path $repoRoot "examples\$name"
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $examplesDir $name) -Force
    }
}

# Essential docs only unless -FullDocs
$docsDir = Join-Path $installDir "docs"
New-Item -ItemType Directory -Path $docsDir -Force | Out-Null
if ($FullDocs -and (Test-Path (Join-Path $repoRoot "docs"))) {
    Copy-Item (Join-Path $repoRoot "docs\*") $docsDir -Recurse -Force
} else {
    $docPicks = @(
        "guides\LEIA_PRIMEIRO.md",
        "status\REALIDADE_ATUAL_HONESTA.md",
        "INDEX.md",
        "BUILD_STATUS.md"
    )
    foreach ($rel in $docPicks) {
        $src = Join-Path $repoRoot "docs\$rel"
        if (Test-Path $src) {
            $dest = Join-Path $docsDir $rel
            $parent = Split-Path -Parent $dest
            New-Item -ItemType Directory -Path $parent -Force | Out-Null
            Copy-Item $src $dest -Force
        }
    }
}

if (Test-Path (Join-Path $repoRoot "README.md")) {
    Copy-Item (Join-Path $repoRoot "README.md") $installDir -Force
}
if (Test-Path (Join-Path $repoRoot "schemas")) {
    Copy-Item (Join-Path $repoRoot "schemas") (Join-Path $installDir "schemas") -Recurse -Force
}
Write-Host "  OK" -ForegroundColor Green

if (-not $SkipPath) {
    Write-Host "[4/5] Updating user PATH..." -ForegroundColor Yellow
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$binDir*") {
        if ($currentPath) {
            [Environment]::SetEnvironmentVariable("Path", "$currentPath;$binDir", "User")
        } else {
            [Environment]::SetEnvironmentVariable("Path", "$binDir", "User")
        }
        Write-Host "  - PATH updated (new shells will see 'matter')" -ForegroundColor Gray
    } else {
        Write-Host "  - PATH already configured" -ForegroundColor Gray
    }
    if ($env:PATH -notlike "*$binDir*") {
        $env:PATH = "$binDir;" + $env:PATH
    }
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "[4/5] Skipping PATH update (-SkipPath)" -ForegroundColor Yellow
}

Write-Host "[5/5] Writing install info + smoke..." -ForegroundColor Yellow
$info = @"
Matter Language v0.1.7
Installed: $installDir
CLI source: $cli
Target: x86_64-pc-windows-gnu (project-local on D:)

Usage:
  matter --help
  matter core-status-json
  matter run `"$installDir\examples\hello.matter`"

Uninstall:
  Remove-Item -Recurse -Force `"$installDir`"
  (and remove $binDir from User PATH if desired)
"@
Set-Content -Path (Join-Path $installDir "INSTALL.txt") -Value $info -Encoding UTF8

$matter = Join-Path $binDir "matter.exe"
& $matter core-status-json | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Installed matter.exe failed core-status-json" }
Write-Host "  OK smoke core-status-json" -ForegroundColor Green

Write-Host ""
Write-Host "Matter installed." -ForegroundColor Green
Write-Host "  $matter" -ForegroundColor Gray
Write-Host "Open a new terminal and run: matter --help" -ForegroundColor Cyan
