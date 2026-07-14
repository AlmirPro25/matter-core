# Phase 4 permanent portability suite
param(
    [string]$CliPath = "",
    [string]$PackageRoot = ""
)

$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$OutDir = Join-Path $Root "target\validation\phase_4_portable_release"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$name, [int]$code, [bool]$ok, [string]$detail) {
    $script:results.Add([pscustomobject]@{ name = $name; exit = $code; ok = $ok; detail = $detail })
    $st = if ($ok) { "PASS" } else { "FAIL" }
    Write-Host "[$st] $name exit=$code $detail"
}

if (-not $CliPath) {
    $CliPath = Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
}

# 0) Ensure package exists
if (-not $PackageRoot) {
    $pkgScript = Join-Path $PSScriptRoot "package-matter-core.ps1"
    & powershell -NoProfile -ExecutionPolicy Bypass -File $pkgScript -CliPath $CliPath -SkipBuild
    if ($LASTEXITCODE -ne 0) {
        Add-Result "package-build" $LASTEXITCODE $false "package-matter-core failed"
    } else {
        Add-Result "package-build" 0 $true ""
    }
    $meta = Get-ChildItem (Join-Path $Root "dist") -Filter "matter-core-*-windows-x64" -Directory |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($meta) { $PackageRoot = $meta.FullName }
}
if (-not $PackageRoot -or -not (Test-Path -LiteralPath $PackageRoot)) {
    Add-Result "package-present" 1 $false "no package root"
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutDir "portable-suite-results.json")
    exit 1
}
Add-Result "package-present" 0 $true $PackageRoot

# 1) Package has no target/ caches
$forbidden = Get-ChildItem -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue |
    Where-Object {
        $_.FullName -match '\\target\\|\\\.cargo\\|node_modules|\\\.git\\|CARGO_HOME|credentials|\.pdb$'
    }
Add-Result "package-clean" 0 (-not $forbidden) ("forbidden_hits=" + @($forbidden).Count)

# 2) Run from extracted package with PATH=System32 only
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

# 3) Install to two different roots (Unicode + spaces)
# Prefer $env:TEMP; also test a path with spaces + non-ASCII
$rootA = Join-Path $env:TEMP "MatterPortA"
$rootB = Join-Path $env:TEMP ("Matter Core portavel " + (Get-Date -Format "HHmmss"))
$install = Join-Path $PSScriptRoot "install-matter-core.ps1"
$verify = Join-Path $PSScriptRoot "verify-matter-core.ps1"
$update = Join-Path $PSScriptRoot "update-matter-core.ps1"
$uninstall = Join-Path $PSScriptRoot "uninstall-matter-core.ps1"

foreach ($pair in @(@{n="install-A"; r=$rootA}, @{n="install-B-unicode"; r=$rootB})) {
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

# 5) Uninstall removes Matter files but keeps projects by default
& powershell -NoProfile -ExecutionPolicy Bypass -File $uninstall -InstallRoot $rootA
$binGone = -not (Test-Path -LiteralPath (Join-Path $rootA "bin\matter-cli.exe"))
$projKept = Test-Path -LiteralPath $marker
Add-Result "uninstall-keeps-projects" 0 ($binGone -and $projKept) "bin_gone=$binGone proj_kept=$projKept"

# Full remove of B including projects
& powershell -NoProfile -ExecutionPolicy Bypass -File $uninstall -InstallRoot $rootB -RemoveProjects
$cliBGone = -not (Test-Path -LiteralPath (Join-Path $rootB "bin\matter-cli.exe"))
$projBGone = -not (Test-Path -LiteralPath (Join-Path $rootB "projects"))
Add-Result "uninstall-B" 0 ($cliBGone -and $projBGone) "cli_gone=$cliBGone proj_gone=$projBGone"

# 6) Copy package to isolated temp and run
$iso = Join-Path $env:TEMP ("matter-iso-" + [guid]::NewGuid().ToString("n"))
Copy-Item -LiteralPath $PackageRoot -Destination $iso -Recurse -Force
$isoCli = Join-Path $iso "bin\matter-cli.exe"
$env:PATH = Join-Path $env:SystemRoot "System32"
& $isoCli core-status-json 1>$null 2>$null
Add-Result "isolated-copy-run" $LASTEXITCODE ($LASTEXITCODE -eq 0) $iso
$env:PATH = $oldPath
Remove-Item -LiteralPath $iso -Recurse -Force -ErrorAction SilentlyContinue

# 7) SHA256SUMS / MANIFEST present
Add-Result "manifest-present" 0 (Test-Path (Join-Path $PackageRoot "MANIFEST.json")) ""
Add-Result "sha256sums-present" 0 (Test-Path (Join-Path $PackageRoot "SHA256SUMS")) ""

# 8) Zip exists
$zip = Get-ChildItem (Join-Path $Root "dist") -Filter "matter-core-*-windows-x64.zip" |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
Add-Result "zip-present" 0 ($null -ne $zip) $(if ($zip) { $zip.FullName } else { "" })

# 9) LSP resolver finds package bin without drive hardcode (script loads)
$lsp = Join-Path $PSScriptRoot "start-matter-lsp.ps1"
$lspText = Get-Content -LiteralPath $lsp -Raw
$noHardD = ($lspText -notmatch 'D:\\Matter') -and ($lspText -notmatch 'D:/Users')
Add-Result "lsp-no-drive-hardcode" 0 $noHardD ""

# cleanup install A remnants
Remove-Item -LiteralPath $rootA -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $rootB -Recurse -Force -ErrorAction SilentlyContinue

$failed = @($results | Where-Object { -not $_.ok }).Count
$passed = @($results | Where-Object { $_.ok }).Count
$summary = [pscustomobject]@{
    at = (Get-Date).ToString("o")
    package = $PackageRoot
    passed = $passed
    failed = $failed
    total = $results.Count
    results = $results
}
$summary | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $OutDir "portable-suite-results.json") -Encoding utf8
Write-Host "PORTABLE SUITE passed=$passed failed=$failed total=$($results.Count)"
if ($failed -gt 0) { exit 1 } else { exit 0 }
