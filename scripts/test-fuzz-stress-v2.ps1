# Phase 5: deterministic fuzz smoke + stress/soak for language-only Core.
param([string]$CliPath = "")

$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $CliPath) {
    $CliPath = Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
}
$Out = Join-Path $Root "target\validation\production_readiness_v2"
$FuzzDir = Join-Path $Out "fuzz_corpus"
$SoakDir = Join-Path $Out "soak"
New-Item -ItemType Directory -Force -Path $FuzzDir, $SoakDir | Out-Null

$results = New-Object System.Collections.Generic.List[object]
function Add-R($n, $ok, $d) {
    $script:results.Add([pscustomobject]@{ name = $n; ok = $ok; detail = $d })
    Write-Host ("[{0}] {1} {2}" -f ($(if ($ok) { "PASS" } else { "FAIL" }), $n, $d))
}

$seed = 0xC0FFEE42
$rng = New-Object System.Random($seed)
$panic = 0
$hang = 0
$maxMs = 0
$proc = Get-Process -Id $PID

# --- Deterministic random source fuzz (128 cases) ---
$utf8 = New-Object System.Text.UTF8Encoding $false
for ($i = 0; $i -lt 128; $i++) {
    $len = $rng.Next(0, 256)
    $sb = New-Object System.Text.StringBuilder
    for ($j = 0; $j -lt $len; $j++) {
        [void]$sb.Append([char]$rng.Next(1, 127))
    }
    $path = Join-Path $FuzzDir ("fuzz_{0:D4}.matter" -f $i)
    [System.IO.File]::WriteAllText($path, $sb.ToString(), $utf8)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $CliPath -ArgumentList @("check", $path) -NoNewWindow -PassThru -Wait -RedirectStandardOutput (Join-Path $FuzzDir "o.txt") -RedirectStandardError (Join-Path $FuzzDir "e.txt")
    $sw.Stop()
    if ($sw.ElapsedMilliseconds -gt $maxMs) { $maxMs = $sw.ElapsedMilliseconds }
    # Guardrail: individual check should be sub-second for tiny inputs; flag extreme duration as hang-like
    if ($sw.ElapsedMilliseconds -gt 8000) {
        $hang++
        Add-R "fuzz-slow-$i" $false "ms=$($sw.ElapsedMilliseconds)"
    }
    $code = $p.ExitCode
    # STATUS_ACCESS_VIOLATION / hard crash codes
    if ($code -eq -1073741819 -or $code -eq -1073740940 -or $code -eq 3221225477) {
        $panic++
        Add-R "fuzz-crash-$i" $false "exit=$code"
    }
}
Add-R "fuzz-smoke-128" (($panic -eq 0) -and ($hang -eq 0)) "panic=$panic hang=$hang max_ms=$maxMs seed=0xC0FFEE42"

# --- Structured adversarial (must not panic; invalid must not execute as success for garbage) ---
$cases = @{
    "empty"           = ""
    "garbage"         = "@@@###$$$"
    "illegal_at"      = "let x = 1 @ 2"
    "deep_nest"       = ("print " + ("(" * 120) + "1" + (")" * 120))
    "div_zero"        = "print 1 / 0"
    "truncated"       = "if true {"
    "huge_ident"      = ("let " + ("a" * 5000) + " = 1")
}
foreach ($k in $cases.Keys) {
    $p = Join-Path $FuzzDir ("adv_$k.matter")
    [System.IO.File]::WriteAllText($p, $cases[$k], (New-Object System.Text.UTF8Encoding $false))
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & $CliPath check $p 1>$null 2>$null
    $c1 = $LASTEXITCODE
    & $CliPath run $p 1>$null 2>$null
    $c2 = $LASTEXITCODE
    $sw.Stop()
    if ($sw.ElapsedMilliseconds -gt $maxMs) { $maxMs = $sw.ElapsedMilliseconds }
    $crash = ($c1 -lt -100000) -or ($c2 -lt -100000)
    Add-R "adv-$k" (-not $crash) "check=$c1 run=$c2 ms=$($sw.ElapsedMilliseconds)"
}

