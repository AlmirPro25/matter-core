# Permanent gate: build language-only (and optionally experimental-full) from an
# isolated CARGO_TARGET_DIR so incremental caches cannot hide missing AST/native arms.
#
# Usage:
#   .\scripts\test-clean-checkout-build.ps1
#   .\scripts\test-clean-checkout-build.ps1 -IncludeExperimental
#   .\scripts\test-clean-checkout-build.ps1 -Commit 6c3a77e -WorkRoot D:\matter-gate
#
# Does NOT run cargo clean on the primary repo tree. Uses a worktree + separate target dir.
param(
    [string]$Commit = "HEAD",
    [string]$WorkRoot = "",
    [switch]$IncludeExperimental,
    [switch]$KeepWorktree
)

$ErrorActionPreference = "Continue"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $WorkRoot) {
    $WorkRoot = Join-Path "D:\" ("matter-clean-build-" + (Get-Date -Format "yyyyMMdd_HHmmss"))
}
$TargetDir = Join-Path $WorkRoot "cargo-target"
$Worktree = Join-Path $WorkRoot "src"
$OutDir = Join-Path $Repo "target\validation\clean_checkout_build"
New-Item -ItemType Directory -Force -Path $OutDir, $TargetDir | Out-Null

$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$name, [bool]$ok, [string]$detail) {
    $script:results.Add([pscustomobject]@{ name = $name; ok = $ok; detail = $detail })
    $st = if ($ok) { "PASS" } else { "FAIL" }
    Write-Host "[$st] $name - $detail"
}

$resolved = git -C $Repo rev-parse $Commit 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cannot resolve commit: $Commit"
    exit 2
}
$resolved = $resolved.Trim()

# Fresh worktree (no pre-existing target under it)
if (Test-Path -LiteralPath $Worktree) {
    git -C $Repo worktree remove --force $Worktree 2>$null
    if (Test-Path -LiteralPath $Worktree) {
        Remove-Item -LiteralPath $Worktree -Recurse -Force -ErrorAction SilentlyContinue
    }
}
git -C $Repo worktree add --detach $Worktree $resolved 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $Worktree)) {
    Add-Result "worktree" $false "failed to create worktree at $Worktree for $resolved"
    $results | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $OutDir "results.json")
    exit 1
}
Add-Result "worktree" $true ("path=$Worktree commit=$resolved")

# Confirm isolated target has no matter-cli.exe before build
$pre = @(Get-ChildItem -LiteralPath $TargetDir -Recurse -Filter "matter-cli*.exe" -ErrorAction SilentlyContinue)
Add-Result "no-prebuilt-cli" ($pre.Count -eq 0) ("count=$($pre.Count)")

$env:PATH = "D:\mingw64\mingw64\bin;" + $env:PATH
if (Test-Path "D:\mingw64\mingw64\bin\dlltool.exe") {
    $env:DLLTOOL = "D:\mingw64\mingw64\bin\dlltool.exe"
}
$env:CARGO_TARGET_DIR = $TargetDir

# windows-gnu experimental-full needs libshlwapi.a (egui/arboard); rust self-contained
# may lack it. Prefer repo-vendored tools/windows-gnu-libs, then install into worktree
# and into the active toolchain self-contained dir when missing.
function Ensure-ShlwapiImportLib {
    $vendored = Join-Path $Repo "tools\windows-gnu-libs\libshlwapi.a"
    $candidates = @(
        $vendored,
        "D:\dev-tools\llvm-mingw\current\x86_64-w64-mingw32\lib\libshlwapi.a",
        "C:\msys64\mingw64\lib\libshlwapi.a"
    )
    $src = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $src) {
        Add-Result "shlwapi-import-lib" $false "libshlwapi.a not found (vendor tools/windows-gnu-libs or llvm-mingw)"
        return $false
    }
    $wtLib = Join-Path $Worktree "tools\windows-gnu-libs"
    New-Item -ItemType Directory -Force -Path $wtLib | Out-Null
    Copy-Item -LiteralPath $src -Destination (Join-Path $wtLib "libshlwapi.a") -Force
    $installedSc = $false
    $sysroot = (& rustc --print sysroot 2>$null)
    if ($sysroot) {
        $sc = Join-Path $sysroot.Trim() "lib\rustlib\x86_64-pc-windows-gnu\lib\self-contained\libshlwapi.a"
        if (-not (Test-Path -LiteralPath $sc)) {
            try {
                Copy-Item -LiteralPath $src -Destination $sc -Force -ErrorAction Stop
                $installedSc = $true
            } catch {
                $installedSc = $false
            }
        } else {
            $installedSc = $true
        }
    }
    # Prefer self-contained install (no RUSTFLAGS fingerprint churn). Fallback -L only if needed.
    if (-not $installedSc) {
        $env:RUSTFLAGS = ("-L native={0}" -f $wtLib)
    }
    Add-Result "shlwapi-import-lib" $true ("src=$src self_contained=$installedSc")
    return $true
}
if ($IncludeExperimental) {
    [void](Ensure-ShlwapiImportLib)
}

