param(
    [string]$Manifest = "matter.toml",
    [string]$OutputDir = "tmp_web_demo_live",
    [string]$AppName = "demo_live",
    [int]$Port = 8099,
    [int]$DeltaMs = 16,
    [int]$IntervalMs = 120
)

$ErrorActionPreference = "Stop"

function Resolve-CliPath {
    param([string]$RepoRoot)

    $candidates = @(
        (Join-Path $RepoRoot "target_live_demo_bin2\debug\matter-cli.exe"),
        (Join-Path $RepoRoot "target_live_demo_bin\debug\matter-cli.exe"),
        (Join-Path $RepoRoot "target_codex_cli_visual_project_bridge\debug\matter-cli.exe"),
        (Join-Path $RepoRoot "target\debug\matter-cli.exe"),
        (Join-Path $RepoRoot "target\release\matter-cli.exe")
    )

    foreach ($path in $candidates) {
        if (Test-Path $path) {
            return $path
        }
    }
    return $null
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cliPath = Resolve-CliPath -RepoRoot $repoRoot

if (-not $cliPath) {
    Write-Host "matter-cli.exe nao encontrado. Compile primeiro:"
    Write-Host "  cargo build --manifest-path crates/matter-cli/Cargo.toml --target-dir target_codex_cli_visual_project_bridge"
    exit 1
}

Write-Host "Usando CLI: $cliPath"
Write-Host "Subindo demo viva bidirecional em http://127.0.0.1:$Port/"

$serveArgs = @(
    "project-web-serve-json",
    $Manifest,
    $OutputDir,
    $AppName,
    "$Port"
)

$loopArgs = @(
    "project-web-loop-live-json",
    $Manifest,
    "$Port",
    "$DeltaMs",
    "forever",
    "--interval-ms",
    "$IntervalMs"
)

$serveProc = Start-Process -FilePath $cliPath -ArgumentList $serveArgs -WorkingDirectory $repoRoot -WindowStyle Hidden -PassThru
Start-Sleep -Milliseconds 500
$loopProc = Start-Process -FilePath $cliPath -ArgumentList $loopArgs -WorkingDirectory $repoRoot -WindowStyle Hidden -PassThru

Write-Host ""
Write-Host "Demo em execucao:"
Write-Host "  Canvas:   http://127.0.0.1:$Port/"
Write-Host "  VM State: http://127.0.0.1:$Port/state/vm"
Write-Host ""
Write-Host "PIDs:"
Write-Host "  serve = $($serveProc.Id)"
Write-Host "  loop  = $($loopProc.Id)"
Write-Host ""
Write-Host "Validacao viva:"
& $cliPath web-live-demo-check-json "$Port" 5000
Write-Host ""
Write-Host "Para encerrar:"
Write-Host "  Stop-Process -Id $($serveProc.Id),$($loopProc.Id)"
