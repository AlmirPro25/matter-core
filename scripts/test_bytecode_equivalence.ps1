param(
    [string]$CliPath = ".\target\release\matter-cli.exe"
)

# Test: Bytecode Equivalence
Write-Host "=== Teste de Equivalência de Bytecode ===" -ForegroundColor Cyan
Write-Host ""

$testFiles = @(
    "examples\test_loops.matter",
    "examples\test_for.matter",
    "examples\test_imports.matter",
    "examples\test_stdlib.matter",
    "examples\test_store.matter",
    "examples\test_spawn.matter",
    "examples\test_lists.matter",
    "examples\test_maps.matter",
    "examples\test_structs.matter",
    "examples\test_functions.matter",
    "examples\test_recursion.matter",
    "examples\simple.matter"
)

$allPassed = $true
$outputDir = "target\bytecode_equivalence"

if (-not (Test-Path $CliPath)) {
    Write-Host "CLI nao encontrada em $CliPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

foreach ($file in $testFiles) {
    if (-not (Test-Path $file)) {
        Write-Host "Arquivo não encontrado: $file" -ForegroundColor Yellow
        continue
    }
    
    Write-Host "Testando: $file" -ForegroundColor White
    
    # 1. Executar source
    Write-Host "  1. Executando source..."
    $sourceOutput = & $CliPath run $file 2>&1 | Out-String
    
    # 2. Compilar para bytecode
    $bytecodeFile = Join-Path $outputDir ([System.IO.Path]::GetFileNameWithoutExtension($file) + ".mbc")
    Write-Host "  2. Compilando para $bytecodeFile..."
    & $CliPath compile $file -o $bytecodeFile 2>&1 | Out-Null
    
    # 3. Executar bytecode
    Write-Host "  3. Executando bytecode..."
    $bytecodeOutput = & $CliPath run-bytecode $bytecodeFile 2>&1 | Out-String
    
    # 4. Comparar outputs
    Write-Host "  4. Comparando outputs..."
    if ($sourceOutput -eq $bytecodeOutput) {
        Write-Host "  PASSOU" -ForegroundColor Green
    }
    else {
        Write-Host "  FALHOU" -ForegroundColor Red
        Write-Host "  Source output:"
        Write-Host $sourceOutput
        Write-Host "  Bytecode output:"
        Write-Host $bytecodeOutput
        $allPassed = $false
    }
    
    # Limpar
    if (Test-Path $bytecodeFile) {
        Remove-Item -LiteralPath $bytecodeFile -ErrorAction SilentlyContinue
    }
    
    Write-Host ""
}

Remove-Item -LiteralPath $outputDir -Recurse -ErrorAction SilentlyContinue

if ($allPassed) {
    Write-Host "=== Todos os testes passaram! ===" -ForegroundColor Green
    exit 0
}
else {
    Write-Host "=== Alguns testes falharam ===" -ForegroundColor Red
    exit 1
}
