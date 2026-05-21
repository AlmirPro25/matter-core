param(
  [int]$Runs = 7,
  [switch]$UseBytecode
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$tests = @(
  @{ key='loop_sum'; name='Loop simples (1..1.000.000)' },
  @{ key='fib30'; name='Fibonacci recursivo fib(30)' },
  @{ key='list_ops_10k'; name='Lista: criar, ordenar, somar 10.000' },
  @{ key='string_concat_10k'; name='String concat 10.000 vezes' }
)
$langs = @('matter','python','node','lua')

function Median($arr) {
  if (-not $arr -or $arr.Count -eq 0) { return $null }
  $s = @($arr | Sort-Object)
  $n = $s.Count
  if ($n % 2 -eq 1) { return [double]$s[[int]($n/2)] }
  return ([double]$s[$n/2 - 1] + [double]$s[$n/2]) / 2.0
}
function CmdExists($n) { return $null -ne (Get-Command $n -ErrorAction SilentlyContinue) }

$pythonCmd = if (CmdExists 'python') { 'python' } elseif (CmdExists 'py') { 'py -3' } else { $null }
$nodeCmd = if (CmdExists 'node') { 'node' } else { $null }
$luaCmd = if (CmdExists 'lua') { 'lua' } else { $null }

$avail = @{ matter=$true; python=[bool]$pythonCmd; node=[bool]$nodeCmd; lua=[bool]$luaCmd }

if ($UseBytecode) {
  New-Item -ItemType Directory -Force -Path 'benchmarks/crosslang/matter_bc' | Out-Null
  foreach($t in $tests){
    $src = "benchmarks/crosslang/matter/$($t.key).matter"
    $out = "benchmarks/crosslang/matter_bc/$($t.key).mbc"
    cargo run -q -p matter-cli -- compile $src -o $out > $null
  }
}

# Warmup 1x each language/test (excluded from measurement)
foreach($t in $tests){
  if ($avail.matter) {
    if ($UseBytecode) {
      cargo run -q -p matter-cli -- run-bytecode "benchmarks/crosslang/matter_bc/$($t.key).mbc" > $null
    } else {
      cargo run -q -p matter-cli -- run "benchmarks/crosslang/matter/$($t.key).matter" > $null
    }
  }
  if ($avail.python) { Invoke-Expression "$pythonCmd benchmarks/crosslang/python/$($t.key).py" | Out-Null }
  if ($avail.node) { & node "benchmarks/crosslang/node/$($t.key).js" | Out-Null }
  if ($avail.lua) { & lua "benchmarks/crosslang/lua/$($t.key).lua" | Out-Null }
}

$raw = @{}
$summary = @{}

foreach($t in $tests){
  $raw[$t.key] = @{}
  $summary[$t.key] = @{}
  foreach($lang in $langs){
    if (-not $avail[$lang]) {
      $raw[$t.key][$lang] = @{ available=$false; runs_ms=@(); median_ms=$null; error='runtime not found' }
      $summary[$t.key][$lang] = $null
      continue
    }

    $runSamples = @(); $errors=@()
    for($i=1; $i -le $Runs; $i++){
      try {
        if ($lang -eq 'matter') {
          if ($UseBytecode) {
            $ms = (Measure-Command { cargo run -q -p matter-cli -- run-bytecode "benchmarks/crosslang/matter_bc/$($t.key).mbc" > $null }).TotalMilliseconds
          } else {
            $ms = (Measure-Command { cargo run -q -p matter-cli -- run "benchmarks/crosslang/matter/$($t.key).matter" > $null }).TotalMilliseconds
          }
          $runSamples += [double]$ms
        } elseif ($lang -eq 'python') {
          $out = Invoke-Expression "$pythonCmd benchmarks/crosslang/python/$($t.key).py"
          $lines = @($out | ? { $_ -and $_.ToString().Trim() -ne '' })
          $runSamples += [double]$lines[-1]
        } elseif ($lang -eq 'node') {
          $out = & node "benchmarks/crosslang/node/$($t.key).js"
          $lines = @($out | ? { $_ -and $_.ToString().Trim() -ne '' })
          $runSamples += [double]$lines[-1]
        } elseif ($lang -eq 'lua') {
          $out = & lua "benchmarks/crosslang/lua/$($t.key).lua"
          $lines = @($out | ? { $_ -and $_.ToString().Trim() -ne '' })
          $runSamples += [double]$lines[-1]
        }
      } catch { $errors += $_.Exception.Message }
    }

    $med = Median $runSamples
    $raw[$t.key][$lang] = @{ available=$true; runs_ms=@($runSamples); median_ms=$med; errors=@($errors) }
    $summary[$t.key][$lang] = $med
  }
}

$rankings = @{}
foreach($t in $tests){
  $pairs=@()
  foreach($lang in $langs){ $m=$summary[$t.key][$lang]; if($null -ne $m){ $pairs += [pscustomobject]@{lang=$lang; median_ms=[double]$m}} }
  $rankings[$t.key] = @($pairs | Sort-Object median_ms)
}

$tag = if($UseBytecode){'fair_bytecode'}else{'fair_source'}
$jsonPath = "benchmarks/benchmark_results_$tag.json"
$mdPath = "benchmarks/benchmark_results_$tag.md"

$data = [ordered]@{
  generated_at=(Get-Date).ToString('o')
  mode=[ordered]@{ fair=$true; runs=$Runs; warmup_per_test=1; matter_mode=$(if($UseBytecode){'bytecode'}else{'source'}) }
  environment=[ordered]@{ python_cmd=$pythonCmd; node_cmd=$nodeCmd; lua_cmd=$luaCmd }
  tests=$raw
  rankings=$rankings
}
$data | ConvertTo-Json -Depth 12 | Set-Content $jsonPath -Encoding UTF8

$lines=@()
$lines += "# Benchmark Results ($tag)"
$lines += ''
$lines += "- Rodadas por teste: $Runs"
$lines += '- Warmup: 1 rodada por linguagem/teste (fora da medicao)'
$lines += "- Matter mode: $(if($UseBytecode){'run-bytecode'}else{'run source'})"
$lines += ''

foreach($t in $tests){
  $lines += "## $($t.name)"
  $lines += ''
  $lines += '| Linguagem | Mediana (ms) | Ranking |'
  $lines += '|---|---:|---:|'
  $ranked=$rankings[$t.key]; $rankMap=@{}; for($i=0;$i -lt $ranked.Count;$i++){$rankMap[$ranked[$i].lang]=$i+1}
  foreach($lang in $langs){
    $entry=$raw[$t.key][$lang]
    if(-not $entry.available){ $lines += "| $lang | N/A | N/A |"; continue }
    $median = '{0:N3}' -f [double]$entry.median_ms
    $rk=$rankMap[$lang]
    $row = "| $lang | $median | $rk |"
    if($lang -eq 'matter' -and $rk -eq 1){ $row += ' **<- Matter ganhou este teste**' }
    $lines += $row
  }
  if($rankMap.ContainsKey('matter') -and $rankMap['matter'] -ne 1){ $lines += ''; $lines += "Observacao: Matter nao liderou este teste (ranking $($rankMap['matter']))." }
  $lines += ''
}

$wins=0;$comp=0
foreach($t in $tests){$ranked=$rankings[$t.key];$map=@{};for($i=0;$i -lt $ranked.Count;$i++){$map[$ranked[$i].lang]=$i+1};if($map.ContainsKey('matter')){$comp++; if($map['matter'] -eq 1){$wins++}}}
$lines += '## O que isso significa'
$lines += ''
$lines += "- Matter venceu $wins de $comp testes comparaveis neste modo fair."
$lines += '- Dados de perda indicam foco em otimizar custo de execucao e overhead do caminho escolhido (source ou bytecode).'

$lines -join "`r`n" | Set-Content $mdPath -Encoding UTF8
Write-Host "Generated $mdPath and $jsonPath"
