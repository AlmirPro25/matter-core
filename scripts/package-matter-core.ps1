# Phase 4: Build a portable language-only Matter Core package (no target/, no toolchain).
# Usage:
#   .\scripts\package-matter-core.ps1
#   .\scripts\package-matter-core.ps1 -CliPath path\to\matter-cli.exe -SkipBuild
#   .\scripts\package-matter-core.ps1 -OutDir target\validation\...\temp-package -ZipPath ... -AllowDistWrite
#
# SAFETY (Artifact Recovery Hotfix):
# - Default output is under target/validation/ (NOT dist/).
# - Writing under dist/ requires explicit -AllowDistWrite.
# - Existing ZIP/files are never overwritten unless -ForceOverwrite is passed.
param(
    [string]$CliPath = "",
    [string]$LspPath = "",
    [string]$Version = "",
    [string]$OutDir = "",
    [string]$ZipPath = "",
    [switch]$SkipBuild,
    [switch]$SkipLsp,
    [switch]$AllowDistWrite,
    [switch]$ForceOverwrite
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location -LiteralPath $repoRoot

if (-not $Version) {
    $Version = "0.1.0"
    $cargo = Join-Path $repoRoot "Cargo.toml"
    if (Test-Path -LiteralPath $cargo) {
        $m = Select-String -Path $cargo -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
        if ($m) { $Version = $m.Matches[0].Groups[1].Value }
    }
}

$pkgName = "matter-core-$Version-windows-x64"
$distRoot = Join-Path $repoRoot "dist"

# Default: validation temp package (never touch dist unless explicitly allowed)
if (-not $OutDir) {
    $stamp = Get-Date -Format "yyyyMMdd_HHmmss"
    # Package *content* root — ZIP must NOT live inside this directory (Windows CreateFromDirectory lock).
    $OutDir = Join-Path $repoRoot "target\validation\packages\$pkgName-$stamp\root"
}
if (-not $ZipPath) {
    $parent = Split-Path -Parent $OutDir
    if (-not $parent) { $parent = Join-Path $repoRoot "target\validation\packages" }
    $ZipPath = Join-Path $parent "$pkgName.zip"
}

function Test-PathUnderDist([string]$Path) {
    $full = [System.IO.Path]::GetFullPath($Path)
    $distFull = [System.IO.Path]::GetFullPath($distRoot)
    return $full.StartsWith($distFull, [System.StringComparison]::OrdinalIgnoreCase)
}

if ((Test-PathUnderDist $OutDir) -or (Test-PathUnderDist $ZipPath)) {
    if (-not $AllowDistWrite) {
        throw @"
REFUSED: package output path is under dist/ but -AllowDistWrite was not set.
  OutDir=$OutDir
  ZipPath=$ZipPath
Use a path under target\validation\... or pass -AllowDistWrite explicitly.
Frozen 0.1.0 artifacts must not be overwritten by packaging tests.
"@
    }
}

function Resolve-Cli([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit -PathType Leaf)) {
            throw "CLI not found: $Explicit"
        }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    # Prefer host default release; then gnu target. Record both if both exist.
    $candidates = @(
        (Join-Path $repoRoot "target\release\matter-cli.exe"),
        (Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release\matter-cli.exe")
    )
    $found = @()
    foreach ($c in $candidates) {
        if (Test-Path -LiteralPath $c -PathType Leaf) {
            $item = Get-Item -LiteralPath $c
            $found += [pscustomobject]@{
                path = (Resolve-Path -LiteralPath $c).Path
                mtime = $item.LastWriteTimeUtc
                sha256 = (Get-FileHash -LiteralPath $c -Algorithm SHA256).Hash
                size = $item.Length
            }
        }
    }
    if ($found.Count -eq 0) { return $null }
    # Newest by mtime — never silently prefer a known-stale path order alone
    $best = $found | Sort-Object mtime -Descending | Select-Object -First 1
    if ($found.Count -gt 1) {
        $others = $found | Where-Object { $_.path -ne $best.path }
        foreach ($o in $others) {
            if ($o.sha256 -ne $best.sha256) {
                Write-Host ("NOTE: multiple matter-cli.exe differ; selecting newest mtime: {0} sha={1}" -f $best.path, $best.sha256) -ForegroundColor Yellow
                Write-Host ("  also found: {0} sha={1} mtime={2}" -f $o.path, $o.sha256, $o.mtime) -ForegroundColor Yellow
            }
        }
    }
    return $best.path
}

if (-not $SkipBuild -and -not $CliPath) {
    $buildScript = Join-Path $PSScriptRoot "build-matter-cli.ps1"
    if (Test-Path -LiteralPath $buildScript) {
        Write-Host "Building language-only matter-cli (release)..." -ForegroundColor Cyan
        & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Release
        if ($LASTEXITCODE -ne 0) { throw "build-matter-cli.ps1 failed" }
    } else {
        cargo build -p matter-cli --release --bin matter-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    }
}

# Dedicated LSP binary — independent of language-only CLI (does not enable experimental-full).
$lsp = $null
if (-not $SkipLsp) {
    if (-not $SkipBuild -and -not $LspPath) {
        Write-Host "Building matter-lsp (release, language-only surface)..." -ForegroundColor Cyan
        cargo build -p matter-lsp --release --bin matter-lsp
        if ($LASTEXITCODE -ne 0) { throw "cargo build matter-lsp failed" }
    }
    if ($LspPath) {
        if (-not (Test-Path -LiteralPath $LspPath -PathType Leaf)) { throw "LSP binary not found: $LspPath" }
        $lsp = (Resolve-Path -LiteralPath $LspPath).Path
    } else {
        foreach ($c in @(
            (Join-Path $repoRoot "target\release\matter-lsp.exe"),
            (Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release\matter-lsp.exe")
        )) {
            if (Test-Path -LiteralPath $c -PathType Leaf) {
                $lsp = (Resolve-Path -LiteralPath $c).Path
                break
            }
        }
    }
    if (-not $lsp) {
        Write-Host "WARNING: matter-lsp.exe not found; package will omit LSP binary (-SkipLsp or build first)." -ForegroundColor Yellow
    }
}

$cli = Resolve-Cli $CliPath
if (-not $cli) { throw "matter-cli.exe not found. Build first or pass -CliPath." }
$cliHash = (Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash
$cliItem = Get-Item -LiteralPath $cli
Write-Host ("Using CLI: {0}" -f $cli)
Write-Host ("CLI sha256: {0} size={1} mtime_utc={2}" -f $cliHash, $cliItem.Length, $cliItem.LastWriteTimeUtc.ToString("o"))
if ($lsp) {
    $lspHash = (Get-FileHash -LiteralPath $lsp -Algorithm SHA256).Hash
    $lspItem = Get-Item -LiteralPath $lsp
    Write-Host ("Using LSP: {0}" -f $lsp)
    Write-Host ("LSP sha256: {0} size={1}" -f $lspHash, $lspItem.Length)
}

# Smoke: language-only contract
& $cli core-status-json 1>$null 2>$null
if ($LASTEXITCODE -ne 0) { throw "CLI failed core-status-json: $cli" }
& $cli --version 1>$null 2>$null
if ($LASTEXITCODE -ne 0) { throw "CLI failed --version" }

if (Test-Path -LiteralPath $OutDir) {
    if (-not $ForceOverwrite) {
        throw "OutDir already exists (refusing overwrite without -ForceOverwrite): $OutDir"
    }
    Remove-Item -LiteralPath $OutDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$binDir = Join-Path $OutDir "bin"
$exDir = Join-Path $OutDir "examples"
$schemaDir = Join-Path $OutDir "schemas"
$scriptDir = Join-Path $OutDir "scripts"
$docDir = Join-Path $OutDir "docs"
New-Item -ItemType Directory -Force -Path $binDir, $exDir, $schemaDir, $scriptDir, $docDir | Out-Null

Copy-Item -LiteralPath $cli -Destination (Join-Path $binDir "matter-cli.exe") -Force
Copy-Item -LiteralPath $cli -Destination (Join-Path $binDir "matter.exe") -Force
if ($lsp) {
    Copy-Item -LiteralPath $lsp -Destination (Join-Path $binDir "matter-lsp.exe") -Force
}

$exampleFiles = @(
    "hello.matter", "fibonacci.matter", "events.matter",
    "agent_policy_demo.matter", "first_run.matter", "language_tour.matter"
)
foreach ($name in $exampleFiles) {
    $src = Join-Path $repoRoot "examples\$name"
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $exDir $name) -Force
    }
}

foreach ($s in @("core-status.schema.json")) {
    $src = Join-Path $repoRoot "schemas\$s"
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $schemaDir $s) -Force
    }
}

