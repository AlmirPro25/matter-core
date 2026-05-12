# Run All Benchmarks
# PowerShell script para executar todos os benchmarks

Write-Host "==================================" -ForegroundColor Cyan
Write-Host "Matter Core - Benchmark Suite" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

$benchmarks = @(
    "fibonacci.matter",
    "fibonacci_iterative.matter",
    "sum_array.matter",
    "nested_loops.matter",
    "function_calls.matter",
    "loop_intensive.matter",
    "data_structures.matter",
    "backend_calls.matter",
    "stress_test.matter"
)

$results = @()

foreach ($benchmark in $benchmarks) {
    Write-Host "Running: $benchmark" -ForegroundColor Yellow
    Write-Host "---" -ForegroundColor Gray
    
    $time = (Measure-Command {
        & ..\target\release\matter-cli.exe run $benchmark | Out-Host
    }).TotalMilliseconds
    
    $results += [PSCustomObject]@{
        Benchmark = $benchmark
        Time_ms = [math]::Round($time, 2)
    }
    
    Write-Host ""
    Write-Host "Duration: $([math]::Round($time, 2)) ms" -ForegroundColor Green
    Write-Host ""
    Write-Host "==================================" -ForegroundColor Cyan
    Write-Host ""
}

Write-Host ""
Write-Host "==================================" -ForegroundColor Cyan
Write-Host "BENCHMARK SUMMARY" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

$results | Format-Table -AutoSize

$totalTime = ($results | Measure-Object -Property Time_ms -Sum).Sum
Write-Host ""
Write-Host "Total Time: $([math]::Round($totalTime, 2)) ms" -ForegroundColor Green
Write-Host "Average Time: $([math]::Round($totalTime / $results.Count, 2)) ms" -ForegroundColor Green
Write-Host ""
Write-Host "All benchmarks completed!" -ForegroundColor Green
