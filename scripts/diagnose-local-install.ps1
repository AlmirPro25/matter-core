param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Matter",
    [switch]$AllowMissingPath,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$installRoot = [System.IO.Path]::GetFullPath($InstallDir)
$binDir = Join-Path $installRoot "bin"
$matterExe = Join-Path $binDir "matter.exe"
$matterCliExe = Join-Path $binDir "matter-cli.exe"
$firstRun = Join-Path $installRoot "examples\first_run.matter"
$coreSchema = Join-Path $installRoot "schemas\core-status.schema.json"
$coreStatusPath = Join-Path $installRoot "target\core\core-status.json"
$worldSchema = Join-Path $installRoot "schemas\world-status.schema.json"
$worldStatusPath = Join-Path $installRoot "target\world\world-status.json"
$frontierSchema = Join-Path $installRoot "schemas\frontier-status.schema.json"
$frontierStatusPath = Join-Path $installRoot "target\frontier\frontier-status.json"
$manifestPath = Join-Path $installRoot "INSTALL_MANIFEST.json"
$infoPath = Join-Path $installRoot "INFO.txt"
$uninstaller = Join-Path $installRoot "scripts\uninstall-local.ps1"

$checks = New-Object System.Collections.Generic.List[object]

function Add-Check {
    param(
        [string]$Name,
        [bool]$Ok,
        [string]$Detail
    )

    $checks.Add([ordered]@{
        name = $Name
        ok = $Ok
        detail = $Detail
    }) | Out-Null
}

function Test-File {
    param(
        [string]$Name,
        [string]$Path
    )

    $exists = Test-Path $Path -PathType Leaf
    Add-Check $Name $exists $Path
    return $exists
}

$installDirExists = Test-Path $installRoot -PathType Container
Add-Check "install directory exists" $installDirExists $installRoot

$matterExists = Test-File "matter.exe exists" $matterExe
$matterCliExists = Test-File "matter-cli.exe exists" $matterCliExe
$manifestExists = Test-File "install manifest exists" $manifestPath
$firstRunExists = Test-File "first_run example exists" $firstRun
$coreSchemaExists = Test-File "core status schema exists" $coreSchema
$coreStatusExists = Test-File "core status artifact exists" $coreStatusPath
$worldSchemaExists = Test-File "world status schema exists" $worldSchema
$worldStatusExists = Test-File "world status artifact exists" $worldStatusPath
$frontierSchemaExists = Test-File "frontier status schema exists" $frontierSchema
$frontierStatusExists = Test-File "frontier status artifact exists" $frontierStatusPath
Test-File "INFO.txt exists" $infoPath | Out-Null
Test-File "uninstaller exists" $uninstaller | Out-Null

$manifest = $null
if ($manifestExists) {
    try {
        $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
        Add-Check "install manifest parses" $true "INSTALL_MANIFEST.json"
        Add-Check "install manifest schema" ($manifest.schema -eq "matter.release.install.v1") ([string]$manifest.schema)
    }
    catch {
        Add-Check "install manifest parses" $false $_.Exception.Message
    }
}

