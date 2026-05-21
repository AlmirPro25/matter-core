param(
    [string]$PackageRoot = "dist\matter-core-windows-x64",
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$CliPath,
    [switch]$SkipBuild,
    [switch]$SkipVerify
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Resolve-ReleaseCli {
    param([string]$ExplicitCliPath)

    if ($ExplicitCliPath) {
        if (-not (Test-Path $ExplicitCliPath -PathType Leaf)) {
            throw "CLI binary not found: $ExplicitCliPath"
        }
        return (Resolve-Path $ExplicitCliPath).Path
    }

    $candidates = @()
    if ($env:CARGO_TARGET_DIR) {
        $candidates += (Join-Path $env:CARGO_TARGET_DIR "release\matter-cli.exe")
    }
    $candidates += @(
        "target\release\matter-cli.exe",
        "target_matter\release\matter-cli.exe",
        "..\matter_target\release\matter-cli.exe",
        "F:\Users\almir\Desktop\matter_target\release\matter-cli.exe"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate -PathType Leaf) {
            return (Resolve-Path $candidate).Path
        }
    }

    throw "Release CLI binary not found. Pass -CliPath or run without -SkipBuild."
}

function Copy-File {
    param(
        [string]$Source,
        [string]$Destination
    )

    if (-not (Test-Path $Source -PathType Leaf)) {
        throw "Required release file not found: $Source"
    }
    $parent = Split-Path -Parent $Destination
    if ($parent) {
        New-Item -ItemType Directory -Force $parent | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

function Copy-Directory {
    param(
        [string]$Source,
        [string]$Destination
    )

    if (-not (Test-Path $Source -PathType Container)) {
        throw "Required release directory not found: $Source"
    }
    $parent = Split-Path -Parent $Destination
    if ($parent) {
        New-Item -ItemType Directory -Force $parent | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
}

if (-not $SkipBuild -and -not $CliPath) {
    Write-Host "Building matter-cli release binary..." -ForegroundColor Cyan
    cargo build -p matter-cli --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
}

$cliBinary = Resolve-ReleaseCli $CliPath
$packagePath = [System.IO.Path]::GetFullPath($PackageRoot)
$zipFullPath = [System.IO.Path]::GetFullPath($ZipPath)

if (Test-Path $packagePath) {
    Remove-Item -LiteralPath $packagePath -Recurse -Force
}
New-Item -ItemType Directory -Force $packagePath | Out-Null

Copy-File $cliBinary (Join-Path $packagePath "matter-cli.exe")
Copy-File "README.md" (Join-Path $packagePath "README.md")
Copy-File "docs\USER_ONBOARDING.md" (Join-Path $packagePath "USER_ONBOARDING.md")
Copy-File "docs\LANGUAGE_TOUR.md" (Join-Path $packagePath "LANGUAGE_TOUR.md")
Copy-File "docs\technical\RUST_FFI_ABI.md" (Join-Path $packagePath "docs\technical\RUST_FFI_ABI.md")
Copy-File "docs\technical\FFI_NATIVE_SMOKE.md" (Join-Path $packagePath "docs\technical\FFI_NATIVE_SMOKE.md")
Copy-File "schemas\ffi-validation-matrix.schema.json" (Join-Path $packagePath "schemas\ffi-validation-matrix.schema.json")

foreach ($path in @(
    "examples\README.md",
    "examples\first_run.matter",
    "examples\agent_policy_demo.matter",
    "examples\language_tour.matter",
    "examples\reflexive_self.matter",
    "examples\matter_studio_ui.matter"
)) {
    Copy-File $path (Join-Path $packagePath $path)
}

foreach ($path in @(
    "examples\rust_ffi_plugin",
    "examples\go_native_plugin",
    "examples\node_native_host",
    "apps\matter-studio"
)) {
    Copy-Directory $path (Join-Path $packagePath $path)
}

foreach ($path in @(
    "scripts\export-ffi-validation-matrix.ps1",
    "scripts\export-ffi-validation-report.ps1",
    "scripts\export-release-readiness.ps1",
    "scripts\export-release-package-manifest.ps1",
    "scripts\ffi-smoke-all.ps1",
    "scripts\test-ffi-validation-matrix-contract.ps1",
    "scripts\test-ffi-validation-report-contract.ps1",
    "scripts\test-release-readiness-contract.ps1",
    "scripts\test-release-package-contract.ps1",
    "scripts\test-release-install-contract.ps1",
    "scripts\test-release-artifact-checksums-contract.ps1",
    "scripts\test-release-zip-installer-contract.ps1",
    "scripts\diagnose-local-install.ps1",
    "scripts\install-release-local.ps1",
    "scripts\install-release-zip.ps1",
    "scripts\uninstall-local.ps1",
    "scripts\build-release-package.ps1",
    "scripts\export-release-artifact-checksums.ps1",
    "scripts\verify-release-artifact-checksums.ps1",
    "scripts\verify-release-package.ps1",
    "scripts\rust-ffi-plugin-smoke.ps1",
    "scripts\native-ffi-smoke.ps1",
    "scripts\verify-ffi-smoke-summaries.ps1"
)) {
    Copy-File $path (Join-Path $packagePath $path)
}

$ffiTarget = Join-Path $packagePath "target\ffi"
New-Item -ItemType Directory -Force $ffiTarget | Out-Null
foreach ($pattern in @("target\ffi\*.json", "target\ffi\*.md")) {
    $files = Get-ChildItem $pattern -File -ErrorAction SilentlyContinue
    foreach ($file in $files) {
        Copy-Item -LiteralPath $file.FullName -Destination $ffiTarget -Force
    }
}

& powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-package-manifest.ps1" -PackageRoot $packagePath
if ($LASTEXITCODE -ne 0) {
    throw "Release package manifest export failed with exit code $LASTEXITCODE"
}

if (-not $SkipVerify) {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\verify-release-package.ps1" -PackageRoot $packagePath
    if ($LASTEXITCODE -ne 0) {
        throw "Release package folder verification failed with exit code $LASTEXITCODE"
    }
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-install-contract.ps1" -PackageRoot $packagePath
    if ($LASTEXITCODE -ne 0) {
        throw "Release package folder install contract failed with exit code $LASTEXITCODE"
    }
}

$zipParent = Split-Path -Parent $zipFullPath
if ($zipParent) {
    New-Item -ItemType Directory -Force $zipParent | Out-Null
}
if (Test-Path $zipFullPath) {
    Remove-Item -LiteralPath $zipFullPath -Force
}
Compress-Archive -Path (Join-Path $packagePath "*") -DestinationPath $zipFullPath -Force

& powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-artifact-checksums.ps1" `
    -ArtifactPaths @($zipFullPath) `
    -JsonOut "dist\release-checksums.json" `
    -Sha256Out "dist\SHA256SUMS.txt"
if ($LASTEXITCODE -ne 0) {
    throw "Release artifact checksum export failed with exit code $LASTEXITCODE"
}
& powershell -ExecutionPolicy Bypass -File ".\scripts\verify-release-artifact-checksums.ps1" `
    -JsonPath "dist\release-checksums.json" `
    -Sha256Path "dist\SHA256SUMS.txt"
if ($LASTEXITCODE -ne 0) {
    throw "Release artifact checksum verification failed with exit code $LASTEXITCODE"
}

if (-not $SkipVerify) {
    & powershell -ExecutionPolicy Bypass -File ".\scripts\verify-release-package.ps1" -ZipPath $zipFullPath
    if ($LASTEXITCODE -ne 0) {
        throw "Release package zip verification failed with exit code $LASTEXITCODE"
    }
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-install-contract.ps1" -ZipPath $zipFullPath
    if ($LASTEXITCODE -ne 0) {
        throw "Release package zip install contract failed with exit code $LASTEXITCODE"
    }
    & powershell -ExecutionPolicy Bypass -File ".\scripts\test-release-zip-installer-contract.ps1" `
        -ZipPath $zipFullPath `
        -ChecksumJsonPath "dist\release-checksums.json" `
        -Sha256Path "dist\SHA256SUMS.txt"
    if ($LASTEXITCODE -ne 0) {
        throw "Release zip installer contract failed with exit code $LASTEXITCODE"
    }
}

[ordered]@{
    ok = $true
    package_root = $packagePath
    zip_path = $zipFullPath
    cli_binary = $cliBinary
} | ConvertTo-Json -Depth 4
