param(
    [string]$PackageRoot = "dist\matter-core-windows-x64",
    [string]$ZipPath,
    [switch]$SkipBinary
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$tempExtractRoot = $null

if ($ZipPath) {
    if (-not (Test-Path $ZipPath -PathType Leaf)) {
        throw "Release package zip not found: $ZipPath"
    }

    $tempExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_zip_verify_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force $tempExtractRoot | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $tempExtractRoot -Force

    $directPackageRoot = Join-Path $tempExtractRoot "matter-core-windows-x64"
    if (Test-Path $directPackageRoot -PathType Container) {
        $PackageRoot = $directPackageRoot
    }
    else {
        $PackageRoot = $tempExtractRoot
    }
}

function Assert-Path {
    param(
        [string]$RelativePath,
        [switch]$Directory
    )

    $fullPath = Join-Path $PackageRoot $RelativePath
    if ($Directory) {
        if (-not (Test-Path $fullPath -PathType Container)) {
            throw "Release package missing directory: $RelativePath"
        }
    }
    else {
        if (-not (Test-Path $fullPath -PathType Leaf)) {
            throw "Release package missing file: $RelativePath"
        }
    }
}

function Assert-JsonFile {
    param([string]$RelativePath)

    $fullPath = Join-Path $PackageRoot $RelativePath
    Assert-Path $RelativePath
    Get-Content $fullPath -Raw | ConvertFrom-Json > $null
}

function Assert-ReportContract {
    param(
        [string]$Path,
        [object]$Matrix
    )

    $reportText = Get-Content $Path -Raw
    foreach ($required in @(
        "# FFI Validation Report",
        'Schema: `schemas/ffi-validation-matrix.schema.json`',
        "Production claim allowed",
        '`rust-dynamic-json-abi`',
        '`node-native-napi`',
        '`go-native-cgo`',
        '`java-native-jni`',
        '`false`',
        "examples\node_native_host\smoke.js",
        "examples\go_native_plugin\plugin.go"
    )) {
        if (-not $reportText.Contains($required)) {
            throw "Release package FFI validation report missing expected content: $required"
        }
    }

    if ($reportText.Contains([string](Resolve-Path $PackageRoot))) {
        throw "Release package FFI validation report contains package absolute path"
    }

    if ($reportText -match '\|\s*`[^`]+`\s*\|\s*`[^`]+`\s*\|\s*`[^`]+`\s*\|\s*`true`\s*\|') {
        throw "Release package FFI validation report allows a production claim"
    }

    if ($Matrix) {
        if (-not $reportText.Contains(("Generated: {0}" -f $Matrix.generated_at))) {
            throw "Release package FFI validation report does not match matrix timestamp"
        }

        foreach ($bridge in @($Matrix.bridges)) {
            $rowPrefix = '| `{0}` | `{1}` | `{2}` |' -f $bridge.id, $bridge.crate, $bridge.status
            if (-not $reportText.Contains($rowPrefix)) {
                throw "Release package FFI validation report missing matrix row for $($bridge.id)"
            }

            if (-not $reportText.Contains($bridge.production_blocker)) {
                throw "Release package FFI validation report missing blocker for $($bridge.id)"
            }
        }
    }
}

function Assert-ManifestContract {
    param([string]$Path)

    $manifest = Get-Content $Path -Raw | ConvertFrom-Json
    if ($manifest.format -ne "matter-release-package-manifest-v1") {
        throw "Release package manifest has unexpected format"
    }

    if (-not $manifest.generated_at) {
        throw "Release package manifest missing generated_at"
    }

    $entries = @($manifest.files)
    if ($entries.Count -eq 0) {
        throw "Release package manifest has no file entries"
    }

    if ($manifest.file_count -ne $entries.Count) {
        throw "Release package manifest file_count does not match files length"
    }

    $requiredManifestPaths = @(
        "README.md",
        "matter-cli.exe",
        "schemas\ffi-validation-matrix.schema.json",
        "scripts\ffi-smoke-all.ps1",
        "scripts\export-release-readiness.ps1",
        "scripts\test-release-readiness-contract.ps1",
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
        "scripts\export-release-package-manifest.ps1"
    )

    foreach ($required in $requiredManifestPaths) {
        if ($SkipBinary -and $required -eq "matter-cli.exe") {
            continue
        }
        if (-not ($entries | Where-Object { $_.path -eq $required } | Select-Object -First 1)) {
            throw "Release package manifest missing required entry: $required"
        }
    }

    foreach ($pattern in @(
        "target\ffi\ffi-validation-matrix*.json",
        "target\ffi\ffi-validation-report*.md",
        "target\ffi\release-readiness*.json"
    )) {
        if (-not ($entries | Where-Object { $_.path -like $pattern } | Select-Object -First 1)) {
            throw "Release package manifest missing required entry pattern: $pattern"
        }
    }

    foreach ($entry in $entries) {
        if (-not $entry.path) {
            throw "Release package manifest contains an entry without path"
        }
        if ([System.IO.Path]::IsPathRooted([string]$entry.path)) {
            throw "Release package manifest contains absolute path: $($entry.path)"
        }

        $fullPath = Join-Path $PackageRoot $entry.path
        if (-not (Test-Path $fullPath -PathType Leaf)) {
            throw "Release package manifest references missing file: $($entry.path)"
        }

        $file = Get-Item $fullPath
        if ([int64]$entry.size_bytes -ne $file.Length) {
            throw "Release package manifest size mismatch for $($entry.path)"
        }

        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fullPath).Hash.ToLowerInvariant()
        if ($hash -ne $entry.sha256) {
            throw "Release package manifest hash mismatch for $($entry.path)"
        }
    }
}

