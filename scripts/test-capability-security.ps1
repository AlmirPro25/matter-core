# Phase 3 capability security suite (language-only binary)
param([string]$Cli = "")
$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $Cli) {
  $Cli = Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
}
if (-not (Test-Path -LiteralPath $Cli)) { Write-Error "CLI missing: $Cli"; exit 2 }

$OutDir = Join-Path $Root "target\validation\phase_3_capability_security"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$results = New-Object System.Collections.Generic.List[object]

function Add-Result([string]$name, [int]$exitCode, [bool]$ok, [string]$detail) {
  $script:results.Add([pscustomobject]@{ name = $name; exit = $exitCode; ok = $ok; detail = $detail })
  $st = if ($ok) { "PASS" } else { "FAIL" }
  Write-Host "[$st] $name exit=$exitCode $detail"
}

function Invoke-Cli {
  param([string[]]$CliArgs, [string]$OutFile, [string]$ErrFile)
  # Always quote argv for cmd.exe so | & ; are not interpreted as shell operators.
  $argLine = ($CliArgs | ForEach-Object { '"' + ($_ -replace '"', '""') + '"' }) -join ' '
  cmd /c "`"$Cli`" $argLine 1>`"$OutFile`" 2>`"$ErrFile`""
  return $LASTEXITCODE
}

# --- denied commands: exit != 0 and must not claim executed ---
$denied = @(
  "agent-ui", "polyglot-status-json", "shell", "exec", "package-install",
  "curl", "powershell", "cmd", "net-serve", "bridge-python", "run-shell",
  "tool-pipeline-demo-json"
)
foreach ($d in $denied) {
  $errFile = Join-Path $OutDir "deny-$d.stderr.txt"
  $outFile = Join-Path $OutDir "deny-$d.stdout.txt"
  $code = Invoke-Cli -CliArgs @($d) -OutFile $outFile -ErrFile $errFile
  $err = if (Test-Path -LiteralPath $errFile) { Get-Content -LiteralPath $errFile -Raw -ErrorAction SilentlyContinue } else { "" }
  if ($null -eq $err) { $err = "" }
  $out = if (Test-Path -LiteralPath $outFile) { Get-Content -LiteralPath $outFile -Raw -ErrorAction SilentlyContinue } else { "" }
  if ($null -eq $out) { $out = "" }
  $notExecuted = ($err -match "NOT executed|not available|Unknown command")
  $noFalseOk = ($out -notmatch '"ok"\s*:\s*true')
  $pass = ($code -ne 0) -and $notExecuted -and $noFalseOk
  Add-Result "deny-$d" $code $pass "not_executed=$notExecuted no_ok_true=$noFalseOk"
}

# --- Matter source cannot reach shell/net/agent/python backends ---
$tmp = Join-Path $OutDir "probe"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$utf8 = New-Object System.Text.UTF8Encoding $false
function Write-NoBom([string]$path, [string]$text) {
  [System.IO.File]::WriteAllText($path, $text, $utf8)
}

Write-NoBom (Join-Path $tmp "try_agent.matter") "agent.say(`"hi`")`n"
Write-NoBom (Join-Path $tmp "try_net.matter") "print net.get(`"http://127.0.0.1:9/`")`n"
Write-NoBom (Join-Path $tmp "try_python.matter") "python.call(`"print`", 1)`n"
Write-NoBom (Join-Path $tmp "valid.matter") "print `"core-ok`"`n"

foreach ($f in @("try_agent.matter", "try_net.matter", "try_python.matter")) {
  $errFile = Join-Path $OutDir "run-$f.stderr.txt"
  $outFile = Join-Path $OutDir "run-$f.stdout.txt"
  $code = Invoke-Cli -CliArgs @("run", (Join-Path $tmp $f)) -OutFile $outFile -ErrFile $errFile
  $err = ""
  if (Test-Path -LiteralPath $errFile) { $err = Get-Content -LiteralPath $errFile -Raw }
  $snippet = if ($err.Length -gt 100) { $err.Substring(0, 100) } else { $err }
  Add-Result "matter-block-$f" $code ($code -ne 0) $snippet.Trim()
}

$code = Invoke-Cli -CliArgs @("run", (Join-Path $tmp "valid.matter")) -OutFile (Join-Path $OutDir "valid.out") -ErrFile (Join-Path $OutDir "valid.err")
Add-Result "matter-valid-intact" $code ($code -eq 0) ""

