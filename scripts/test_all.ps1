# Test: Full Matter Core validation
Write-Host "=== Matter Core: Validacao Completa ===" -ForegroundColor Cyan
Write-Host ""

$allPassed = $true

function Run-Step($name, [scriptblock]$command) {
    Write-Host "==> $name" -ForegroundColor White
    & $command
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        Write-Host "FALHOU: $name (exit code $exitCode)" -ForegroundColor Red
        $script:allPassed = $false
    }
    else {
        Write-Host "PASSOU: $name" -ForegroundColor Green
    }

    Write-Host ""
}

Run-Step "cargo test --workspace" {
    cargo test --workspace
}

Run-Step "cargo build --release --workspace" {
    cargo build --release --workspace
}

Run-Step "bytecode equivalence" {
    powershell -ExecutionPolicy Bypass -File .\test_bytecode_equivalence.ps1
}

Run-Step "api bridge json contract" {
    powershell -ExecutionPolicy Bypass -File .\test_api_bridge.ps1
}

if ($allPassed) {
    Write-Host "=== Validacao completa passou ===" -ForegroundColor Green
    exit 0
}
else {
    Write-Host "=== Validacao completa falhou ===" -ForegroundColor Red
    exit 1
}