$requiredFiles = @(
    "README.md",
    "LANGUAGE_TOUR.md",
    "USER_ONBOARDING.md",
    "docs\technical\RUST_FFI_ABI.md",
    "docs\technical\FFI_NATIVE_SMOKE.md",
    "schemas\ffi-validation-matrix.schema.json",
    "examples\README.md",
    "examples\first_run.matter",
    "examples\language_tour.matter",
    "examples\reflexive_self.matter",
    "examples\matter_studio_ui.matter",
    "examples\rust_ffi_plugin\Cargo.toml",
    "examples\rust_ffi_plugin\src\lib.rs",
    "examples\rust_ffi_plugin\args_add.json",
    "examples\rust_ffi_plugin\args_describe.json",
    "examples\rust_ffi_plugin\args_stats.json",
    "examples\go_native_plugin\plugin.go",
    "examples\go_native_plugin\README.md",
    "examples\node_native_host\smoke.js",
    "examples\node_native_host\README.md",
    "scripts\export-ffi-validation-matrix.ps1",
    "scripts\export-ffi-validation-report.ps1",
    "scripts\export-release-readiness.ps1",
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
    "scripts\export-release-package-manifest.ps1",
    "scripts\verify-release-package.ps1",
    "scripts\rust-ffi-plugin-smoke.ps1",
    "scripts\native-ffi-smoke.ps1",
    "scripts\verify-ffi-smoke-summaries.ps1"
)

