param(
    [string]$RustSummary = "target\ffi\rust-smoke.json",
    [string]$NativeSummary = "target\ffi\native-smoke.json",
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot
$script:LastVerifierOutput = ""

function Invoke-PackageVerifier {
    param([string[]]$VerifierArgs)

    $stdout = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_contract_stdout_" + [guid]::NewGuid().ToString("N") + ".log")
    $stderr = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_contract_stderr_" + [guid]::NewGuid().ToString("N") + ".log")

    try {
        $argumentList = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\verify-release-package.ps1") + $VerifierArgs
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & powershell @argumentList > $stdout 2> $stderr
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = $oldErrorActionPreference

        $script:LastVerifierOutput = @(
            if (Test-Path $stdout) { Get-Content $stdout -Raw }
            if (Test-Path $stderr) { Get-Content $stderr -Raw }
        ) -join "`n"

        return $exitCode
    }
    finally {
        foreach ($path in @($stdout, $stderr)) {
            if (Test-Path $path) {
                Remove-Item -LiteralPath $path -Force
            }
        }
    }
}

function Expect-Success {
    param([string]$Name, [int]$ExitCode)
    if ($ExitCode -ne 0) {
        if ($script:LastVerifierOutput) {
            Write-Host $script:LastVerifierOutput
        }
        throw "$Name should have passed, got exit code $ExitCode"
    }
}

function Expect-Failure {
    param([string]$Name, [int]$ExitCode)
    if ($ExitCode -eq 0) {
        throw "$Name should have failed"
    }
}

function Copy-RequiredPackageFiles {
    param([string]$PackageRoot)

    New-Item -ItemType Directory -Force $PackageRoot | Out-Null
    New-Item -ItemType File -Force (Join-Path $PackageRoot "matter-cli.exe") | Out-Null

    foreach ($dir in @(
        "docs\technical",
        "schemas",
        "examples",
        "scripts",
        "target\ffi",
        "examples\rust_ffi_plugin\src",
        "examples\go_native_plugin",
        "examples\node_native_host"
    )) {
        New-Item -ItemType Directory -Force (Join-Path $PackageRoot $dir) | Out-Null
    }

    $pairs = @(
        @("README.md", "README.md"),
        @("docs\USER_ONBOARDING.md", "USER_ONBOARDING.md"),
        @("docs\LANGUAGE_TOUR.md", "LANGUAGE_TOUR.md"),
        @("docs\technical\RUST_FFI_ABI.md", "docs\technical\RUST_FFI_ABI.md"),
        @("docs\technical\FFI_NATIVE_SMOKE.md", "docs\technical\FFI_NATIVE_SMOKE.md"),
        @("schemas\ffi-validation-matrix.schema.json", "schemas\ffi-validation-matrix.schema.json"),
        @("examples\README.md", "examples\README.md"),
        @("examples\first_run.matter", "examples\first_run.matter"),
        @("examples\language_tour.matter", "examples\language_tour.matter"),
        @("examples\reflexive_self.matter", "examples\reflexive_self.matter"),
        @("examples\matter_studio_ui.matter", "examples\matter_studio_ui.matter"),
        @("examples\rust_ffi_plugin\Cargo.toml", "examples\rust_ffi_plugin\Cargo.toml"),
        @("examples\rust_ffi_plugin\src\lib.rs", "examples\rust_ffi_plugin\src\lib.rs"),
        @("examples\rust_ffi_plugin\args_add.json", "examples\rust_ffi_plugin\args_add.json"),
        @("examples\rust_ffi_plugin\args_describe.json", "examples\rust_ffi_plugin\args_describe.json"),
        @("examples\rust_ffi_plugin\args_stats.json", "examples\rust_ffi_plugin\args_stats.json"),
        @("examples\go_native_plugin\plugin.go", "examples\go_native_plugin\plugin.go"),
        @("examples\go_native_plugin\README.md", "examples\go_native_plugin\README.md"),
        @("examples\node_native_host\smoke.js", "examples\node_native_host\smoke.js"),
        @("examples\node_native_host\README.md", "examples\node_native_host\README.md"),
        @("scripts\export-ffi-validation-matrix.ps1", "scripts\export-ffi-validation-matrix.ps1"),
        @("scripts\export-ffi-validation-report.ps1", "scripts\export-ffi-validation-report.ps1"),
        @("scripts\export-release-readiness.ps1", "scripts\export-release-readiness.ps1"),
        @("scripts\ffi-smoke-all.ps1", "scripts\ffi-smoke-all.ps1"),
        @("scripts\test-ffi-validation-matrix-contract.ps1", "scripts\test-ffi-validation-matrix-contract.ps1"),
        @("scripts\test-ffi-validation-report-contract.ps1", "scripts\test-ffi-validation-report-contract.ps1"),
        @("scripts\test-release-readiness-contract.ps1", "scripts\test-release-readiness-contract.ps1"),
        @("scripts\test-release-package-contract.ps1", "scripts\test-release-package-contract.ps1"),
        @("scripts\export-release-package-manifest.ps1", "scripts\export-release-package-manifest.ps1"),
        @("scripts\verify-release-package.ps1", "scripts\verify-release-package.ps1"),
        @("scripts\rust-ffi-plugin-smoke.ps1", "scripts\rust-ffi-plugin-smoke.ps1"),
        @("scripts\native-ffi-smoke.ps1", "scripts\native-ffi-smoke.ps1"),
        @("scripts\verify-ffi-smoke-summaries.ps1", "scripts\verify-ffi-smoke-summaries.ps1"),
        @($RustSummary, "target\ffi\rust-smoke.json"),
        @($NativeSummary, "target\ffi\native-smoke.json"),
        @($MatrixPath, "target\ffi\ffi-validation-matrix.json"),
        @("target\ffi\ffi-validation-report.md", "target\ffi\ffi-validation-report.md"),
        @("target\ffi\release-readiness.json", "target\ffi\release-readiness.json")
    )

    foreach ($pair in $pairs) {
        if (-not (Test-Path $pair[0])) {
            throw "Missing source file for release package contract test: $($pair[0])"
        }
        Copy-Item $pair[0] (Join-Path $PackageRoot $pair[1]) -Force
    }
}

foreach ($path in @($RustSummary, $NativeSummary, $MatrixPath)) {
    if (-not (Test-Path $path)) {
        throw "Missing input for release package contract test: $path"
    }
}

& powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-readiness.ps1" -MatrixPath $MatrixPath -Out "target\ffi\release-readiness.json"
if ($LASTEXITCODE -ne 0) {
    throw "Release readiness export failed with exit code $LASTEXITCODE"
}

$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_contract_" + [guid]::NewGuid().ToString("N"))

try {
    $packageRoot = Join-Path $workRoot "matter-core-windows-x64"
    Copy-RequiredPackageFiles $packageRoot
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-package-manifest.ps1" -PackageRoot $packageRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Release package manifest export failed with exit code $LASTEXITCODE"
    }

    Expect-Success "valid release package folder" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    $zipPath = Join-Path $workRoot "matter-core-windows-x64.zip"
    Compress-Archive -Path (Join-Path $packageRoot "*") -DestinationPath $zipPath -Force
    Expect-Success "valid release package zip" (Invoke-PackageVerifier @("-ZipPath", $zipPath))

    $releaseNamedPackageRoot = Join-Path $workRoot "matter-core-windows-x64-release-names"
    Copy-RequiredPackageFiles $releaseNamedPackageRoot
    Rename-Item -LiteralPath (Join-Path $releaseNamedPackageRoot "target\ffi\ffi-validation-matrix.json") -NewName "ffi-validation-matrix-release.json"
    Rename-Item -LiteralPath (Join-Path $releaseNamedPackageRoot "target\ffi\ffi-validation-report.md") -NewName "ffi-validation-report-release.md"
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-readiness.ps1" `
        -MatrixPath (Join-Path $releaseNamedPackageRoot "target\ffi\ffi-validation-matrix-release.json") `
        -Out (Join-Path $releaseNamedPackageRoot "target\ffi\release-readiness-release.json")
    if ($LASTEXITCODE -ne 0) {
        throw "Release-named readiness export failed with exit code $LASTEXITCODE"
    }
    Remove-Item -LiteralPath (Join-Path $releaseNamedPackageRoot "target\ffi\release-readiness.json") -Force
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-package-manifest.ps1" -PackageRoot $releaseNamedPackageRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Release-named package manifest export failed with exit code $LASTEXITCODE"
    }
    Expect-Success "valid release-named FFI artifacts" (Invoke-PackageVerifier @("-PackageRoot", $releaseNamedPackageRoot))

    Remove-Item -LiteralPath (Join-Path $packageRoot "target\ffi\release-package-manifest.json") -Force
    Expect-Failure "release package without manifest" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-package-manifest.ps1" -PackageRoot $packageRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Release package manifest re-export failed with exit code $LASTEXITCODE"
    }
    Remove-Item -LiteralPath (Join-Path $packageRoot "schemas\ffi-validation-matrix.schema.json") -Force
    Expect-Failure "release package without schema" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "schemas\ffi-validation-matrix.schema.json" (Join-Path $packageRoot "schemas\ffi-validation-matrix.schema.json") -Force
    Remove-Item -LiteralPath (Join-Path $packageRoot "target\ffi\ffi-validation-matrix.json") -Force
    Expect-Failure "release package without validation matrix" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item $MatrixPath (Join-Path $packageRoot "target\ffi\ffi-validation-matrix.json") -Force
    Remove-Item -LiteralPath (Join-Path $packageRoot "target\ffi\release-readiness.json") -Force
    Expect-Failure "release package without release readiness" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "target\ffi\release-readiness.json" (Join-Path $packageRoot "target\ffi\release-readiness.json") -Force
    $readinessPath = Join-Path $packageRoot "target\ffi\release-readiness.json"
    $staleReadiness = Get-Content $readinessPath -Raw | ConvertFrom-Json
    $staleReadiness.matrix_generated_at = "2000-01-01T00:00:00.0000000-00:00"
    $staleReadiness | ConvertTo-Json -Depth 10 | Set-Content -Path $readinessPath -Encoding UTF8
    Expect-Failure "release package with stale release readiness" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "target\ffi\release-readiness.json" (Join-Path $packageRoot "target\ffi\release-readiness.json") -Force
    $driftedReadiness = Get-Content $readinessPath -Raw | ConvertFrom-Json
    ($driftedReadiness.required_smoke_statuses | Where-Object { $_.bridge -eq "go-native-cgo" }).status = "missing_smoke"
    $driftedReadiness | ConvertTo-Json -Depth 10 | Set-Content -Path $readinessPath -Encoding UTF8
    Expect-Failure "release package with drifted release readiness" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "target\ffi\release-readiness.json" (Join-Path $packageRoot "target\ffi\release-readiness.json") -Force
    Copy-Item "target\ffi\ffi-validation-report.md" (Join-Path $packageRoot "target\ffi\ffi-validation-report.md") -Force
    Set-Content -Path (Join-Path $packageRoot "target\ffi\ffi-validation-report.md") -Value "# Broken report" -Encoding UTF8
    Expect-Failure "release package with broken validation report" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "target\ffi\ffi-validation-report.md" (Join-Path $packageRoot "target\ffi\ffi-validation-report.md") -Force
    $reportPath = Join-Path $packageRoot "target\ffi\ffi-validation-report.md"
    $staleReport = (Get-Content $reportPath -Raw) -replace "Generated: [^\r\n]+", "Generated: 2000-01-01T00:00:00.0000000-00:00"
    Set-Content -Path $reportPath -Value $staleReport -Encoding UTF8
    Expect-Failure "release package with stale validation report" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))

    Copy-Item "target\ffi\ffi-validation-report.md" (Join-Path $packageRoot "target\ffi\ffi-validation-report.md") -Force
    Remove-Item -LiteralPath (Join-Path $packageRoot "scripts\ffi-smoke-all.ps1") -Force
    Expect-Failure "release package without ffi-smoke-all" (Invoke-PackageVerifier @("-PackageRoot", $packageRoot))
}
finally {
    if (Test-Path $workRoot) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}

[ordered]@{
    ok = $true
    checked = @(
        "valid release package folder passes",
        "valid release package zip passes",
        "release-named FFI artifacts pass",
        "missing release package manifest fails",
        "missing schema fails",
        "missing validation matrix fails",
        "missing release readiness fails",
        "stale release readiness fails",
        "drifted release readiness fails",
        "broken validation report fails",
        "stale validation report fails",
        "missing ffi-smoke-all fails"
    )
} | ConvertTo-Json -Depth 4
