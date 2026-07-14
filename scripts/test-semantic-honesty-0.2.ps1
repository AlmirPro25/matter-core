# Matter 0.2.0 — Semantic Honesty + File Capabilities v1 permanent suite
param([string]$Cli = "")
$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $Cli) {
  # Prefer host default target/release (current toolchain is already *-windows-gnu).
  $cand = @(
    (Join-Path $Root "target\release\matter-cli.exe"),
    (Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe")
  )
  foreach ($c in $cand) { if (Test-Path -LiteralPath $c) { $Cli = $c; break } }
}
if (-not $Cli -or -not (Test-Path -LiteralPath $Cli)) {
  Write-Error "CLI not found. Build: cargo build -p matter-cli --release"
  exit 2
}

$OutDir = Join-Path $Root "target\validation\matter_0_2_semantic_honesty"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$name, [bool]$ok, [string]$detail) {
  $script:results.Add([pscustomobject]@{ name = $name; ok = $ok; detail = $detail })
  $status = if ($ok) { "PASS" } else { "FAIL" }
  Write-Host ("[{0}] {1} - {2}" -f $status, $name, $detail)
}

function Invoke-Cli {
  param([string[]]$ArgList)
  $raw = & $Cli @ArgList 2>&1
  $lines = foreach ($item in @($raw)) {
    if ($null -eq $item) { continue }
    if ($item -is [System.Management.Automation.ErrorRecord]) {
      [string]$item.ToString()
    } else {
      [string]$item
    }
  }
  $out = ($lines -join [Environment]::NewLine)
  return @{ exit = $LASTEXITCODE; out = $out }
}

function Write-Matter([string]$path, [string]$body) {
  $utf8NoBom = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText($path, $body, $utf8NoBom)
}

# --- Semantic honesty: import/export/types/panic must fail check/compile/run ---
$sem = Join-Path $Root "tests\fixtures\semantic_0_2"
$rejectCases = @(
  @{ name = "import_reject"; file = "import_reject.matter"; needle = "import is not implemented" },
  @{ name = "export_reject"; file = "export_reject.matter"; needle = "export is not implemented" },
  @{ name = "type_annotation_reject"; file = "type_annotation_reject.matter"; needle = "type annotations unsupported" },
  @{ name = "panic_reject"; file = "panic_reject.matter"; needle = "not implemented" }
)

foreach ($c in $rejectCases) {
  $p = Join-Path $sem $c.file
  $r = Invoke-Cli @("check", $p)
  $ok = ($r.exit -ne 0) -and (
    ($r.out -match [regex]::Escape($c.needle)) -or
    ($r.out -match "not implemented") -or
    ($r.out -match "unsupported") -or
    ($r.out -match "reserved") -or
    ($r.out -match "Semantic") -or
    ($r.out -match "Compilation error") -or
    ($r.out -match "compile error")
  )
  Add-Result ("check-" + $c.name) $ok ("exit=" + $r.exit + " out=" + ($r.out.Trim().Substring(0, [Math]::Min(120, $r.out.Trim().Length))))
  $r2 = Invoke-Cli @("compile", $p, "-o", (Join-Path $OutDir ($c.name + ".mbc")))
  Add-Result ("compile-" + $c.name) ($r2.exit -ne 0) ("exit=" + $r2.exit)
  $r3 = Invoke-Cli @("run", $p)
  Add-Result ("run-" + $c.name) ($r3.exit -ne 0) ("exit=" + $r3.exit)
  $rj = Invoke-Cli @("check-json", $p)
  $jsonFail = ($rj.exit -ne 0) -and ($rj.out -match '"ok"\s*:\s*false')
  Add-Result ("check-json-" + $c.name) $jsonFail "json"
}

# --- Match equality positive ---
$matchPos = Join-Path $sem "match_equality_positive.matter"
$rm = Invoke-Cli @("run", $matchPos)
# First arm wins on equality: must print "two" then "done"; must not print arm labels "one"/"three" as whole lines.
$lines = @($rm.out -split "(\r?\n)" | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne "" })
$hasTwo = $lines -contains "two"
$hasDone = $lines -contains "done"
$hasOne = $lines -contains "one"
$hasThree = $lines -contains "three"
$okMatch = ($rm.exit -eq 0) -and $hasTwo -and $hasDone -and (-not $hasOne) -and (-not $hasThree)
Add-Result "match-equality-positive" $okMatch ("exit=$($rm.exit) lines=$($lines -join ',')" )

$matchNone = Join-Path $sem "match_no_arm.matter"
$rn = Invoke-Cli @("run", $matchNone)
$okNone = ($rn.exit -eq 0) -and ($rn.out -match "after-no-match")
Add-Result "match-no-arm" $okNone $rn.out.Trim()

