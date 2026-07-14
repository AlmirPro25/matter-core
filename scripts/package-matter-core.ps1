# Phase 4: Build a portable language-only Matter Core package (no target/, no toolchain).
# Usage:
#   .\scripts\package-matter-core.ps1
#   .\scripts\package-matter-core.ps1 -CliPath path\to\matter-cli.exe -SkipBuild
param(
    [string]$CliPath = "",
    [string]$Version = "",
    [string]$OutDir = "",
    [switch]$SkipBuild
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
if (-not $OutDir) {
    $OutDir = Join-Path $repoRoot "dist\$pkgName"
}
$zipPath = Join-Path $repoRoot "dist\$pkgName.zip"

function Resolve-Cli([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit -PathType Leaf)) {
            throw "CLI not found: $Explicit"
        }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    $candidates = @(
        (Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release\matter-cli.exe"),
        (Join-Path $repoRoot "target\release\matter-cli.exe")
    )
    foreach ($c in $candidates) {
        if (Test-Path -LiteralPath $c -PathType Leaf) {
            return (Resolve-Path -LiteralPath $c).Path
        }
    }
    return $null
}

if (-not $SkipBuild -and -not $CliPath) {
    $buildScript = Join-Path $PSScriptRoot "build-matter-cli.ps1"
    if (Test-Path -LiteralPath $buildScript) {
        Write-Host "Building language-only matter-cli (release)..." -ForegroundColor Cyan
        & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Release
        if ($LASTEXITCODE -ne 0) { throw "build-matter-cli.ps1 failed" }
    } else {
        cargo build -p matter-cli --release --target x86_64-pc-windows-gnu --bin matter-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    }
}

$cli = Resolve-Cli $CliPath
if (-not $cli) { throw "matter-cli.exe not found. Build first or pass -CliPath." }

# Smoke: language-only contract (NOT world/frontier experimental)
& $cli core-status-json 1>$null 2>$null
if ($LASTEXITCODE -ne 0) { throw "CLI failed core-status-json: $cli" }
& $cli --version 1>$null 2>$null
if ($LASTEXITCODE -ne 0) { throw "CLI failed --version" }

if (Test-Path -LiteralPath $OutDir) {
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

# Core examples only (no polyglot/ffi demos)
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

# Schemas needed for core status consumers
foreach ($s in @("core-status.schema.json")) {
    $src = Join-Path $repoRoot "schemas\$s"
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $schemaDir $s) -Force
    }
}

# Portable install/verify/uninstall scripts (ship with package)
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
``````

## Install (any drive / path)

``````powershell
.\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot "`$env:LOCALAPPDATA\Matter"
# or any path, including paths with spaces/Unicode:
# .\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot "E:\Apps\Matter Core"
``````

User projects should live under ``<InstallRoot>\projects\`` (preserved on update).

## Uninstall

``````powershell
.\scripts\uninstall-matter-core.ps1 -InstallRoot "`$env:LOCALAPPDATA\Matter"
``````

Removes only files listed in the install manifest. Does **not** delete ``projects\`` unless ``-RemoveProjects`` is passed.

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

# File list + hashes
$files = Get-ChildItem -LiteralPath $OutDir -Recurse -File | Sort-Object FullName
$entries = @()
$sums = New-Object System.Text.StringBuilder
foreach ($f in $files) {
    $rel = $f.FullName.Substring($OutDir.Length).TrimStart('\', '/')
    $relUnix = $rel -replace '\\', '/'
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $f.FullName).Hash.ToLowerInvariant()
    $entries += [pscustomobject]@{
        path  = $relUnix
        bytes = $f.Length
        sha256 = $hash
    }
    [void]$sums.AppendLine("$hash  $relUnix")
}

$manifest = [pscustomobject]@{
    name            = "matter-core"
    version         = $Version
    edition         = "language-only"
    target          = "x86_64-pc-windows-gnu"
    created_at      = (Get-Date).ToString("o")
    package_root    = $pkgName
    file_count      = $entries.Count
    total_bytes     = ($entries | Measure-Object -Property bytes -Sum).Sum
    files           = $entries
    excludes        = @("target/", "src/", "crates/", ".cargo/", "node_modules/", "credentials", "*.pdb")
    runtime_requires = @("Windows x64", "system CRT DLLs only (no python3/opengl/mf)")
}
$manifestPath = Join-Path $OutDir "MANIFEST.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding utf8
# re-hash after manifest write
$mfHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
[void]$sums.AppendLine("$mfHash  MANIFEST.json")
Set-Content -LiteralPath (Join-Path $OutDir "SHA256SUMS") -Value $sums.ToString().TrimEnd() -Encoding ascii

# ZIP
if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($OutDir, $zipPath)

$zipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath).Hash
$cliItem = Get-Item -LiteralPath (Join-Path $binDir "matter-cli.exe")
Write-Host "OK package: $OutDir" -ForegroundColor Green
Write-Host "OK zip:     $zipPath" -ForegroundColor Green
Write-Host ("CLI {0:N1} MB  package files={1} zip_sha256={2}" -f ($cliItem.Length/1MB), $entries.Count, $zipHash)

@{
    version   = $Version
    package   = $OutDir
    zip       = $zipPath
    zip_sha256 = $zipHash
    cli_bytes = $cliItem.Length
    file_count = $entries.Count
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $repoRoot "dist\$pkgName.meta.json") -Encoding utf8

exit 0
