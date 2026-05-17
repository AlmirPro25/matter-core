param(
    [string]$ReportPath = "target\ffi\ffi-validation-report.md"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

if (-not (Test-Path $ReportPath)) {
    throw "Missing FFI validation report: $ReportPath"
}

$report = Get-Content $ReportPath -Raw
$repoRootText = [string]$RepoRoot

foreach ($required in @(
    "# FFI Validation Report",
    'Schema: `schemas/ffi-validation-matrix.schema.json`',
    "Production claim allowed",
    "production-ready",
    '`rust-dynamic-json-abi`',
    '`node-native-napi`',
    '`go-native-cgo`',
    '`java-native-jni`',
    '`false`',
    "examples\node_native_host\smoke.js",
    "examples\go_native_plugin\plugin.go"
)) {
    if (-not $report.Contains($required)) {
        throw "FFI validation report missing expected content: $required"
    }
}

if ($report.Contains($repoRootText)) {
    throw "FFI validation report contains workspace absolute path"
}

if ($report -match '\|\s*`[^`]+`\s*\|\s*`[^`]+`\s*\|\s*`[^`]+`\s*\|\s*`true`\s*\|') {
    throw "FFI validation report allows a production claim"
}

[ordered]@{
    ok = $true
    report = $ReportPath
    checked = @(
        "required bridge rows",
        "production blockers",
        "relative example paths",
        "no workspace absolute path",
        "no production claim allowed"
    )
} | ConvertTo-Json -Depth 4
