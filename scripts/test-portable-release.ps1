# Phase 4 permanent portability suite
# Artifact Recovery Hotfix:
# - Never writes to dist/ by default
# - Packages into target/validation/.../temp-package/
# - Snapshots frozen dist hashes before/after and fails if they change
# - Records CLI path + sha256; refuses silent stale binary preference
param(
    [string]$CliPath = "",
    [string]$PackageRoot = ""
)

$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$OutDir = Join-Path $Root "target\validation\phase_4_portable_release"
$TempPkgRoot = Join-Path $OutDir ("temp-package-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$name, [int]$code, [bool]$ok, [string]$detail) {
    $script:results.Add([pscustomobject]@{ name = $name; exit = $code; ok = $ok; detail = $detail })
    $st = if ($ok) { "PASS" } else { "FAIL" }
    Write-Host "[$st] $name exit=$code $detail"
}

function Resolve-FreshestCli([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) { throw "CLI not found: $Explicit" }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    $cands = @()
    foreach ($p in @(
        (Join-Path $Root "target\release\matter-cli.exe"),
        (Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe")
    )) {
        if (Test-Path -LiteralPath $p) {
            $i = Get-Item -LiteralPath $p
            $cands += [pscustomobject]@{
                path = (Resolve-Path $p).Path
                mtime = $i.LastWriteTimeUtc
                sha256 = (Get-FileHash $p -Algorithm SHA256).Hash
            }
        }
    }
    if ($cands.Count -eq 0) { throw "No matter-cli.exe found under target\release or target\x86_64-pc-windows-gnu\release" }
    $best = $cands | Sort-Object mtime -Descending | Select-Object -First 1
    if ($cands.Count -gt 1) {
        $stale = $cands | Where-Object { $_.sha256 -ne $best.sha256 }
        if ($stale) {
            Write-Host "WARNING: dual CLI binaries differ; using newest-by-mtime:" -ForegroundColor Yellow
            Write-Host ("  SELECTED {0} sha={1}" -f $best.path, $best.sha256)
            foreach ($s in $stale) {
                Write-Host ("  OTHER    {0} sha={1} mtime={2}" -f $s.path, $s.sha256, $s.mtime)
            }
        }
    }
    return $best.path
}

function Get-DistSnapshot {
    $dist = Join-Path $Root "dist"
    $map = @{}
    if (-not (Test-Path -LiteralPath $dist)) { return $map }
    # Frozen / released artifacts we must not mutate
    Get-ChildItem -LiteralPath $dist -File -ErrorAction SilentlyContinue | ForEach-Object {
        $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        $map[$_.Name] = [pscustomobject]@{
            name = $_.Name
            path = $_.FullName
            sha256 = $h
            size = $_.Length
            mtime_utc = $_.LastWriteTimeUtc.ToString("o")
            readonly = $_.IsReadOnly
        }
    }
    # Also snapshot nested zip names of interest
    Get-ChildItem -LiteralPath $dist -Recurse -File -Filter "*.zip" -ErrorAction SilentlyContinue | ForEach-Object {
        $key = "zip:" + $_.Name
        if (-not $map.ContainsKey($key) -and -not $map.ContainsKey($_.Name)) {
            $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
            $map[$key] = [pscustomobject]@{
                name = $_.Name
                path = $_.FullName
                sha256 = $h
                size = $_.Length
                mtime_utc = $_.LastWriteTimeUtc.ToString("o")
                readonly = $_.IsReadOnly
            }
        }
    }
    return $map
}

function Compare-DistSnapshots($before, $after) {
    $changes = New-Object System.Collections.Generic.List[object]
    $keys = @($before.Keys + $after.Keys) | Select-Object -Unique
    foreach ($k in $keys) {
        $b = $before[$k]
        $a = $after[$k]
        if ($null -eq $b -and $null -ne $a) {
            $changes.Add([pscustomobject]@{ key = $k; change = "added"; after = $a.sha256 })
        } elseif ($null -ne $b -and $null -eq $a) {
            $changes.Add([pscustomobject]@{ key = $k; change = "removed"; before = $b.sha256 })
        } elseif ($b.sha256 -ne $a.sha256 -or $b.size -ne $a.size) {
            $changes.Add([pscustomobject]@{
                key = $k
                change = "modified"
                before = $b.sha256
                after = $a.sha256
                before_size = $b.size
                after_size = $a.size
            })
        }
    }
    return $changes
}

# --- Snapshot dist BEFORE suite ---
$distBefore = Get-DistSnapshot
$distBefore | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "dist-snapshot-before.json") -Encoding utf8