# language-only
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$logLang = Join-Path $OutDir "language-only.log"
Push-Location $Worktree
try {
    cargo build -p matter-cli --release --bin matter-cli 2>&1 | Tee-Object $logLang | Out-Null
    $ecLang = $LASTEXITCODE
} finally {
    Pop-Location
}
$sw.Stop()
$cliLang = Join-Path $TargetDir "release\matter-cli.exe"
$langOk = ($ecLang -eq 0) -and (Test-Path -LiteralPath $cliLang)
$langSha = if ($langOk) { (Get-FileHash -LiteralPath $cliLang -Algorithm SHA256).Hash } else { $null }
Add-Result "build-language-only" $langOk ("exit=$ecLang sec=$([math]::Round($sw.Elapsed.TotalSeconds,1)) sha=$langSha")

# experimental-full (optional)
$expOk = $true
$expSha = $null
if ($IncludeExperimental) {
    $sw2 = [System.Diagnostics.Stopwatch]::StartNew()
    $logExp = Join-Path $OutDir "experimental-full.log"
    Push-Location $Worktree
    try {
        cargo build -p matter-cli --release --features experimental-full --bin matter-cli-experimental 2>&1 |
            Tee-Object $logExp | Out-Null
        $ecExp = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    $sw2.Stop()
    $cliExp = Join-Path $TargetDir "release\matter-cli-experimental.exe"
    $expOk = ($ecExp -eq 0) -and (Test-Path -LiteralPath $cliExp)
    $expSha = if ($expOk) { (Get-FileHash -LiteralPath $cliExp -Algorithm SHA256).Hash } else { $null }
    Add-Result "build-experimental-full" $expOk ("exit=$ecExp sec=$([math]::Round($sw2.Elapsed.TotalSeconds,1)) sha=$expSha")
} else {
    Add-Result "build-experimental-full" $true "skipped (pass -IncludeExperimental)"
}

# Surface integrity unit tests (991dcf4 variants)
Push-Location $Worktree
try {
    cargo test -p matter-ast --lib surface_integrity_991dcf4 --release 2>&1 |
        Tee-Object (Join-Path $OutDir "ast-surface-tests.log") | Out-Null
    $ecAst = $LASTEXITCODE
} finally {
    Pop-Location
}
Add-Result "ast-surface-integrity-tests" ($ecAst -eq 0) ("exit=$ecAst")

# Optional: bytecode MakeClosure constructible in this target dir
Push-Location $Worktree
try {
    cargo test -p matter-bytecode --lib make_closure_surface --release 2>&1 |
        Tee-Object (Join-Path $OutDir "make-closure-tests.log") | Out-Null
    $ecMc = $LASTEXITCODE
} finally {
    Pop-Location
}
# If filter finds no tests yet, treat as soft skip only when exit=0 with 0 tests
Add-Result "make-closure-surface-tests" ($ecMc -eq 0) ("exit=$ecMc")

$pass = @($results | Where-Object ok).Count
$fail = @($results | Where-Object { -not $_.ok }).Count
$summary = [ordered]@{
    ok = ($fail -eq 0)
    commit = $resolved
    worktree = $Worktree
    cargo_target_dir = $TargetDir
    language_only_sha256 = $langSha
    experimental_sha256 = $expSha
    pass = $pass
    fail = $fail
    total = $results.Count
    results = $results
    timestamp = (Get-Date).ToString("o")
}
$summary | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $OutDir "summary.json") -Encoding utf8
Write-Host ("CLEAN BUILD GATE pass={0} fail={1}" -f $pass, $fail)

if (-not $KeepWorktree) {
    git -C $Repo worktree remove --force $Worktree 2>$null
}

if ($fail -ne 0) { exit 1 } else { exit 0 }
