# Run real Matter apps + polyglot smokes against release CLI on D:
param(
    [string]$CliPath = ""
)

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$candidates = @(
    $CliPath,
    "target\x86_64-pc-windows-gnu\release\matter-cli.exe",
    "D:\Matter\bin\matter.exe",
    "D:\Matter\bin\matter-cli.exe"
)
$cli = $candidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $cli) { throw "matter-cli not found. Build with scripts\build-matter-cli.ps1 -Release" }
$cli = (Resolve-Path $cli).Path
Write-Host "CLI: $cli" -ForegroundColor Cyan

$cases = @(
    @{ name = "hello"; args = @("run", "examples\hello.matter") },
    @{ name = "first_run"; args = @("run", "examples\first_run.matter") },
    @{ name = "fibonacci"; args = @("run", "examples\fibonacci.matter") },
    @{ name = "diario_tarefas"; args = @("run", "examples\apps\diario_tarefas.matter") },
    @{ name = "calculadora_orcamento"; args = @("run", "examples\apps\calculadora_orcamento.matter") },
    @{ name = "agent_policy"; args = @("run", "examples\agent_policy_demo.matter") },
    @{ name = "polyglot_runtime"; args = @("run", "examples\polyglot_runtime_smoke.matter") },
    @{ name = "python_math"; args = @("run", "examples\polyglot\python_math_smoke.matter") },
    @{ name = "python_numpy"; args = @("run", "examples\polyglot\python_numpy.matter") },
    @{ name = "node_path"; args = @("run", "examples\polyglot\node_path_smoke.matter") },
    @{ name = "core-status"; args = @("core-status-json") },
    @{ name = "polyglot-status"; args = @("polyglot-status-json") }
)

$results = @()
foreach ($c in $cases) {
    Write-Host ("==> {0}" -f $c.name) -ForegroundColor Yellow
    $out = & $cli @($c.args) 2>&1 | Out-String
    $code = $LASTEXITCODE
    $ok = ($code -eq 0)
    # JSON commands must also report ok:true when present
    if ($ok -and ($c.name -like "*-status")) {
        try {
            $j = $out | ConvertFrom-Json
            if ($null -ne $j.ok) { $ok = [bool]$j.ok }
        } catch { $ok = $false }
    }
    $tail = (($out.Trim() -split "`r?`n") | Select-Object -Last 4) -join " | "
    $results += [ordered]@{ name = $c.name; ok = $ok; exit = $code; tail = $tail }
    if ($ok) { Write-Host "PASS" -ForegroundColor Green } else { Write-Host "FAIL exit=$code" -ForegroundColor Red; Write-Host $tail }
}

$pass = @($results | Where-Object { $_.ok }).Count
$fail = @($results | Where-Object { -not $_.ok }).Count
$summary = [ordered]@{
    at = (Get-Date).ToString("o")
    cli = $cli
    pass = $pass
    fail = $fail
    total = $results.Count
    results = $results
}

New-Item -ItemType Directory -Force -Path "target\validation" | Out-Null
$summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 "target\validation\real-apps-smoke.json"
Write-Host ("SUMMARY pass={0} fail={1} total={2}" -f $pass, $fail, $results.Count) -ForegroundColor $(if ($fail -eq 0) { "Green" } else { "Yellow" })
Write-Host "Report: target\validation\real-apps-smoke.json"
exit $(if ($fail -eq 0) { 0 } else { 1 })
