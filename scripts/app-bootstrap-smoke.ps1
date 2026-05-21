param(
    [string]$ProjectDir = "target\quick-app",
    [string]$Template = "basic",
    [string]$MatterCliPath = "",
    [switch]$WithEnergy
)

$ErrorActionPreference = "Stop"

function Invoke-MatterJson {
    param(
        [string[]]$Args
    )

    if ([string]::IsNullOrWhiteSpace($script:MatterCliResolvedPath)) {
        $output = & cargo run -q -p matter-cli -- @Args
    } else {
        $output = & $script:MatterCliResolvedPath @Args
    }
    if ($LASTEXITCODE -ne 0) {
        throw "matter-cli command failed: $($Args -join ' ')"
    }
    return $output | ConvertFrom-Json
}

Write-Host "== Matter App Bootstrap + Smoke ==" -ForegroundColor Cyan
Write-Host "ProjectDir: $ProjectDir"
Write-Host "Template:   $Template"

if (-not [string]::IsNullOrWhiteSpace($MatterCliPath)) {
    $script:MatterCliResolvedPath = (Resolve-Path $MatterCliPath).Path
} elseif (Test-Path "target\release\matter-cli.exe" -PathType Leaf) {
    $script:MatterCliResolvedPath = (Resolve-Path "target\release\matter-cli.exe").Path
} else {
    $script:MatterCliResolvedPath = ""
}

if ([string]::IsNullOrWhiteSpace($script:MatterCliResolvedPath)) {
    Write-Host "CLI mode:   cargo run -q -p matter-cli -- <cmd>" -ForegroundColor Yellow
} else {
    Write-Host "CLI mode:   $script:MatterCliResolvedPath <cmd>" -ForegroundColor Green
}

$null = New-Item -ItemType Directory -Force $ProjectDir

$initArgs = @("init-json", $ProjectDir, "--template", $Template)
$init = Invoke-MatterJson -Args $initArgs
if (-not $init.ok) {
    throw "init-json returned ok=false"
}

$manifestPath = Join-Path $ProjectDir "matter.toml"
if (-not (Test-Path $manifestPath -PathType Leaf)) {
    throw "matter.toml was not created at $manifestPath"
}
$manifestPath = (Resolve-Path $manifestPath).Path

$check = Invoke-MatterJson -Args @("project-check-json", $manifestPath)
if (-not $check.ok) {
    throw "project-check-json returned ok=false"
}

$runArgs = @("project-run-json", $manifestPath)
if ($WithEnergy) {
    $runArgs += "--with-energy"
}
$run = Invoke-MatterJson -Args $runArgs
if (-not $run.ok) {
    throw "project-run-json returned ok=false"
}

$result = [PSCustomObject]@{
    ok = $true
    projectDir = $ProjectDir
    template = $Template
    manifest = $manifestPath
    init = [PSCustomObject]@{
        ok = $init.ok
        package = $init.package
    }
    check = [PSCustomObject]@{
        ok = $check.ok
    }
    run = [PSCustomObject]@{
        ok = $run.ok
        output = $run.output
    }
}

$result | ConvertTo-Json -Depth 8