foreach ($s in @(
    "install-matter-core.ps1",
    "verify-matter-core.ps1",
    "update-matter-core.ps1",
    "uninstall-matter-core.ps1"
)) {
    $src = Join-Path $PSScriptRoot $s
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $scriptDir $s) -Force
    }
}

if (Test-Path -LiteralPath (Join-Path $repoRoot "LICENSE")) {
    Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination (Join-Path $OutDir "LICENSE") -Force
}

$readme = @"
# Matter Core $Version (Windows x64) — language-only portable package

This package is self-contained. **No Rust, GCC, Python, or Node is required** on the destination machine.

## Layout

- ``bin/matter-cli.exe`` / ``bin/matter.exe`` — language-only CLI
- ``bin/matter-lsp.exe`` — Matter LSP (stdio; optional if built). Independent of experimental-full.
- ``examples/`` — core sample programs
- ``schemas/`` — JSON schemas
- ``scripts/`` — install / verify / update / uninstall

## Run from extracted ZIP (no install)

``````powershell
cd <extracted-folder>
.\bin\matter-cli.exe --version
.\bin\matter-cli.exe core-status-json
.\bin\matter-cli.exe run .\examples\hello.matter
.\bin\matter-cli.exe compile .\examples\hello.matter -o .\hello.mbc
.\bin\matter-cli.exe run-bytecode .\hello.mbc
# LSP (stdio; VS Code extension looks for this binary — not "matter-cli lsp")
# .\bin\matter-lsp.exe
``````

