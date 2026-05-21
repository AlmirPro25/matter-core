param(
    [string]$ProgramPath = "examples\\first_run.matter",
    [switch]$CiLike,
    [switch]$SkipFmt,
    [switch]$SkipBenchmarkGate,
    [switch]$FlowOnly,
    [int]$BenchmarkIterations = 20,
    [string]$OutJson = "target\\validation\\ai-canonical-profile-summary.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "Running AI canonical profile..." -ForegroundColor Cyan
Write-Host ("Program: {0}" -f $ProgramPath) -ForegroundColor DarkCyan
Write-Host ("OutJson: {0}" -f $OutJson) -ForegroundColor DarkCyan

if ($FlowOnly) {
    $flowOutDir = "target\\ai-flow\\profile-flow-only"
    $flowArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\\scripts\\ai-app-canonical-flow.ps1",
        "-ProgramPath", $ProgramPath,
        "-OutDir", $flowOutDir,
        "-BenchmarkIterations", "$BenchmarkIterations"
    )
    if ($SkipBenchmarkGate) {
        $flowArgs += "-SkipBenchmarkGate"
    }
    powershell @flowArgs
    $exit = $LASTEXITCODE

    $summary = [PSCustomObject]@{
        timestamp = (Get-Date).ToString("o")
        mode = "flow_only"
        success = ($exit -eq 0)
        program = $ProgramPath
        flow_out_dir = $flowOutDir
        benchmark_iterations = $BenchmarkIterations
        skip_benchmark_gate = [bool]$SkipBenchmarkGate
    }
    $outDir = Split-Path -Parent $OutJson
    if ($outDir) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    $summary | ConvertTo-Json -Depth 6 | Set-Content -Path $OutJson -Encoding UTF8
    exit $exit
}
else {
    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\\scripts\\validate-full-workspace.ps1",
        "-Quick",
        "-JsonSummary",
        "-JsonOut", $OutJson,
        "-RunAiCanonicalFlow",
        "-AiFlowProgramPath", $ProgramPath,
        "-AiFlowBenchmarkIterations", "$BenchmarkIterations"
    )

    if ($CiLike) {
        $args += "-CiMode"
    }
    if ($SkipFmt) {
        $args += "-SkipFmt"
    }
    if ($SkipBenchmarkGate) {
        $args += "-AiFlowSkipBenchmarkGate"
    }

    powershell @args
    exit $LASTEXITCODE
}