try {
    if (-not (Test-Path $PackageRoot -PathType Container)) {
        throw "Release package root not found: $PackageRoot"
    }

    if (-not $SkipBinary) {
        Assert-Path "matter-cli.exe"
    }

    foreach ($file in $requiredFiles) {
        Assert-Path $file
    }

    foreach ($dir in @(
        "examples\rust_ffi_plugin",
        "examples\go_native_plugin",
        "examples\node_native_host",
        "scripts",
        "schemas",
        "target\ffi"
    )) {
        Assert-Path $dir -Directory
    }

    Assert-JsonFile "schemas\ffi-validation-matrix.schema.json"

    $ffiJson = Get-ChildItem (Join-Path $PackageRoot "target\ffi") -Filter "*.json" -File
    if (@($ffiJson).Count -eq 0) {
        throw "Release package missing FFI JSON summaries under target\ffi"
    }

    $matrix = $ffiJson | Where-Object { $_.Name -like "ffi-validation-matrix*.json" } | Select-Object -First 1
    if (-not $matrix) {
        throw "Release package missing FFI validation matrix JSON under target\ffi"
    }

    $report = Get-ChildItem (Join-Path $PackageRoot "target\ffi") -Filter "ffi-validation-report*.md" -File | Select-Object -First 1
    if (-not $report) {
        throw "Release package missing FFI validation report under target\ffi"
    }

    $manifest = Get-ChildItem (Join-Path $PackageRoot "target\ffi") -Filter "release-package-manifest*.json" -File | Select-Object -First 1
    if (-not $manifest) {
        throw "Release package missing release package manifest under target\ffi"
    }

    $matrixJson = Get-Content $matrix.FullName -Raw | ConvertFrom-Json
    if ($matrixJson.PSObject.Properties['$schema'].Value -ne "schemas/ffi-validation-matrix.schema.json") {
        throw "Release package FFI validation matrix has unexpected schema reference"
    }

    $readiness = $ffiJson | Where-Object { $_.Name -like "release-readiness*.json" } | Select-Object -First 1
    if (-not $readiness) {
        throw "Release package missing release readiness JSON under target\ffi"
    }

    $readinessJson = Get-Content $readiness.FullName -Raw | ConvertFrom-Json
    if (-not $readinessJson.can_publish_experimental_release) {
        throw "Release package readiness does not allow an experimental release"
    }
    if ($readinessJson.can_claim_general_production) {
        throw "Release package readiness allows a general production claim"
    }
    if ($readinessJson.readiness_tier -ne "experimental_release_candidate") {
        throw "Release package readiness has unexpected tier: $($readinessJson.readiness_tier)"
    }
    if ($readinessJson.matrix_generated_at -ne $matrixJson.generated_at) {
        throw "Release package readiness does not match matrix timestamp"
    }
    if ($readinessJson.matrix_schema -ne "schemas/ffi-validation-matrix.schema.json") {
        throw "Release package readiness has unexpected matrix schema"
    }
    if ([System.IO.Path]::IsPathRooted([string]$readinessJson.matrix)) {
        throw "Release package readiness contains an absolute matrix path"
    }

    foreach ($requiredBridgeId in @("rust-dynamic-json-abi", "node-native-napi", "go-native-cgo")) {
        $matrixBridge = $matrixJson.bridges | Where-Object { $_.id -eq $requiredBridgeId } | Select-Object -First 1
        $readinessBridge = $readinessJson.required_smoke_statuses | Where-Object { $_.bridge -eq $requiredBridgeId } | Select-Object -First 1
        if (-not $matrixBridge) {
            throw "Release package matrix missing required smoke bridge: $requiredBridgeId"
        }
        if (-not $readinessBridge) {
            throw "Release package readiness missing required smoke bridge: $requiredBridgeId"
        }
        if ($readinessBridge.status -ne $matrixBridge.status) {
            throw "Release package readiness status mismatch for $requiredBridgeId"
        }
        if ($readinessBridge.status -ne "validated_smoke") {
            throw "Release package readiness required smoke is not validated: $requiredBridgeId"
        }
    }

    Assert-ReportContract $report.FullName $matrixJson
    Assert-ManifestContract $manifest.FullName

    [ordered]@{
        ok = $true
        package_root = $PackageRoot
        zip_path = if ($ZipPath) { $ZipPath } else { $null }
        checked_files = @($requiredFiles).Count
        ffi_json_files = @($ffiJson | ForEach-Object { $_.Name })
        ffi_report = $report.Name
        release_readiness = $readiness.Name
        release_manifest = $manifest.Name
    } | ConvertTo-Json -Depth 4
}
finally {
    if ($tempExtractRoot -and (Test-Path $tempExtractRoot)) {
        Remove-Item -LiteralPath $tempExtractRoot -Recurse -Force
    }
}