if ($manifest -and $matterExists) {
    $matterHash = (Get-FileHash -LiteralPath $matterExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $matterEntry = @($manifest.installed_binaries) | Where-Object { $_.path -eq "bin\matter.exe" } | Select-Object -First 1
    Add-Check "matter.exe hash matches manifest" ($matterEntry -and $matterEntry.sha256 -eq $matterHash) $matterHash
}

if ($manifest -and $matterCliExists) {
    $matterCliHash = (Get-FileHash -LiteralPath $matterCliExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $matterCliEntry = @($manifest.installed_binaries) | Where-Object { $_.path -eq "bin\matter-cli.exe" } | Select-Object -First 1
    Add-Check "matter-cli.exe hash matches manifest" ($matterCliEntry -and $matterCliEntry.sha256 -eq $matterCliHash) $matterCliHash
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @()
if ($userPath) {
    $pathEntries = $userPath -split ';' | Where-Object { $_ }
}
Add-Check "user PATH contains bin" ($pathEntries -contains $binDir) $binDir
if ($AllowMissingPath -and $pathEntries -notcontains $binDir) {
    $checks.RemoveAt($checks.Count - 1)
    Add-Check "user PATH contains bin" $true "allowed missing PATH for no-path install: $binDir"
}

if ($matterExists) {
    try {
        $capabilitiesOutput = & $matterExe capabilities-json
        $capabilitiesExit = $LASTEXITCODE
        $capabilitiesJson = $capabilitiesOutput | ConvertFrom-Json
        Add-Check "matter capabilities-json works" ($capabilitiesExit -eq 0 -and $capabilitiesJson.ok) "exit=$capabilitiesExit"
    }
    catch {
        Add-Check "matter capabilities-json works" $false $_.Exception.Message
    }
}

if ($matterExists) {
    try {
        $coreOutput = & $matterExe core-status-json
        $coreExit = $LASTEXITCODE
        $coreJson = $coreOutput | ConvertFrom-Json
        Add-Check "matter core-status-json works" ($coreExit -eq 0 -and $coreJson.ok -and $coreJson.summary.claim -eq "experimental_language_runtime") "exit=$coreExit"
    }
    catch {
        Add-Check "matter core-status-json works" $false $_.Exception.Message
    }
}

if ($matterExists) {
    try {
        $worldOutput = & $matterExe world-status-json
        $worldExit = $LASTEXITCODE
        $worldJson = $worldOutput | ConvertFrom-Json
        Add-Check "matter world-status-json works" ($worldExit -eq 0 -and $worldJson.ok -and $worldJson.summary.mode -eq "logical_world_partition") "exit=$worldExit"
    }
    catch {
        Add-Check "matter world-status-json works" $false $_.Exception.Message
    }
}

if ($matterExists) {
    try {
        $frontierOutput = & $matterExe frontier-status-json
        $frontierExit = $LASTEXITCODE
        $frontierJson = $frontierOutput | ConvertFrom-Json
        $frontierOk = $frontierExit -eq 0 `
            -and $frontierJson.ok `
            -and $frontierJson.summary.all_non_stub `
            -and $frontierJson.summary.all_simulated `
            -and (-not $frontierJson.summary.any_hardware)
        Add-Check "matter frontier-status-json works" $frontierOk "exit=$frontierExit"
    }
    catch {
        Add-Check "matter frontier-status-json works" $false $_.Exception.Message
    }
}

if ($coreSchemaExists -and $coreStatusExists) {
    try {
        $coreArtifact = Get-Content $coreStatusPath -Raw | ConvertFrom-Json
        $schemaRef = $coreArtifact.PSObject.Properties['$schema'].Value
        Add-Check "core status artifact schema" ($schemaRef -eq "schemas/core-status.schema.json") ([string]$schemaRef)
        Add-Check "core status artifact claim" ($coreArtifact.summary.claim -eq "experimental_language_runtime") ([string]$coreArtifact.summary.claim)
    }
    catch {
        Add-Check "core status artifact parses" $false $_.Exception.Message
    }
}

if ($worldSchemaExists -and $worldStatusExists) {
    try {
        $worldArtifact = Get-Content $worldStatusPath -Raw | ConvertFrom-Json
        $schemaRef = $worldArtifact.PSObject.Properties['$schema'].Value
        Add-Check "world status artifact schema" ($schemaRef -eq "schemas/world-status.schema.json") ([string]$schemaRef)
        Add-Check "world status artifact mode" ($worldArtifact.summary.mode -eq "logical_world_partition") ([string]$worldArtifact.summary.mode)
    }
    catch {
        Add-Check "world status artifact parses" $false $_.Exception.Message
    }
}

if ($frontierSchemaExists -and $frontierStatusExists) {
    try {
        $frontierArtifact = Get-Content $frontierStatusPath -Raw | ConvertFrom-Json
        $schemaRef = $frontierArtifact.PSObject.Properties['$schema'].Value
        Add-Check "frontier status artifact schema" ($schemaRef -eq "schemas/frontier-status.schema.json") ([string]$schemaRef)
        Add-Check "frontier status artifact reality flags" `
            ($frontierArtifact.summary.all_non_stub -and $frontierArtifact.summary.all_simulated -and (-not $frontierArtifact.summary.any_hardware)) `
            "non_stub=$($frontierArtifact.summary.all_non_stub); simulated=$($frontierArtifact.summary.all_simulated); any_hardware=$($frontierArtifact.summary.any_hardware)"
    }
    catch {
        Add-Check "frontier status artifact parses" $false $_.Exception.Message
    }
}

if ($matterExists -and $firstRunExists) {
    try {
        $runOutput = & $matterExe run $firstRun
        $runExit = $LASTEXITCODE
        Add-Check "matter runs first_run example" ($runExit -eq 0 -and (($runOutput -join "`n").Contains("Matter"))) "exit=$runExit"
    }
    catch {
        Add-Check "matter runs first_run example" $false $_.Exception.Message
    }
}

$checkArray = @($checks.ToArray())
$failed = @($checkArray | Where-Object { -not [bool]$_["ok"] })
$result = [ordered]@{
    "ok" = ($failed.Count -eq 0)
    "install_dir" = $installRoot
    "failed_checks" = $failed.Count
    "checks" = $checkArray
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
}
else {
    Write-Host "Matter local install diagnosis" -ForegroundColor Cyan
    Write-Host "Install dir: $installRoot" -ForegroundColor Cyan
    foreach ($check in $checkArray) {
        $status = if ($check["ok"]) { "OK" } else { "FAIL" }
        $color = if ($check["ok"]) { "Green" } else { "Red" }
        Write-Host ("[{0}] {1} - {2}" -f $status, $check["name"], $check["detail"]) -ForegroundColor $color
    }
}

if ($failed.Count -gt 0) {
    exit 1
}
