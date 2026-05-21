Write-Host "========================================"
Write-Host "  BENCHMARK: GOLANG vs MATTER CORE      "
Write-Host "========================================"
Write-Host ""

Write-Host ">>> Compilando Go..."
go build -o benchmark_go.exe benchmark.go
Write-Host ">>> Executando Golang..."
$go_time = Measure-Command { .\benchmark_go.exe | Out-Default }
Write-Host ">>> Golang executou em: $($go_time.TotalSeconds) segundos."
Write-Host ""

Write-Host ">>> Executando Matter Core Nativo (LLVM O3)..."
$matter_time = Measure-Command { .\output.exe | Out-Default }
Write-Host ">>> Matter executou em: $($matter_time.TotalSeconds) segundos."
Write-Host ""
Write-Host "========================================"
$ratio = $go_time.TotalSeconds / $matter_time.TotalSeconds
Write-Host "RESULTADO: O Matter Core executou em $([math]::Round($matter_time.TotalSeconds, 4))s e o Go em $([math]::Round($go_time.TotalSeconds, 4))s."
Write-Host "========================================"