## Install (any drive / path)

``````powershell
.\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot "`$env:LOCALAPPDATA\Matter"
``````

User projects should live under ``<InstallRoot>\projects\`` (preserved on update).

## Uninstall

``````powershell
.\scripts\uninstall-matter-core.ps1 -InstallRoot "`$env:LOCALAPPDATA\Matter"
``````

Edition: language-only. Experimental full binary is **not** included.
"@
Set-Content -LiteralPath (Join-Path $OutDir "README.md") -Value $readme -Encoding utf8

$installTxt = @"
Matter Core $Version — portable language-only package
Package root is this directory (relative paths only; no C:/D:/F: required).

Quick start:
  .\bin\matter-cli.exe --help
  .\bin\matter-cli.exe run .\examples\hello.matter

Install:
  powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot `$env:LOCALAPPDATA\Matter
"@
Set-Content -LiteralPath (Join-Path $OutDir "INSTALL.txt") -Value $installTxt -Encoding utf8

$files = Get-ChildItem -LiteralPath $OutDir -Recurse -File | Sort-Object FullName
$entries = @()
$sums = New-Object System.Text.StringBuilder
foreach ($f in $files) {
    $rel = $f.FullName.Substring($OutDir.Length).TrimStart('\', '/')
    $relUnix = $rel -replace '\\', '/'
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $f.FullName).Hash.ToLowerInvariant()
    $entries += [pscustomobject]@{
        path   = $relUnix
        bytes  = $f.Length
        sha256 = $hash
    }
    [void]$sums.AppendLine("$hash  $relUnix")
}

$manifest = [pscustomobject]@{
    name             = "matter-core"
    version          = $Version
    edition          = "language-only"
    target           = "x86_64-pc-windows-gnu"
    created_at       = (Get-Date).ToString("o")
    package_root     = $pkgName
    file_count       = $entries.Count
    total_bytes      = ($entries | Measure-Object -Property bytes -Sum).Sum
    files            = $entries
    excludes         = @("target/", "src/", "crates/", ".cargo/", "node_modules/", "credentials", "*.pdb")
    runtime_requires = @("Windows x64", "system CRT DLLs only (no python3/opengl/mf)")
    cli_source       = $cli
    cli_sha256       = $cliHash
}
$manifestPath = Join-Path $OutDir "MANIFEST.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding utf8
$mfHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
[void]$sums.AppendLine("$mfHash  MANIFEST.json")
Set-Content -LiteralPath (Join-Path $OutDir "SHA256SUMS") -Value $sums.ToString().TrimEnd() -Encoding ascii

# ZIP — refuse overwrite by default
$zipParent = Split-Path -Parent $ZipPath
if ($zipParent -and -not (Test-Path -LiteralPath $zipParent)) {
    New-Item -ItemType Directory -Force -Path $zipParent | Out-Null
}
if (Test-Path -LiteralPath $ZipPath) {
    if (-not $ForceOverwrite) {
        throw "ZipPath already exists (refusing overwrite without -ForceOverwrite): $ZipPath"
    }
    # Frozen read-only files must not be force-deleted silently
    $existing = Get-Item -LiteralPath $ZipPath
    if ($existing.IsReadOnly -and -not $ForceOverwrite) {
        throw "ZipPath is read-only; refusing overwrite: $ZipPath"
    }
    if ($existing.IsReadOnly) {
        Set-ItemProperty -LiteralPath $ZipPath -Name IsReadOnly -Value $false
    }
    Remove-Item -LiteralPath $ZipPath -Force
}
# Guard: ZipPath must not be under OutDir (cannot zip a directory into a file inside itself).
$outFull = [System.IO.Path]::GetFullPath($OutDir).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
$zipFull = [System.IO.Path]::GetFullPath($ZipPath)
if ($zipFull.StartsWith($outFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "ZipPath must not be inside OutDir (would lock CreateFromDirectory): ZipPath=$ZipPath OutDir=$OutDir"
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($OutDir, $ZipPath)

$zipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath).Hash
$cliOut = Get-Item -LiteralPath (Join-Path $binDir "matter-cli.exe")
Write-Host "OK package: $OutDir" -ForegroundColor Green
Write-Host "OK zip:     $ZipPath" -ForegroundColor Green
Write-Host ("CLI {0:N1} MB  package files={1} zip_sha256={2}" -f ($cliOut.Length / 1MB), $entries.Count, $zipHash)
Write-Host ("CLI path={0} sha256={1}" -f $cli, $cliHash)

$metaPath = Join-Path $OutDir "$pkgName.meta.json"
@{
    version    = $Version
    package    = $OutDir
    zip        = $ZipPath
    zip_sha256 = $zipHash
    cli_path   = $cli
    cli_sha256 = $cliHash
    cli_bytes  = $cliOut.Length
    file_count = $entries.Count
    wrote_to_dist = ((Test-PathUnderDist $OutDir) -or (Test-PathUnderDist $ZipPath))
} | ConvertTo-Json | Set-Content -LiteralPath $metaPath -Encoding utf8

# Only write meta under dist if AllowDistWrite and packaging into dist
if ($AllowDistWrite -and (Test-PathUnderDist $ZipPath)) {
    $distMeta = Join-Path $distRoot "$pkgName.meta.json"
    if ((Test-Path -LiteralPath $distMeta) -and -not $ForceOverwrite) {
        Write-Host "NOTE: dist meta exists; not overwriting without -ForceOverwrite: $distMeta" -ForegroundColor Yellow
    } else {
        Copy-Item -LiteralPath $metaPath -Destination $distMeta -Force
    }
}

exit 0
