param(
    [switch]$Release,
    [string]$CliPath,
    [switch]$IncludeJava,
    [string]$JsonDir = "target\ffi"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

New-Item -ItemType Directory -Force $JsonDir | Out-Null

$rustSummary = Join-Path $JsonDir "rust-smoke.json"
$nativeSummary = Join-Path $JsonDir "native-smoke.json"

$rustArgs = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\rust-ffi-plugin-smoke.ps1", "-JsonOut", $rustSummary)
if ($Release) {
    $rustArgs += "-Release"
}
if ($CliPath) {
    $rustArgs += @("-CliPath", $CliPath)
}

$nativeArgs = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\native-ffi-smoke.ps1", "-JsonOut", $nativeSummary)
if ($IncludeJava) {
    $nativeArgs += "-IncludeJava"
}

$verifySummaryArgs = @(
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    ".\scripts\verify-ffi-smoke-summaries.ps1",
    "-RustSummary",
    $rustSummary,
    "-NativeSummary",
    $nativeSummary
)
if ($IncludeJava) {
    $verifySummaryArgs += "-RequireJava"
}

$matrixOut = Join-Path $JsonDir "ffi-validation-matrix.json"
$matrixArgs = @(
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    ".\scripts\export-ffi-validation-matrix.ps1",
    "-RustSummary",
    $rustSummary,
    "-NativeSummary",
    $nativeSummary,
    "-Out",
    $matrixOut
)

$verifyMatrixArgs = @(
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    ".\scripts\verify-ffi-smoke-summaries.ps1",
    "-RustSummary",
    $rustSummary,
    "-NativeSummary",
    $nativeSummary,
    "-MatrixPath",
    $matrixOut,
    "-CheckMatrix"
)
if ($IncludeJava) {
    $verifyMatrixArgs += "-RequireJava"
}

$reportOut = Join-Path $JsonDir "ffi-validation-report.md"
$readinessOut = Join-Path $JsonDir "release-readiness.json"

Write-Host "== Rust FFI smoke"
& powershell @rustArgs
if ($LASTEXITCODE -ne 0) {
    throw "Rust FFI smoke failed with exit code $LASTEXITCODE"
}

Write-Host "== Native FFI smoke"
& powershell @nativeArgs
if ($LASTEXITCODE -ne 0) {
    throw "Native FFI smoke failed with exit code $LASTEXITCODE"
}

Write-Host "== Verify FFI smoke summaries"
& powershell @verifySummaryArgs
if ($LASTEXITCODE -ne 0) {
    throw "FFI smoke summary verification failed with exit code $LASTEXITCODE"
}

Write-Host "== Export FFI validation matrix"
& powershell @matrixArgs
if ($LASTEXITCODE -ne 0) {
    throw "FFI validation matrix export failed with exit code $LASTEXITCODE"
}

Write-Host "== Verify FFI validation matrix"
& powershell @verifyMatrixArgs
if ($LASTEXITCODE -ne 0) {
    throw "FFI validation matrix verification failed with exit code $LASTEXITCODE"
}

Write-Host "== Export FFI validation report"
& powershell -ExecutionPolicy Bypass -File .\scripts\export-ffi-validation-report.ps1 `
    -MatrixPath $matrixOut `
    -Out $reportOut
if ($LASTEXITCODE -ne 0) {
    throw "FFI validation report export failed with exit code $LASTEXITCODE"
}

Write-Host "== Test FFI validation report contract"
& powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-report-contract.ps1 `
    -ReportPath $reportOut
if ($LASTEXITCODE -ne 0) {
    throw "FFI validation report contract test failed with exit code $LASTEXITCODE"
}

Write-Host "== Test FFI validation matrix contract"
& powershell -ExecutionPolicy Bypass -File .\scripts\test-ffi-validation-matrix-contract.ps1 `
    -RustSummary $rustSummary `
    -NativeSummary $nativeSummary `
    -MatrixPath $matrixOut
if ($LASTEXITCODE -ne 0) {
    throw "FFI validation matrix contract test failed with exit code $LASTEXITCODE"
}

Write-Host "== Export release readiness"
& powershell -ExecutionPolicy Bypass -File .\scripts\export-release-readiness.ps1 `
    -MatrixPath $matrixOut `
    -Out $readinessOut
if ($LASTEXITCODE -ne 0) {
    throw "Release readiness export failed with exit code $LASTEXITCODE"
}

Write-Host "== Test release readiness contract"
& powershell -ExecutionPolicy Bypass -File .\scripts\test-release-readiness-contract.ps1 `
    -MatrixPath $matrixOut
if ($LASTEXITCODE -ne 0) {
    throw "Release readiness contract test failed with exit code $LASTEXITCODE"
}

Write-Host "== Test release package contract"
& powershell -ExecutionPolicy Bypass -File .\scripts\test-release-package-contract.ps1 `
    -RustSummary $rustSummary `
    -NativeSummary $nativeSummary `
    -MatrixPath $matrixOut
if ($LASTEXITCODE -ne 0) {
    throw "Release package contract test failed with exit code $LASTEXITCODE"
}

[ordered]@{
    ok             = $true
    rust_summary   = $rustSummary
    native_summary = $nativeSummary
    matrix         = $matrixOut
    report         = $reportOut
    readiness      = $readinessOut
    java_required  = [bool]$IncludeJava
} | ConvertTo-Json -Depth 4