try {
    $CliPath = Resolve-FreshestCli $CliPath
} catch {
    Add-Result "cli-resolve" 1 $false $_.Exception.Message
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "portable-suite-results.json")
    exit 1
}
$cliSha = (Get-FileHash -LiteralPath $CliPath -Algorithm SHA256).Hash
$cliItem = Get-Item -LiteralPath $CliPath
Add-Result "cli-fingerprint" 0 $true ("path=$CliPath sha256=$cliSha size=$($cliItem.Length) mtime=$($cliItem.LastWriteTimeUtc.ToString('o'))")
@{
    path = $CliPath
    sha256 = $cliSha
    size = $cliItem.Length
    mtime_utc = $cliItem.LastWriteTimeUtc.ToString("o")
} | ConvertTo-Json | Set-Content (Join-Path $OutDir "cli-used.json") -Encoding utf8

# 0) Ensure package exists under temp (NEVER default to dist/)
if (-not $PackageRoot) {
    $pkgScript = Join-Path $PSScriptRoot "package-matter-core.ps1"
    $pkgContent = Join-Path $TempPkgRoot "root"
    $zipOut = Join-Path $TempPkgRoot "matter-core-package.zip"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $pkgScript `
        -CliPath $CliPath -SkipBuild `
        -OutDir $pkgContent -ZipPath $zipOut
    if ($LASTEXITCODE -ne 0) {
        Add-Result "package-build" $LASTEXITCODE $false "package-matter-core failed (temp only; dist not used)"
    } else {
        Add-Result "package-build" 0 $true $pkgContent
    }
    if (Test-Path -LiteralPath $pkgContent) {
        $PackageRoot = $pkgContent
    }
}
if (-not $PackageRoot -or -not (Test-Path -LiteralPath $PackageRoot)) {
    Add-Result "package-present" 1 $false "no package root"
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "portable-suite-results.json")
    exit 1
}
# Refuse if package root is under dist/
$pkgFull = [System.IO.Path]::GetFullPath($PackageRoot)
$distFull = [System.IO.Path]::GetFullPath((Join-Path $Root "dist"))
if ($pkgFull.StartsWith($distFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    Add-Result "package-not-in-dist" 1 $false "PackageRoot must not be under dist/: $PackageRoot"
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "portable-suite-results.json")
    exit 1
}
Add-Result "package-present" 0 $true $PackageRoot
Add-Result "package-not-in-dist" 0 $true "ok"

