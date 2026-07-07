# ============================================================================
# Matter Core - Frontier Paradigms Test System (Pure ASCII Encoding Safe)
# ============================================================================
# Este script compila o matter-cli e executa todos os scripts demo do Matter,
# validando o funcionamento dos backends fisicos, biologicos, quimicos e quanticos.
# ============================================================================

$ErrorActionPreference = "Stop"

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " >>> INICIALIZANDO SISTEMA DE TESTES DE FRONTEIRA - MATTER CORE" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "* Consultando status honesto dos backends frontier..." -ForegroundColor Yellow
$frontierStatusRaw = & powershell -ExecutionPolicy Bypass -File .\scripts\test-frontier-status-contract.ps1 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] contrato frontier-status-json falhou" -ForegroundColor Red
    Write-Host $frontierStatusRaw -ForegroundColor DarkRed
    exit 1
}

Write-Host "[OK] Backends frontier: funcionais, nao-stub, simulados, sem hardware real" -ForegroundColor Green
Write-Host ""

# Demos a serem testadas
$demos = @(
    @{ File = "organoid_test.matter";      Name = "Wetware (Organoides Cerebrais)";   Desc = "Simulacao biologica e feedback de dopamina" }
    @{ File = "epr_demo.matter";           Name = "Quantum (Paradoxo EPR)";           Desc = "Emaranhamento quantico e Lorentz-boost" }
    @{ File = "chemistry_demo.matter";     Name = "Chemistry (Orbital Overlap)";      Desc = "Reacoes estequiometricas e RREF quantica" }
    @{ File = "genesis_demo.matter";       Name = "Genesis (Schrodinger-Schwarzschild)";Desc = "Mecanica Quantica em espaco-tempo curvo" }
    @{ File = "relativity_demo.matter";    Name = "Relativity (Boost de Lorentz)";     Desc = "Simulacao cinematica relativistica" }
    @{ File = "neuromorphic_demo.matter";  Name = "Neuromorphic (SNN Simulada)";      Desc = "Rede neural baseada em spikes, sem hardware real" }
    @{ File = "closed_loop_demo.matter";   Name = "Closed-Loop Cybernetics";          Desc = "Feedback hibrido biologico e de silicio" }
)

$results = @()
$totalStart = [System.Diagnostics.Stopwatch]::StartNew()

foreach ($demo in $demos) {
    Write-Host "----------------------------------------------------------------" -ForegroundColor Gray
    Write-Host "* Executando: $($demo.Name) [$($demo.File)]" -ForegroundColor Yellow
    Write-Host "   Descricao: $($demo.Desc)" -ForegroundColor DarkGray
    Write-Host ""
    
    $demoStart = [System.Diagnostics.Stopwatch]::StartNew()
    $failed = $false
    $output = ""
    
    try {
        # Executa o arquivo usando a CLI do Matter via cargo run (quieto)
        $output = & cargo run -q -p matter-cli -- run $demo.File 2>&1
        $demoStart.Stop()
        
        if ($LASTEXITCODE -ne 0) {
            $failed = $true
        }
    }
    catch {
        $failed = $true
        $output = $_.Exception.Message
        $demoStart.Stop()
    }
    
    $runTime = [Math]::Round($demoStart.Elapsed.TotalSeconds, 2)
    
    # Exibe saida abreviada ou status
    if ($failed) {
        Write-Host "[FAIL] FALHA ao rodar $($demo.File)" -ForegroundColor Red
        Write-Host "Erro: $output" -ForegroundColor DarkRed
        $results += [PSCustomObject]@{ Demo = $demo.Name; File = $demo.File; Status = "FAIL"; Duration = "$($runTime)s" }
    } else {
        # Mostrar as primeiras e ultimas linhas do output para verificar sucesso
        $lines = $output -split "`r?`n"
        if ($lines.Length -gt 15) {
            $lines | Select-Object -First 6 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
            Write-Host "   ... [saida omitida para simplificar] ..." -ForegroundColor DarkGray
            $lines | Select-Object -Last 6 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
        } else {
            $lines | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
        }
        
        Write-Host "[OK] Sucesso em $($runTime)s" -ForegroundColor Green
        $results += [PSCustomObject]@{ Demo = $demo.Name; File = $demo.File; Status = "PASS"; Duration = "$($runTime)s" }
    }
    Write-Host ""
}

$totalStart.Stop()
$totalTime = [Math]::Round($totalStart.Elapsed.TotalSeconds, 2)

# 3. Sumario de Validacao das Fronteiras
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " *** SUMARIO DE VALIDACAO DAS PARADIGMAS DE FRONTEIRA" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host ""

$anyFail = $false
foreach ($res in $results) {
    $statusColor = if ($res.Status -eq "PASS") { "Green" } else { "Red" }
    Write-Host -NoNewline "[$($res.Status)] " -ForegroundColor $statusColor
    Write-Host -NoNewline "$($res.Demo) ($($res.File))" -ForegroundColor White
    Write-Host " - $($res.Duration)" -ForegroundColor DarkGray
    if ($res.Status -eq "FAIL") {
        $anyFail = $true
    }
}
Write-Host ""
Write-Host "Tempo total de execucao: $($totalTime)s" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

if ($anyFail) {
    Write-Host "[FAIL] Validacao das fronteiras falhou!" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[SUCCESS] Todos os paradigmas frontier simulados foram validados!" -ForegroundColor Green
    exit 0
}
