# Phase 2 core-only suite (no polyglot / visual / agent / network)
param([string]$Cli = "")
$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $Cli) {
  # Prefer freshest binary (release first by mtime); never silent-stale gnu-only default.
  $cands = @()
  foreach ($p in @(
    (Join-Path $Root "target\release\matter-cli.exe"),
    (Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe")
  )) {
    if (Test-Path -LiteralPath $p) {
      $i = Get-Item -LiteralPath $p
      $cands += [pscustomobject]@{ path = $p; mtime = $i.LastWriteTimeUtc; sha = (Get-FileHash $p -Algorithm SHA256).Hash }
    }
  }
  if ($cands.Count -eq 0) {
    Write-Error "CLI not found under target\release or target\x86_64-pc-windows-gnu\release"
    exit 2
  }
  $best = $cands | Sort-Object mtime -Descending | Select-Object -First 1
  $Cli = $best.path
  Write-Host ("CLI selected: {0} sha256={1}" -f $Cli, $best.sha)
}
if (-not (Test-Path -LiteralPath $Cli)) {
  Write-Error "CLI not found: $Cli"
  exit 2
}
$OutDir = Join-Path $Root "target\validation\phase_2_core_hardening"
$FixtureSrc = Join-Path $Root "tests\fixtures\invalid\source"
$FixtureMbc = Join-Path $Root "tests\fixtures\invalid\mbc"
$invDir = Join-Path $OutDir "corpus_invalid"
$mbcDir = Join-Path $OutDir "corpus_mbc"
New-Item -ItemType Directory -Force -Path $OutDir,$invDir,$mbcDir | Out-Null

# Sync canonical fixtures -> validation evidence copies
if (Test-Path -LiteralPath $FixtureSrc) {
  Copy-Item -LiteralPath (Join-Path $FixtureSrc "*") -Destination $invDir -Force
}
if (Test-Path -LiteralPath $FixtureMbc) {
  Copy-Item -LiteralPath (Join-Path $FixtureMbc "*") -Destination $mbcDir -Force
}

$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$name, [int]$exitCode, [bool]$ok, [string]$detail) {
  $script:results.Add([pscustomobject]@{ name=$name; exit=$exitCode; ok=$ok; detail=$detail })
  $status = if ($ok) { "PASS" } else { "FAIL" }
  Write-Host "[$status] $name (exit=$exitCode) $detail"
}

$valid = @(
  @{ name="hello"; path="examples\hello.matter" },
  @{ name="fibonacci"; path="examples\fibonacci.matter" },
  @{ name="events"; path="examples\events.matter" },
  @{ name="agent_policy"; path="examples\agent_policy_demo.matter" }
)

foreach ($v in $valid) {
  $p = Join-Path $Root $v.path
  if (-not (Test-Path -LiteralPath $p)) {
    Add-Result $v.name -1 $false "missing $($v.path)"
    continue
  }
  $null = & $Cli run $p 2>&1
  Add-Result "run-$($v.name)" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
  $null = & $Cli check $p 2>&1
  Add-Result "check-$($v.name)" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
}

$hello = Join-Path $Root "examples\hello.matter"
$mbcOut = Join-Path $OutDir "hello-phase2.mbc"
$null = & $Cli compile $hello -o $mbcOut 2>&1
Add-Result "compile-hello" $LASTEXITCODE ($LASTEXITCODE -eq 0) $mbcOut
if ($LASTEXITCODE -eq 0) {
  $null = & $Cli run-bytecode $mbcOut 2>&1
  Add-Result "run-bytecode-hello" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
}

Get-ChildItem -LiteralPath $invDir -Filter "*.matter" -ErrorAction SilentlyContinue | ForEach-Object {
  $null = & $Cli check $_.FullName 2>&1
  $code = $LASTEXITCODE
  $json = (& $Cli check-json $_.FullName 2>&1 | Out-String)
  $jsonOkFalse = $json -match '"ok"\s*:\s*false'
  Add-Result "reject-$($_.BaseName)" $code (($code -ne 0) -and $jsonOkFalse) "json_ok_false=$jsonOkFalse"
  $errFile = Join-Path $OutDir ("stderr-" + $_.BaseName + ".txt")
  $null = & $Cli run $_.FullName 1>$null 2>$errFile
  $rcode = $LASTEXITCODE
  $stderrLen = 0
  if (Test-Path -LiteralPath $errFile) { $stderrLen = (Get-Item -LiteralPath $errFile).Length }
  Add-Result "run-fail-$($_.BaseName)" $rcode (($rcode -ne 0) -and ($stderrLen -gt 0)) "stderr_bytes=$stderrLen"
}

Get-ChildItem -LiteralPath $mbcDir -ErrorAction SilentlyContinue | ForEach-Object {
  $null = & $Cli run-bytecode $_.FullName 1>$null 2>$null
  Add-Result "mbc-reject-$($_.Name)" $LASTEXITCODE ($LASTEXITCODE -ne 0) ""
  $json = (& $Cli run-bytecode-json $_.FullName 2>&1 | Out-String)
  $jcode = $LASTEXITCODE
  $jsonOkFalse = $json -match '"ok"\s*:\s*false'
  Add-Result "mbc-json-reject-$($_.Name)" $jcode (($jcode -ne 0) -and $jsonOkFalse) "json_ok_false=$jsonOkFalse"
}

$sys32 = Join-Path $env:SystemRoot "System32"
$oldPath = $env:PATH
try {
  $env:PATH = $sys32
  $null = & $Cli --version 1>$null 2>$null
  Add-Result "minimal-path-version" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
  $null = & $Cli core-status-json 1>$null 2>$null
  Add-Result "minimal-path-core-status" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
} finally {
  $env:PATH = $oldPath
}

$seed = 0xC0FFEE
$rng = New-Object System.Random $seed
$fuzzFail = 0
$fuzzOk = 0
for ($i=0; $i -lt 32; $i++) {
  $len = $rng.Next(0, 64)
  $sb = New-Object System.Text.StringBuilder
  for ($j=0; $j -lt $len; $j++) {
    [void]$sb.Append([char](32 + $rng.Next(0, 95)))
  }
  $tmp = Join-Path $OutDir ("fuzz_" + $i + ".matter")
  Set-Content -LiteralPath $tmp -Value $sb.ToString() -Encoding utf8
  $null = & $Cli check $tmp 1>$null 2>$null
  if ($LASTEXITCODE -gt 2) { $fuzzFail++ } else { $fuzzOk++ }
  Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
}
Add-Result "fuzz-smoke" 0 ($fuzzFail -eq 0) "ok=$fuzzOk fail_crash=$fuzzFail"

$failed = @($results | Where-Object { -not $_.ok }).Count
$passed = @($results | Where-Object { $_.ok }).Count
$summary = [pscustomobject]@{
  at = (Get-Date).ToString("o")
  cli = $Cli
  passed = $passed
  failed = $failed
  total = $results.Count
  results = $results
}
$summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutDir "core-suite-results.json") -Encoding utf8
Write-Host "CORE SUITE: passed=$passed failed=$failed total=$($results.Count)"
if ($failed -gt 0) { exit 1 } else { exit 0 }

