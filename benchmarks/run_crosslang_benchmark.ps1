param()

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

$avail = @{
  matter = $true
  python = [bool]$pythonCmd
  node = [bool]$nodeCmd
  lua = [bool]$luaCmd
}

$raw = @{}
$summary = @{}

foreach($t in $tests){
  $raw[$t.key] = @{}
  $summary[$t.key] = @{}

  foreach($lang in $langs){
    $runs = @()
    $errors = @()

    if (-not $avail[$lang]) {
      $raw[$t.key][$lang] = @{ available=$false; runs_ms=@(); median_ms=$null; error='runtime not found' }
      $summary[$t.key][$lang] = $null
      continue
    }

    for($i=1; $i -le 3; $i++){
      try {
        if ($lang -eq 'matter') {
          $path = "benchmarks/crosslang/matter/$($t.key).matter"
          $ms = (Measure-Command { cargo run -q -p matter-cli -- run $path > $null }).TotalMilliseconds
          $runs += [double]$ms
        } elseif ($lang -eq 'python') {
          $path = "benchmarks/crosslang/python/$($t.key).py"
          $cmd = "$pythonCmd $path"
          $out = Invoke-Expression $cmd
          $lines = @($out | Where-Object { $_ -ne $null -and $_.ToString().Trim() -ne '' })
          $ms = [double]$lines[-1]
          $runs += $ms
        } elseif ($lang -eq 'node') {
          $path = "benchmarks/crosslang/node/$($t.key).js"
          $out = & node $path
          $lines = @($out | Where-Object { $_ -ne $null -and $_.ToString().Trim() -ne '' })
          $ms = [double]$lines[-1]
          $runs += $ms
        } elseif ($lang -eq 'lua') {
          $path = "benchmarks/crosslang/lua/$($t.key).lua"
          $out = & lua $path
          $lines = @($out | Where-Object { $_ -ne $null -and $_.ToString().Trim() -ne '' })
          $ms = [double]$lines[-1]
          $runs += $ms
        }
      } catch {
        $errors += $_.Exception.Message
      }
    }

    $median = Median $runs
    $raw[$t.key][$lang] = @{
      available = $true
      runs_ms = @($runs)
      median_ms = $median
      errors = @($errors)
    }
    $summary[$t.key][$lang] = $median
  }
}

$rankings = @{}
foreach($t in $tests){
  $pairs = @()
  foreach($lang in $langs){
    $m = $summary[$t.key][$lang]
    if ($null -ne $m) { $pairs += [pscustomobject]@{ lang=$lang; median_ms=[double]$m } }
  }
  $rankings[$t.key] = @($pairs | Sort-Object median_ms)
}

$data = [ordered]@{
  generated_at = (Get-Date).ToString('o')
  environment = [ordered]@{
    powershell = $PSVersionTable.PSVersion.ToString()
    python_cmd = $pythonCmd
    node_cmd = $nodeCmd
    lua_cmd = $luaCmd
  }
  tests = $raw
  rankings = $rankings
}

$data | ConvertTo-Json -Depth 12 | Set-Content 'benchmarks/benchmark_results.json' -Encoding UTF8

$lines = @()
$lines += '# Benchmark Results: Matter vs Python vs Node vs Lua'
$lines += ''
$lines += '- Rodadas por teste: 3'
$lines += '- Metrica usada: mediana (ms)'
$lines += '- Matter medido com `Measure-Command` no PowerShell'
$lines += '- Python/Node/Lua medidos com temporizador nativo dentro do codigo'
$lines += ''

foreach($t in $tests){
  $lines += "## $($t.name)"
  $lines += ''
  $lines += '| Linguagem | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Mediana (ms) | Ranking |'
  $lines += '|---|---:|---:|---:|---:|---:|'

  $ranked = $rankings[$t.key]
  $rankMap = @{}
  for($i=0; $i -lt $ranked.Count; $i++){ $rankMap[$ranked[$i].lang] = $i + 1 }

  foreach($lang in $langs){
    $entry = $raw[$t.key][$lang]
    if (-not $entry.available) {
      $lines += "| $lang | - | - | - | N/A | N/A |"
      continue
    }
    $r = @($entry.runs_ms)
    while($r.Count -lt 3){ $r += $null }
    $median = if ($null -eq $entry.median_ms) { 'N/A' } else { ('{0:N3}' -f [double]$entry.median_ms) }
    $rk = if ($rankMap.ContainsKey($lang)) { $rankMap[$lang] } else { 'N/A' }
    $line = "| $lang | {0} | {1} | {2} | {3} | {4} |" -f `
      ($(if($null -ne $r[0]){('{0:N3}' -f [double]$r[0])}else{'-'})), `
      ($(if($null -ne $r[1]){('{0:N3}' -f [double]$r[1])}else{'-'})), `
      ($(if($null -ne $r[2]){('{0:N3}' -f [double]$r[2])}else{'-'})), `
      $median, $rk
    if ($lang -eq 'matter' -and $rankMap.ContainsKey('matter') -and $rankMap['matter'] -eq 1) {
      $line += '  **<- Matter ganhou este teste**'
    }
    $lines += $line
  }

  if ($rankMap.ContainsKey('matter') -and $rankMap['matter'] -ne 1) {
    $lines += ''
    $lines += "Observacao: Matter nao liderou este teste (ranking $($rankMap['matter']))."
  }

  $lines += ''
}

$matterWins = 0
$matterTotalComparable = 0
foreach($t in $tests){
  $rankMap = @{}
  $ranked = $rankings[$t.key]
  for($i=0; $i -lt $ranked.Count; $i++){ $rankMap[$ranked[$i].lang] = $i + 1 }
  if ($rankMap.ContainsKey('matter')) {
    $matterTotalComparable += 1
    if ($rankMap['matter'] -eq 1) { $matterWins += 1 }
  }
}

$lines += '## O que isso significa'
$lines += ''
$lines += "- Matter venceu $matterWins de $matterTotalComparable testes comparaveis nesta maquina/ambiente."
$lines += '- Se Matter ganhou em algum teste, isso indica que o caminho VM/execucao atual esta competitivo para esse padrao de carga.'
$lines += '- Onde Matter perdeu, os dados apontam oportunidade de otimizar runtime, chamadas de backend e custo de inicializacao do CLI.'
$lines += '- Compare tambem variancia entre runs: alta variacao sugere ruido de ambiente e necessidade de mais iteracoes para decisao de tuning.'
$lines += '- Este benchmark e um retrato local (hardware/OS atual); para decisoes de produto, rode em CI padronizado com maquina dedicada.'

$lines -join "`r`n" | Set-Content 'benchmarks/benchmark_results.md' -Encoding UTF8
Write-Host 'Generated benchmarks/benchmark_results.md and benchmarks/benchmark_results.json'