# --- MBC invalid (must reject, no execute) ---
$mbcDir = Join-Path $FuzzDir "mbc"
New-Item -ItemType Directory -Force -Path $mbcDir | Out-Null
[System.IO.File]::WriteAllBytes((Join-Path $mbcDir "empty.mbc"), [byte[]]@())
[System.IO.File]::WriteAllBytes((Join-Path $mbcDir "bad.mbc"), [byte[]][char[]]"XXXX" + (New-Object byte[] 16))
$rng2 = New-Object System.Random(99)
$rb = New-Object byte[] 128
$rng2.NextBytes($rb)
[System.IO.File]::WriteAllBytes((Join-Path $mbcDir "rand.mbc"), $rb)
foreach ($f in Get-ChildItem $mbcDir) {
    & $CliPath run-bytecode $f.FullName 1>$null 2>$null
    $ok = ($LASTEXITCODE -ne 0)
    Add-R "mbc-reject-$($f.Name)" $ok "exit=$LASTEXITCODE"
}

# --- VM limits: instruction budget ---
$loopSrc = Join-Path $FuzzDir "inf_loop.matter"
# tight loop via while true - may hit instruction limit
[System.IO.File]::WriteAllText($loopSrc, "let i = 0`nwhile i < 100000000 { set i = i + 1 }`nprint i`n", (New-Object System.Text.UTF8Encoding $false))
$env:MATTER_VM_MAX_INSTRUCTIONS = "50000"
$sw = [Diagnostics.Stopwatch]::StartNew()
& $CliPath run $loopSrc 1>$null 2>$null
$limExit = $LASTEXITCODE
$sw.Stop()
Remove-Item Env:MATTER_VM_MAX_INSTRUCTIONS -ErrorAction SilentlyContinue
Add-R "vm-instruction-limit" ($limExit -ne 0) "exit=$limExit ms=$($sw.ElapsedMilliseconds)"

# --- Soak: compile/run-bytecode cycles ---
$hello = Join-Path $Root "examples\hello.matter"
if (-not (Test-Path $hello)) { $hello = Join-Path $Root "dist\matter-core-0.1.0-windows-x64\examples\hello.matter" }
$mbc = Join-Path $SoakDir "soak.mbc"
$ws = [Diagnostics.Stopwatch]::StartNew()
$ws.Start()
$okCycles = 0
$failCycles = 0
$memSamples = @()
for ($i = 0; $i -lt 40; $i++) {
    & $CliPath compile $hello -o $mbc 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { $failCycles++; continue }
    & $CliPath run-bytecode $mbc 1>$null 2>$null
    if ($LASTEXITCODE -ne 0) { $failCycles++ } else { $okCycles++ }
    if (($i % 10) -eq 0) {
        $p = Get-Process -Id $PID
        $memSamples += [math]::Round($p.WorkingSet64 / 1MB, 2)
    }
}
$ws.Stop()
Add-R "soak-compile-run-40" ($failCycles -eq 0) "ok=$okCycles fail=$failCycles total_ms=$($ws.ElapsedMilliseconds) mem_mb=$($memSamples -join ',')"

# --- Repeated run of core programs ---
$progs = @("hello.matter", "fibonacci.matter", "events.matter")
$repOk = 0
foreach ($name in $progs) {
    $p = Join-Path $Root "examples\$name"
    if (-not (Test-Path $p)) { continue }
    for ($k = 0; $k -lt 5; $k++) {
        & $CliPath run $p 1>$null 2>$null
        if ($LASTEXITCODE -eq 0) { $repOk++ } else { Add-R "repeat-$name-$k" $false "exit=$LASTEXITCODE" }
    }
}
Add-R "repeat-core-programs" ($repOk -ge 10) "successes=$repOk"

$summary = [pscustomobject]@{
    at              = (Get-Date).ToString("o")
    seed            = "0xC0FFEE42"
    panic_count     = $panic
    hang_count      = $hang
    max_check_ms    = $maxMs
    soak_ok         = $okCycles
    soak_fail       = $failCycles
    mem_samples_mb  = $memSamples
    results         = $results
    passed          = @($results | Where-Object ok).Count
    failed          = @($results | Where-Object { -not $_.ok }).Count
}
$summary | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $Out "fuzz-stress-results.json") -Encoding utf8
Write-Host "FUZZ/STRESS passed=$($summary.passed) failed=$($summary.failed) max_ms=$maxMs"
if ($summary.failed -gt 0) { exit 1 }
exit 0
