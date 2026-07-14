# Phase 5: Unified Production Readiness V2 gate (language-only Matter Core).
# Fails immediately if a mandatory gate fails (except BLOCKED). Emits JSON for CI.
param(
    [string]$CliPath = "",
    [switch]$SkipClippy,
    [switch]$SkipFmt,
    [switch]$SkipCargoTests,
    [switch]$SkipPortable,
    [switch]$SkipFuzzStress,
    [switch]$SkipRepro
)

$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location -LiteralPath $Root
$Out = Join-Path $Root "target\validation\production_readiness_v2"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

if (-not $CliPath) {
    $CliPath = Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
}
if (-not (Test-Path -LiteralPath $CliPath)) {
    Write-Error "CLI missing: $CliPath"
    exit 2
}

$env:PATH = "D:\mingw64\mingw64\bin;D:\dev-tools\cargo\bin;" + $env:PATH
if (-not $env:RUSTUP_HOME) { $env:RUSTUP_HOME = "D:\dev-tools\rustup" }
if (-not $env:CARGO_HOME) { $env:CARGO_HOME = "D:\dev-tools\cargo" }
$env:CC = "D:\mingw64\mingw64\bin\gcc.exe"
$env:CXX = "D:\mingw64\mingw64\bin\g++.exe"
$env:DLLTOOL = "D:\mingw64\mingw64\bin\dlltool.exe"

$gates = New-Object System.Collections.Generic.List[object]
$abort = $false

function Add-Gate([string]$id, [bool]$mandatory, [bool]$ok, [string]$status, $detail) {
    $script:gates.Add([pscustomobject]@{
            id = $id; mandatory = $mandatory; ok = $ok; status = $status; detail = $detail
        })
    $color = if ($ok) { "Green" } elseif ($status -eq "BLOCKED") { "Yellow" } else { "Red" }
    Write-Host "[$status] $id" -ForegroundColor $color
    if ($mandatory -and (-not $ok) -and ($status -eq "FAIL")) {
        $script:abort = $true
    }
}

function Run-Exit([scriptblock]$b) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & $b | Out-Null
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 0 }
    $sw.Stop()
    return @{ exit = [int]$code; ms = $sw.ElapsedMilliseconds }
}

# 1 Core
$r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-core-suite.ps1" -Cli $CliPath }
Add-Gate "core_suite" $true ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) $r

# 2 Security
if (-not $abort) {
    $r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-capability-security.ps1" -Cli $CliPath }
    Add-Gate "security_suite" $true ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) $r
}

# 3 Portable
if (-not $abort) {
    if ($SkipPortable) {
        Add-Gate "portable_suite" $true $false "BLOCKED" @{ reason = "skipped" }
    } else {
        $pkg = Join-Path $Root "dist\matter-core-0.1.0-windows-x64"
        if (Test-Path $pkg) {
            $r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-portable-release.ps1" -CliPath $CliPath -PackageRoot $pkg }
        } else {
            $r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-portable-release.ps1" -CliPath $CliPath }
        }
        Add-Gate "portable_suite" $true ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) $r
    }
}

# 4 Cargo tests
if (-not $abort) {
    if ($SkipCargoTests) {
        Add-Gate "cargo_core_tests" $true $false "BLOCKED" @{ reason = "skipped" }
    } else {
        $crates = @("matter-lexer", "matter-parser", "matter-ast", "matter-bytecode", "matter-vm", "matter-backend", "matter-stdlib", "matter-runtime", "matter-energy")
        $map = @{}
        $ok = $true
        foreach ($c in $crates) {
            $r = Run-Exit { cargo test -p $c --target x86_64-pc-windows-gnu --lib 2>&1 | Out-Null }
            $map[$c] = $r.exit
            if ($r.exit -ne 0) { $ok = $false }
        }
        $r = Run-Exit { cargo test -p matter-cli --target x86_64-pc-windows-gnu --bin matter-cli -- capability_policy 2>&1 | Out-Null }
        $map["matter-cli_capability_policy"] = $r.exit
        if ($r.exit -ne 0) { $ok = $false }
        Add-Gate "cargo_core_tests" $true $ok $(if ($ok) { "PASS" } else { "FAIL" }) $map
    }
}

# 5 fmt (non-mandatory WARN)
if (-not $SkipFmt) {
    $r = Run-Exit { cargo fmt --all -- --check 2>&1 | Tee-Object "$Out\fmt-check.log" | Out-Null }
    Add-Gate "cargo_fmt_check" $false ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "WARN" }) $r
}

# 6 clippy (non-mandatory if only warnings)
if (-not $SkipClippy) {
    $r = Run-Exit {
        cargo clippy -p matter-lexer -p matter-parser -p matter-bytecode -p matter-vm -p matter-runtime --target x86_64-pc-windows-gnu -- -D warnings 2>&1 |
            Tee-Object "$Out\clippy-core.log" | Out-Null
    }
    if ($r.exit -eq 0) {
        Add-Gate "cargo_clippy_core" $false $true "PASS" @{ deny_warnings = $true }
    } else {
        $r2 = Run-Exit {
            cargo clippy -p matter-lexer -p matter-parser -p matter-bytecode -p matter-vm -p matter-runtime --target x86_64-pc-windows-gnu 2>&1 |
                Tee-Object "$Out\clippy-core-allow.log" | Out-Null
        }
        Add-Gate "cargo_clippy_core" $false ($r2.exit -eq 0) $(if ($r2.exit -eq 0) { "WARN" } else { "FAIL" }) @{
            deny_exit = $r.exit; allow_exit = $r2.exit
        }
    }
}