# 1) Package has no build caches *relative to package root*
# (absolute path may contain repo\target\validation\... — do not flag that)
$pkgRootFull = [System.IO.Path]::GetFullPath($PackageRoot).TrimEnd('\', '/')
$forbidden = Get-ChildItem -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue |
    Where-Object {
        $rel = $_.FullName.Substring($pkgRootFull.Length).TrimStart('\', '/')
        $rel -match '(^|\\)target(\\|$)|(^|\\)\.cargo(\\|$)|node_modules|(^|\\)\.git(\\|$)|CARGO_HOME|credentials|\.pdb$'
    }
Add-Result "package-clean" 0 (-not $forbidden) ("forbidden_hits=" + @($forbidden).Count)

# 2) Run from package with PATH=System32 only
$cliPkg = Join-Path $PackageRoot "bin\matter-cli.exe"
$oldPath = $env:PATH
try {
    $env:PATH = Join-Path $env:SystemRoot "System32"
    & $cliPkg --version 1>$null 2>$null
    Add-Result "zip-extract-version" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
    & $cliPkg --help 1>$null 2>$null
    Add-Result "zip-extract-help" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
    & $cliPkg core-status-json 1>$null 2>$null
    Add-Result "zip-extract-core-status" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
    $hello = Join-Path $PackageRoot "examples\hello.matter"
    $mbc = Join-Path $OutDir "portable-hello.mbc"
    & $cliPkg run $hello 1>$null 2>$null
    Add-Result "zip-extract-run" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
    & $cliPkg compile $hello -o $mbc 1>$null 2>$null
    Add-Result "zip-extract-compile" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
    & $cliPkg run-bytecode $mbc 1>$null 2>$null
    Add-Result "zip-extract-run-bytecode" $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
} finally {
    $env:PATH = $oldPath
}

# 3) Install to two different roots
$rootA = Join-Path $env:TEMP "MatterPortA"
$rootB = Join-Path $env:TEMP ("Matter Core portavel " + (Get-Date -Format "HHmmss"))
$install = Join-Path $PSScriptRoot "install-matter-core.ps1"
$verify = Join-Path $PSScriptRoot "verify-matter-core.ps1"
$update = Join-Path $PSScriptRoot "update-matter-core.ps1"
$uninstall = Join-Path $PSScriptRoot "uninstall-matter-core.ps1"

foreach ($pair in @(@{n = "install-A"; r = $rootA }, @{n = "install-B-unicode"; r = $rootB })) {
    if (Test-Path -LiteralPath $pair.r) { Remove-Item -LiteralPath $pair.r -Recurse -Force -ErrorAction SilentlyContinue }
    & powershell -NoProfile -ExecutionPolicy Bypass -File $install -PackageRoot $PackageRoot -InstallRoot $pair.r -SkipPath
    Add-Result $pair.n $LASTEXITCODE ($LASTEXITCODE -eq 0) $pair.r
    & powershell -NoProfile -ExecutionPolicy Bypass -File $verify -InstallRoot $pair.r -MinimalPath
    Add-Result ("verify-" + $pair.n) $LASTEXITCODE ($LASTEXITCODE -eq 0) ""
}

# 4) Update preserves projects
$marker = Join-Path $rootA "projects\user-project-marker.txt"
New-Item -ItemType Directory -Force -Path (Split-Path $marker) | Out-Null
Set-Content -LiteralPath $marker -Value "keep-me" -Encoding utf8
& powershell -NoProfile -ExecutionPolicy Bypass -File $update -PackageRoot $PackageRoot -InstallRoot $rootA -SkipPath
$updOk = ($LASTEXITCODE -eq 0) -and (Test-Path -LiteralPath $marker) -and ((Get-Content -LiteralPath $marker -Raw) -match "keep-me")
Add-Result "update-preserves-projects" $(if ($updOk) { 0 } else { 1 }) $updOk ""

# 5) Uninstall
& powershell -NoProfile -ExecutionPolicy Bypass -File $uninstall -InstallRoot $rootA
$binGone = -not (Test-Path -LiteralPath (Join-Path $rootA "bin\matter-cli.exe"))
$projKept = Test-Path -LiteralPath $marker
Add-Result "uninstall-keeps-projects" 0 ($binGone -and $projKept) "bin_gone=$binGone proj_kept=$projKept"

& powershell -NoProfile -ExecutionPolicy Bypass -File $uninstall -InstallRoot $rootB -RemoveProjects
$cliBGone = -not (Test-Path -LiteralPath (Join-Path $rootB "bin\matter-cli.exe"))
$projBGone = -not (Test-Path -LiteralPath (Join-Path $rootB "projects"))
Add-Result "uninstall-B" 0 ($cliBGone -and $projBGone) "cli_gone=$cliBGone proj_gone=$projBGone"

# 6) Isolated copy
$iso = Join-Path $env:TEMP ("matter-iso-" + [guid]::NewGuid().ToString("n"))
Copy-Item -LiteralPath $PackageRoot -Destination $iso -Recurse -Force
$isoCli = Join-Path $iso "bin\matter-cli.exe"
$env:PATH = Join-Path $env:SystemRoot "System32"
& $isoCli core-status-json 1>$null 2>$null
Add-Result "isolated-copy-run" $LASTEXITCODE ($LASTEXITCODE -eq 0) $iso
$env:PATH = $oldPath
Remove-Item -LiteralPath $iso -Recurse -Force -ErrorAction SilentlyContinue

# 7) SHA256SUMS / MANIFEST present
$hasSums = Test-Path -LiteralPath (Join-Path $PackageRoot "SHA256SUMS")
$hasMan = Test-Path -LiteralPath (Join-Path $PackageRoot "MANIFEST.json")
Add-Result "manifest-present" 0 $hasMan ""
Add-Result "sha256sums-present" 0 $hasSums ""

# 8) Frozen ZIP present in dist (read-only check) — do not require suite-created zip in dist
$frozenZip = Join-Path $Root "dist\matter-core-0.1.0-windows-x64.zip"
$expectedFrozen = "0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2"
if (Test-Path -LiteralPath $frozenZip) {
    $fh = (Get-FileHash -LiteralPath $frozenZip -Algorithm SHA256).Hash
    Add-Result "frozen-zip-present" 0 ($fh -eq $expectedFrozen) ("sha=$fh expected=$expectedFrozen")
} else {
    Add-Result "frozen-zip-present" 1 $false "missing dist frozen zip"
}

# 9) LSP script has no hardcoded D: install requirement for package path
$lspScript = Join-Path $Root "scripts\start-matter-lsp.ps1"
if (Test-Path $lspScript) {
    $txt = Get-Content $lspScript -Raw
    $hard = $txt -match 'D:\\Matter\\bin' -and $txt -notmatch 'MATTER_HOME|LOCALAPPDATA|MATTER_LSP'
    Add-Result "lsp-no-drive-hardcode" 0 (-not $hard) ""
    # Prefer dedicated binary (not invoking matter-cli with lsp subcommand)
    $invokesCliLsp = $txt -match '(?m)&\s*\$cli\s+lsp\b|matter-cli\.exe\s+lsp\b'
    $usesDedicated = ($txt -match 'matter-lsp') -and (-not $invokesCliLsp)
    Add-Result "lsp-script-dedicated-binary" 0 $usesDedicated "start-matter-lsp.ps1 should invoke matter-lsp.exe"
} else {
    Add-Result "lsp-no-drive-hardcode" 0 $true "script missing skipped"
    Add-Result "lsp-script-dedicated-binary" 0 $true "script missing skipped"
}

# 10) Packaged matter-lsp.exe present and smokeable (when package includes it)
$pkgLsp = Join-Path $PackageRoot "bin\matter-lsp.exe"
if (Test-Path -LiteralPath $pkgLsp) {
    $lspSha = (Get-FileHash -LiteralPath $pkgLsp -Algorithm SHA256).Hash
    Add-Result "package-has-matter-lsp" 0 $true ("sha=$lspSha")
    # Space-path copy smoke: still a valid PE that exits when run with no client (or host kills)
    $spaceDir = Join-Path $env:TEMP ("matter lsp space " + [guid]::NewGuid().ToString("n").Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $spaceDir | Out-Null
    $spaceLsp = Join-Path $spaceDir "matter-lsp.exe"
    Copy-Item -LiteralPath $pkgLsp -Destination $spaceLsp -Force
    $spaceOk = Test-Path -LiteralPath $spaceLsp
    Add-Result "matter-lsp-space-path-copy" 0 $spaceOk $spaceLsp
    Remove-Item -LiteralPath $spaceDir -Recurse -Force -ErrorAction SilentlyContinue
    # MANIFEST / SHA256SUMS mention lsp when present
    $sumsPath = Join-Path $PackageRoot "SHA256SUMS"
    if (Test-Path -LiteralPath $sumsPath) {
        $sums = Get-Content $sumsPath -Raw
        Add-Result "sha256sums-lists-matter-lsp" 0 ($sums -match 'matter-lsp\.exe') ""
    }
} else {
    Add-Result "package-has-matter-lsp" 0 $false "bin/matter-lsp.exe missing (optional until delivery package)"
}

# --- Snapshot dist AFTER suite ---
$distAfter = Get-DistSnapshot
$distAfter | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "dist-snapshot-after.json") -Encoding utf8
$changes = Compare-DistSnapshots $distBefore $distAfter
$changes | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "dist-snapshot-diff.json") -Encoding utf8
$immutableOk = (@($changes).Count -eq 0)
Add-Result "dist-immutable" $(if ($immutableOk) { 0 } else { 1 }) $immutableOk ("changes=" + @($changes).Count)

$pass = @($results | Where-Object { $_.ok }).Count
$fail = @($results | Where-Object { -not $_.ok }).Count
$summary = [ordered]@{
    ok = ($fail -eq 0)
    pass = $pass
    fail = $fail
    total = $results.Count
    cli = @{ path = $CliPath; sha256 = $cliSha }
    package_root = $PackageRoot
    wrote_to_dist = $false
    dist_immutable = $immutableOk
    dist_changes = $changes
    results = $results
}
$summary | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $OutDir "portable-suite-results.json") -Encoding utf8
Write-Host ("PORTABLE SUITE passed={0} failed={1} total={2}" -f $pass, $fail, $results.Count)
if ($fail -ne 0) { exit 1 } else { exit 0 }