# --- injection-like CLI command names must not execute ---
foreach ($i in @("run;calc", "eval|whoami", "check&dir")) {
  $code = Invoke-Cli -CliArgs @($i) -OutFile (Join-Path $OutDir "inj.out") -ErrFile (Join-Path $OutDir "inj.err")
  Add-Result "cli-injection-$i" $code ($code -ne 0) ""
}

# --- capabilities-json security claims ---
$capOut = Join-Path $OutDir "capabilities.out"
$capErr = Join-Path $OutDir "capabilities.err"
$null = Invoke-Cli -CliArgs @("capabilities-json") -OutFile $capOut -ErrFile $capErr
$cap = Get-Content -LiteralPath $capOut -Raw
$capOk = ($cap -match '"edition"\s*:\s*"language-only"') -and ($cap -match '"shell_spawn"\s*:\s*false') -and ($cap -match '"is_sandbox"\s*:\s*false')
Add-Result "capabilities-security-json" 0 $capOk "has security block"

# --- minimal PATH ---
$old = $env:PATH
try {
  $env:PATH = (Join-Path $env:SystemRoot "System32")
  $code = Invoke-Cli -CliArgs @("--version") -OutFile (Join-Path $OutDir "ver.out") -ErrFile (Join-Path $OutDir "ver.err")
  Add-Result "minimal-path" $code ($code -eq 0) ""
  $code = Invoke-Cli -CliArgs @("eval", "print 1") -OutFile (Join-Path $OutDir "eval.out") -ErrFile (Join-Path $OutDir "eval.err")
  Add-Result "minimal-path-eval" $code ($code -eq 0) ""
} finally {
  $env:PATH = $old
}

# --- no secrets in deny diagnostics ---
$errFile = Join-Path $OutDir "secrets.err"
$outFile = Join-Path $OutDir "secrets.out"
$null = Invoke-Cli -CliArgs @("agent-ui") -OutFile $outFile -ErrFile $errFile
$probe = (Get-Content -LiteralPath $errFile -Raw) + (Get-Content -LiteralPath $outFile -Raw)
$noSecret = ($probe -notmatch "API_KEY|SECRET|PASSWORD|BEGIN RSA|private_key")
Add-Result "no-secrets-on-deny" 0 $noSecret ""

# --- size vs phase2 post baseline (~3.55MB) ---
$item = Get-Item -LiteralPath $Cli
$baseline = 3725413
$delta = [Math]::Abs($item.Length - $baseline)
Add-Result "size-near-baseline" 0 ($delta -lt 500000) "bytes=$($item.Length) delta=$delta"

# --- no python3.dll ---
$objdump = $null
foreach ($c in @($env:MATTER_MINGW_BIN, "D:\mingw64\mingw64\bin", "C:\mingw64\mingw64\bin")) {
    if ($c) {
        $p = Join-Path $c "objdump.exe"
        if (Test-Path $p) { $objdump = $p; break }
    }
}
if (-not $objdump) {
    $cmd = Get-Command objdump -ErrorAction SilentlyContinue
    if ($cmd) { $objdump = $cmd.Source }
}
if ($objdump -and (Test-Path $objdump)) {
  $dll = & $objdump -p $Cli 2>$null | Out-String
  $noPy = ($dll -notmatch "python3")
  $noGl = ($dll -notmatch "opengl32|mfplat")
  Add-Result "dll-no-python" 0 $noPy ""
  Add-Result "dll-no-opengl-mf" 0 $noGl ""
  $dll | Select-String "DLL Name" | Set-Content (Join-Path $OutDir "dll-after.txt")
}

$failed = @($results | Where-Object { -not $_.ok }).Count
$passed = @($results | Where-Object { $_.ok }).Count
$summary = [pscustomobject]@{
  at      = (Get-Date).ToString("o")
  cli     = $Cli
  passed  = $passed
  failed  = $failed
  total   = $results.Count
  results = $results
}
$summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutDir "security-suite-results.json") -Encoding utf8
Write-Host "SECURITY SUITE passed=$passed failed=$failed total=$($results.Count)"
if ($failed -gt 0) { exit 1 } else { exit 0 }