$matchStr = Join-Path $sem "match_string.matter"
$rs = Invoke-Cli @("run", $matchStr)
Add-Result "match-string" (($rs.exit -eq 0) -and ($rs.out -match "Z")) $rs.out.Trim()

# --- File Capabilities v1 ---
$sandbox = Join-Path $OutDir "sandbox"
if (Test-Path -LiteralPath $sandbox) { Remove-Item -LiteralPath $sandbox -Recurse -Force }
New-Item -ItemType Directory -Force -Path $sandbox | Out-Null
$escapeOutside = Join-Path $OutDir "outside_escape.txt"
Set-Content -LiteralPath $escapeOutside -Value "SECRET" -Encoding utf8

$wPath = (Join-Path $sandbox "secret.txt")
$wPathFwd = $wPath.Replace("\", "/")

# Default deny write
$tmpDenyW = Join-Path $OutDir "tmp_write_deny.matter"
Write-Matter $tmpDenyW ('file.write("' + $wPathFwd + '", "nope")')
$rd = Invoke-Cli @("run", $tmpDenyW)
Add-Result "fs-write-default-deny" (($rd.exit -ne 0) -and ($rd.out -match "capability_denied")) $rd.out.Trim()

# Default deny read (create file first so failure is capability)
Set-Content -LiteralPath $wPath -Value "x" -Encoding utf8
$tmpDenyR = Join-Path $OutDir "tmp_read_deny.matter"
Write-Matter $tmpDenyR ('print file.read("' + $wPathFwd + '")')
$rr = Invoke-Cli @("run", $tmpDenyR)
Add-Result "fs-read-default-deny" (($rr.exit -ne 0) -and ($rr.out -match "capability_denied")) $rr.out.Trim()

# Default deny delete
$tmpDenyD = Join-Path $OutDir "tmp_delete_deny.matter"
Write-Matter $tmpDenyD ('file.delete("' + $wPathFwd + '")')
$rdel = Invoke-Cli @("run", $tmpDenyD)
Add-Result "fs-delete-default-deny" (($rdel.exit -ne 0) -and ($rdel.out -match "capability_denied")) $rdel.out.Trim()

# Allow write+read only under sandbox
$tmpOk = Join-Path $OutDir "tmp_write_read_allow.matter"
Write-Matter $tmpOk (
  'file.write("' + $wPathFwd + '", "hello")' + [Environment]::NewLine +
  'print file.read("' + $wPathFwd + '")'
)
$rok = Invoke-Cli @("run", $tmpOk, "--allow-fs-write", $sandbox, "--allow-fs-read", $sandbox)
Add-Result "fs-write-read-allow" (($rok.exit -eq 0) -and ($rok.out -match "hello")) $rok.out.Trim()

# Write does not allow delete
$tmpWnD = Join-Path $OutDir "tmp_write_not_delete.matter"
Write-Matter $tmpWnD ('file.delete("' + $wPathFwd + '")')
$rwd = Invoke-Cli @("run", $tmpWnD, "--allow-fs-write", $sandbox)
Add-Result "fs-write-not-delete" (($rwd.exit -ne 0) -and ($rwd.out -match "capability_denied")) $rwd.out.Trim()

# Read does not allow write
$tmpRnW = Join-Path $OutDir "tmp_read_not_write.matter"
Write-Matter $tmpRnW ('file.write("' + $wPathFwd + '", "pwn")')
$rrw = Invoke-Cli @("run", $tmpRnW, "--allow-fs-read", $sandbox)
Add-Result "fs-read-not-write" (($rrw.exit -ne 0) -and ($rrw.out -match "capability_denied")) $rrw.out.Trim()

# Traversal with ..
$travPath = (Join-Path $sandbox "..\outside_escape.txt").Replace("\", "/")
$tmpTrav = Join-Path $OutDir "tmp_traversal.matter"
Write-Matter $tmpTrav ('print file.read("' + $travPath + '")')
$rt = Invoke-Cli @("run", $tmpTrav, "--allow-fs-read", $sandbox)
Add-Result "fs-traversal-dotdot" (($rt.exit -ne 0) -and ($rt.out -match "capability_denied")) $rt.out.Trim()

# Absolute outside root
$outAbs = $escapeOutside.Replace("\", "/")
$tmpAbs = Join-Path $OutDir "tmp_abs_outside.matter"
Write-Matter $tmpAbs ('print file.read("' + $outAbs + '")')
$ra = Invoke-Cli @("run", $tmpAbs, "--allow-fs-read", $sandbox)
Add-Result "fs-absolute-outside" (($ra.exit -ne 0) -and ($ra.out -match "capability_denied")) $ra.out.Trim()

# Unicode + spaces inside root
$uni = Join-Path $sandbox "arquivo com espaco ae.txt"
$uniSlash = $uni.Replace("\", "/")
$tmpUni = Join-Path $OutDir "tmp_unicode.matter"
Write-Matter $tmpUni (
  'file.write("' + $uniSlash + '", "unicode-ok")' + [Environment]::NewLine +
  'print file.read("' + $uniSlash + '")'
)
$ru = Invoke-Cli @("run", $tmpUni, "--allow-fs-write", $sandbox, "--allow-fs-read", $sandbox)
Add-Result "fs-unicode-spaces" (($ru.exit -eq 0) -and ($ru.out -match "unicode-ok")) $ru.out.Trim()

# Bytecode cannot bypass policy
$tmpBcSrc = Join-Path $OutDir "tmp_bc.matter"
$tmpBc = Join-Path $OutDir "tmp_bc.mbc"
Write-Matter $tmpBcSrc ('file.write("' + $wPathFwd + '", "via-mbc")')
$null = Invoke-Cli @("compile", $tmpBcSrc, "-o", $tmpBc)
$rbc = Invoke-Cli @("run-bytecode", $tmpBc)
Add-Result "fs-bytecode-no-bypass" (($rbc.exit -ne 0) -and ($rbc.out -match "capability_denied")) $rbc.out.Trim()

# JSON same result
$rj2 = Invoke-Cli @("run-json", $tmpDenyW)
$jsonCap = ($rj2.exit -ne 0) -and ($rj2.out -match "capability_denied")
Add-Result "fs-json-capability-denied" $jsonCap $rj2.out.Trim()

# Env must not grant access
$env:MATTER_ALLOW_FS_READ = $sandbox
$env:MATTER_ALLOW_FS_ALL = "1"
$renv = Invoke-Cli @("run", $tmpDenyR)
Add-Result "fs-env-no-grant" (($renv.exit -ne 0) -and ($renv.out -match "capability_denied")) "env ignored"
Remove-Item Env:MATTER_ALLOW_FS_READ -ErrorAction SilentlyContinue
Remove-Item Env:MATTER_ALLOW_FS_ALL -ErrorAction SilentlyContinue

# Delete with explicit grant
$tmpDelOk = Join-Path $OutDir "tmp_delete_ok.matter"
Write-Matter $tmpDelOk ('file.delete("' + $wPathFwd + '")')
if (-not (Test-Path -LiteralPath $wPath)) { Set-Content -LiteralPath $wPath -Value "x" -Encoding utf8 }
$rdo = Invoke-Cli @("run", $tmpDelOk, "--allow-fs-delete", $sandbox)
Add-Result "fs-delete-allow" ($rdo.exit -eq 0) $rdo.out.Trim()

# Compile -o host packaging still works
$hello = Join-Path $Root "examples\hello.matter"
$mbcHost = Join-Path $OutDir "hello_host.mbc"
$rc = Invoke-Cli @("compile", $hello, "-o", $mbcHost)
Add-Result "compile-o-host-ok" (($rc.exit -eq 0) -and (Test-Path -LiteralPath $mbcHost)) "host write"

# Core suite examples still run
foreach ($ex in @("examples\hello.matter", "examples\fibonacci.matter", "examples\events.matter")) {
  $ep = Join-Path $Root $ex
  $re = Invoke-Cli @("run", $ep)
  Add-Result ("compat-run-" + (Split-Path $ex -Leaf)) ($re.exit -eq 0) ("exit=" + $re.exit)
}

# ZIP 0.1.0 hash unchanged
$zip = Join-Path $Root "dist\matter-core-0.1.0-windows-x64.zip"
$expected = "0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2"
if (Test-Path -LiteralPath $zip) {
  $h = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash
  Add-Result "zip-0.1.0-hash-unchanged" ($h -eq $expected) $h
} else {
  Add-Result "zip-0.1.0-hash-unchanged" $false "ZIP missing"
}

$pass = @($results | Where-Object { $_.ok }).Count
$fail = @($results | Where-Object { -not $_.ok }).Count
$summary = [ordered]@{
  ok = ($fail -eq 0)
  suite = "matter_0_2_semantic_honesty"
  development_track = "0.2.0"
  production_ready = $false
  release_candidate = $false
  pass = $pass
  fail = $fail
  total = $results.Count
  results = $results
  cli = $Cli
  timestamp = (Get-Date).ToString("o")
}
$summaryPath = Join-Path $OutDir "summary.json"
($summary | ConvertTo-Json -Depth 6) | Set-Content -LiteralPath $summaryPath -Encoding utf8
Write-Host ""
Write-Host ("PASS={0} FAIL={1} TOTAL={2} -> {3}" -f $pass, $fail, $results.Count, $summaryPath)
if ($fail -ne 0) { exit 1 } else { exit 0 }
