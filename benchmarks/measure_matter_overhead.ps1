param(
  [string]$ProgramPath = "benchmarks/crosslang/matter/loop_sum.matter",
  [int]$Runs = 7
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Median($arr) {
  $s = @($arr | Sort-Object)
  $n = $s.Count
  if ($n % 2 -eq 1) { return [double]$s[[int]($n/2)] }
  return ([double]$s[$n/2 - 1] + [double]$s[$n/2]) / 2.0
}

$meta = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$targetDir = $meta.target_directory
$exe = Join-Path (Join-Path $targetDir 'debug') 'matter-cli.exe'

if (-not (Test-Path $exe)) {
  cargo build -p matter-cli > $null
}

$mbcDir = 'benchmarks/crosslang/matter_bc'
New-Item -ItemType Directory -Force -Path $mbcDir | Out-Null
$mbc = Join-Path $mbcDir 'loop_sum.mbc'
& $exe compile $ProgramPath -o $mbc > $null

# Warmup (excluded)
& cargo run -q -p matter-cli -- run $ProgramPath > $null
& $exe run $ProgramPath > $null
& $exe run-bytecode $mbc > $null

$cargoRun = @()
$binRun = @()
$binRunBytecode = @()

for($i=0; $i -lt $Runs; $i++){
  $cargoRun += (Measure-Command { cargo run -q -p matter-cli -- run $ProgramPath > $null }).TotalMilliseconds
}

for($i=0; $i -lt $Runs; $i++){
  $binRun += (Measure-Command { & $exe run $ProgramPath > $null }).TotalMilliseconds
  $binRunBytecode += (Measure-Command { & $exe run-bytecode $mbc > $null }).TotalMilliseconds
}

$benchJson = & $exe benchmark-json $ProgramPath --iterations $Runs | ConvertFrom-Json
$vmInternalMs = [double]$benchJson.bytecode.stats.median_ns / 1000000.0

$cargoMedian = Median $cargoRun
$binMedian = Median $binRun
$binBcMedian = Median $binRunBytecode

$result = [ordered]@{
  generated_at = (Get-Date).ToString('o')
  program = $ProgramPath
  runs = $Runs
  timings_ms = [ordered]@{
    cargo_run_source_median = $cargoMedian
    cli_binary_source_median = $binMedian
    cli_binary_bytecode_median = $binBcMedian
    vm_internal_benchmark_median = $vmInternalMs
  }
  overhead_estimates_ms = [ordered]@{
    cargo_wrapper_overhead = ($cargoMedian - $binMedian)
    cli_startup_plus_compile_overhead = ($binMedian - $vmInternalMs)
    cli_startup_overhead_on_bytecode = ($binBcMedian - $vmInternalMs)
  }
  raw_runs_ms = [ordered]@{
    cargo_run_source = $cargoRun
    cli_binary_source = $binRun
    cli_binary_bytecode = $binRunBytecode
  }
}

$result | ConvertTo-Json -Depth 8 | Set-Content benchmarks/overhead_breakdown.json -Encoding UTF8

$md = @()
$md += '# Matter Overhead Breakdown'
$md += ''
$md += ('- Programa: `{0}`' -f $ProgramPath)
$md += "- Rodadas por medicao: $Runs"
$md += ''
$md += '| Camada | Mediana (ms) |'
$md += '|---|---:|'
$md += ('| cargo run + CLI + compile + VM | {0:N3} |' -f $cargoMedian)
$md += ('| CLI binario + compile + VM | {0:N3} |' -f $binMedian)
$md += ('| CLI binario + bytecode + VM | {0:N3} |' -f $binBcMedian)
$md += ('| VM (benchmark interno) | {0:N3} |' -f $vmInternalMs)
$md += ''
$md += '## Separacao estimada'
$md += ''
$md += ('- Overhead do `cargo run`: **{0:N3} ms**' -f ($cargoMedian - $binMedian))
$md += ('- Overhead de startup+compile no CLI: **{0:N3} ms**' -f ($binMedian - $vmInternalMs))
$md += ('- Overhead de startup sobre bytecode: **{0:N3} ms**' -f ($binBcMedian - $vmInternalMs))
$md += ''
$md += '## Nota metodologica'
$md += ''
$md += '- O valor "VM (benchmark interno)" vem de `benchmark-json` e representa o miolo de execucao medido dentro do processo.'
$md += '- A separacao e uma estimativa pratica para orientar otimização (startup/processo vs loop da VM).'

$md -join "`r`n" | Set-Content benchmarks/overhead_breakdown.md -Encoding UTF8
Write-Host 'Generated benchmarks/overhead_breakdown.md and benchmarks/overhead_breakdown.json'
