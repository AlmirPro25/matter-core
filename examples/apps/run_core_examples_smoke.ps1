# Reproducible checks for docs/core-examples-recovery (language-only CLI)
param([string]$Cli = "")
if (-not $Cli) { $Cli = (Get-Command matter-cli -EA SilentlyContinue).Source }
if (-not $Cli -or -not (Test-Path $Cli)) { Write-Error "Pass -Cli path to language-only matter-cli.exe"; exit 2 }
$Root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
# script lives in examples/ when committed - use relative from repo root
$Repo = git rev-parse --show-toplevel
Set-Location $Repo
$core = @(
  "examples/apps/calculadora_orcamento.matter",
  "examples/apps/diario_tarefas.matter"
)
$fail = 0
foreach ($f in $core) {
  & $Cli run $f
  if ($LASTEXITCODE -ne 0) { $fail++; Write-Host "FAIL $f" } else { Write-Host "PASS $f" }
}
Write-Host "expected_fail new_features_demo (REWORK):"
& $Cli run examples/new_features_demo.matter
Write-Host "polyglot requires experimental CLI; skip here"
exit $fail