# 7 Fuzz/stress
if (-not $abort) {
    if ($SkipFuzzStress) {
        Add-Gate "fuzz_stress" $true $false "BLOCKED" @{ reason = "skipped" }
    } else {
        $r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-fuzz-stress-v2.ps1" -CliPath $CliPath }
        Add-Gate "fuzz_stress" $true ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) $r
    }
}

# 8 Package integrity
$zip = Join-Path $Root "dist\matter-core-0.1.0-windows-x64.zip"
$pkg = Join-Path $Root "dist\matter-core-0.1.0-windows-x64"
$pkgOk = (Test-Path $zip) -and (Test-Path $pkg) -and (Test-Path "$pkg\MANIFEST.json") -and (Test-Path "$pkg\SHA256SUMS")
$zh = if (Test-Path $zip) { (Get-FileHash -Algorithm SHA256 $zip).Hash } else { $null }
Add-Gate "package_integrity" $true $pkgOk $(if ($pkgOk) { "PASS" } else { "FAIL" }) @{ zip_sha256 = $zh }

# 9 Repro
if (-not $abort) {
    if ($SkipRepro) {
        Add-Gate "package_repro" $true $false "BLOCKED" @{ reason = "skipped" }
    } else {
        $r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\test-package-repro-v2.ps1" -CliPath $CliPath }
        $reproFile = Join-Path $Out "repro-compare.json"
        if (Test-Path $reproFile) {
            $rj = Get-Content $reproFile -Raw | ConvertFrom-Json
            $ok = ($rj.verdict -eq "PASS" -or $rj.verdict -eq "PASS_LOGICAL")
            Add-Gate "package_repro" $true $ok $rj.verdict $rj
        } else {
            Add-Gate "package_repro" $true ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) $r
        }
    }
}

# 10 Independent Windows — this is the dev host
Add-Gate "independent_windows" $true $false "BLOCKED" @{
    reason = "No clean Windows VM available; host has Rust/MinGW/dev tools"
    host   = $env:COMPUTERNAME
}

# 11 Dependency audit / SBOM
$r = Run-Exit { powershell -NoProfile -ExecutionPolicy Bypass -File "$Root\scripts\export-core-sbom-v2.ps1" }
$auditPath = Join-Path $Out "dependency-audit.json"
if (Test-Path $auditPath) {
    $ad = Get-Content $auditPath -Raw | ConvertFrom-Json
    if ($ad.critical_or_high -gt 0) {
        Add-Gate "dependency_audit" $true $false "FAIL" $ad
    } elseif (-not $ad.tool_available) {
        Add-Gate "dependency_audit" $true $false "BLOCKED" $ad
    } else {
        Add-Gate "dependency_audit" $true $true "PASS" $ad
    }
} else {
    Add-Gate "dependency_audit" $true $false "BLOCKED" @{ reason = "no dependency-audit.json"; exit = $r.exit }
}

# 12 Dangerous caps
$err = Join-Path $Out "deny-agent.err"
cmd /c "`"$CliPath`" agent-ui 1>nul 2>`"$err`""
Add-Gate "dangerous_caps_denied" $true ($LASTEXITCODE -ne 0) $(if ($LASTEXITCODE -ne 0) { "PASS" } else { "FAIL" }) @{ exit = $LASTEXITCODE }

# Verdict
$mandatory = @($gates | Where-Object { $_.mandatory })
$anyFail = @($mandatory | Where-Object { (-not $_.ok) -and ($_.status -eq "FAIL") }).Count -gt 0
$anyBlocked = @($mandatory | Where-Object { $_.status -eq "BLOCKED" }).Count -gt 0
$allOk = @($mandatory | Where-Object { -not $_.ok }).Count -eq 0

if ($anyFail) { $verdict = "NOT_READY" }
elseif ($anyBlocked) { $verdict = "BLOCKED_EXTERNAL_VALIDATION" }
elseif ($allOk) { $verdict = "RELEASE_CANDIDATE" }
else { $verdict = "NOT_READY" }

$result = [pscustomobject]@{
    schema_version     = 2
    at                 = (Get-Date).ToString("o")
    product            = "matter-core"
    version            = "0.1.0"
    edition            = "language-only"
    cli                = $CliPath
    cli_sha256         = (Get-FileHash -Algorithm SHA256 -LiteralPath $CliPath).Hash
    verdict            = $verdict
    production_ready   = $false
    gates              = $gates
    notes              = @(
        "PRODUCTION_READY not used without independent clean Windows validation",
        "core-status-json remains production_ready:false"
    )
}
$jsonPath = Join-Path $Out "production-readiness-v2.json"
$result | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $jsonPath -Encoding utf8
Copy-Item $jsonPath (Join-Path $Root "production-readiness-v2.json") -Force
Write-Host ""
Write-Host "VERDICT: $verdict" -ForegroundColor Cyan
Write-Host "JSON: $jsonPath"
if ($verdict -eq "NOT_READY") { exit 1 }
exit 0
