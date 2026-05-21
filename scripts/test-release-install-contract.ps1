param(
    [string]$PackageRoot = "dist\matter-core-windows-x64",
    [string]$ZipPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$tempExtractRoot = $null
$installRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_install_" + [guid]::NewGuid().ToString("N"))

if ($ZipPath) {
    if (-not (Test-Path $ZipPath -PathType Leaf)) {
        throw "Release package zip not found: $ZipPath"
    }

    $tempExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_install_zip_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force $tempExtractRoot | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $tempExtractRoot -Force

    $directPackageRoot = Join-Path $tempExtractRoot "matter-core-windows-x64"
    if (Test-Path $directPackageRoot -PathType Container) {
        $PackageRoot = $directPackageRoot
    }
    else {
        $PackageRoot = $tempExtractRoot
    }
}

try {
    if (-not (Test-Path $PackageRoot -PathType Container)) {
        throw "Release package root not found: $PackageRoot"
    }

    $installer = Join-Path $PackageRoot "scripts\install-release-local.ps1"
    if (-not (Test-Path $installer -PathType Leaf)) {
        throw "Release package installer not found: scripts\install-release-local.ps1"
    }

    & powershell -ExecutionPolicy Bypass -File $installer -InstallDir $installRoot -NoPath
    if ($LASTEXITCODE -ne 0) {
        throw "Release installer failed with exit code $LASTEXITCODE"
    }

    $matterExe = Join-Path $installRoot "bin\matter.exe"
    $matterCliExe = Join-Path $installRoot "bin\matter-cli.exe"
    $firstRun = Join-Path $installRoot "examples\first_run.matter"
    $installManifest = Join-Path $installRoot "INSTALL_MANIFEST.json"
    $diagnoser = Join-Path $installRoot "scripts\diagnose-local-install.ps1"
    foreach ($required in @($matterExe, $matterCliExe, $firstRun, (Join-Path $installRoot "INFO.txt"), $installManifest, $diagnoser)) {
        if (-not (Test-Path $required -PathType Leaf)) {
            throw "Installed release missing expected file: $required"
        }
    }

    $manifest = Get-Content $installManifest -Raw | ConvertFrom-Json
    if ($manifest.schema -ne "matter.release.install.v1") {
        throw "Installed release manifest has unexpected schema: $($manifest.schema)"
    }
    if ($manifest.path_modified) {
        throw "Installed release manifest should report path_modified=false when -NoPath is used"
    }
    if ($manifest.post_install_check -ne "capabilities-json") {
        throw "Installed release manifest did not record the post-install check"
    }
    $binaryEntries = @($manifest.installed_binaries)
    if ($binaryEntries.Count -ne 2) {
        throw "Installed release manifest should record exactly two installed binaries"
    }
    $matterHash = (Get-FileHash -LiteralPath $matterExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $matterEntry = $binaryEntries | Where-Object { $_.path -eq "bin\matter.exe" } | Select-Object -First 1
    if (-not $matterEntry -or $matterEntry.sha256 -ne $matterHash) {
        throw "Installed release manifest does not match bin\matter.exe hash"
    }

    $capabilities = & $matterExe capabilities-json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter capabilities-json failed with exit code $LASTEXITCODE"
    }
    $capabilitiesJson = $capabilities | ConvertFrom-Json
    if (-not $capabilitiesJson.ok) {
        throw "Installed matter capabilities-json did not report ok=true"
    }

    $runOutput = & $matterExe run $firstRun
    if ($LASTEXITCODE -ne 0) {
        throw "Installed matter run first_run failed with exit code $LASTEXITCODE"
    }
    if (-not (($runOutput -join "`n").Contains("Matter"))) {
        throw "Installed matter first_run output did not contain expected Matter marker"
    }

    $diagnosis = & powershell -NoProfile -ExecutionPolicy Bypass -File $diagnoser -InstallDir $installRoot -AllowMissingPath -Json
    if ($LASTEXITCODE -ne 0) {
        throw "Installed release diagnosis failed with exit code $LASTEXITCODE"
    }
    $diagnosisJson = $diagnosis | ConvertFrom-Json
    if (-not $diagnosisJson.ok) {
        throw "Installed release diagnosis did not report ok=true"
    }

    $uninstaller = Join-Path $installRoot "scripts\uninstall-local.ps1"
    if (-not (Test-Path $uninstaller -PathType Leaf)) {
        throw "Installed release missing uninstaller: $uninstaller"
    }

    $unsafeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_uninstall_guard_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force $unsafeRoot | Out-Null
    Set-Content -Path (Join-Path $unsafeRoot "not-matter.txt") -Value "not a Matter install" -Encoding UTF8
    try {
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & powershell -NoProfile -ExecutionPolicy Bypass -File $uninstaller -InstallDir $unsafeRoot -NoPath -NoPause > $null 2> $null
        $guardExitCode = $LASTEXITCODE
        $ErrorActionPreference = $oldErrorActionPreference
        if ($guardExitCode -eq 0) {
            throw "Installed release uninstaller should refuse a directory without INSTALL_MANIFEST.json"
        }
        if (-not (Test-Path $unsafeRoot -PathType Container)) {
            throw "Installed release uninstaller removed an unsafe directory without manifest"
        }
    }
    finally {
        $ErrorActionPreference = "Stop"
        if (Test-Path $unsafeRoot) {
            Remove-Item -LiteralPath $unsafeRoot -Recurse -Force
        }
    }

    & powershell -ExecutionPolicy Bypass -File $uninstaller -InstallDir $installRoot -NoPath -NoPause
    if ($LASTEXITCODE -ne 0) {
        throw "Installed release uninstaller failed with exit code $LASTEXITCODE"
    }
    if (Test-Path $installRoot) {
        throw "Installed release uninstaller did not remove install root: $installRoot"
    }

    [ordered]@{
        ok = $true
        package_root = $PackageRoot
        zip_path = if ($ZipPath) { $ZipPath } else { $null }
        install_root = $installRoot
        checked = @(
                    "installer copied matter.exe",
                    "installer copied matter-cli.exe",
                    "installer copied first_run example",
                    "installer copied diagnosis script",
                    "installer wrote install manifest",
                    "install manifest hashes matter.exe",
                    "installer post-install check passed",
                    "installed CLI capabilities-json works",
                    "installed CLI runs first_run",
                    "installed diagnosis passes",
                    "installed uninstaller refuses unsafe directory",
                    "installed uninstaller removes install root"
        )
    } | ConvertTo-Json -Depth 4
}
finally {
    foreach ($path in @($installRoot, $tempExtractRoot)) {
        if ($path -and (Test-Path $path)) {
            Remove-Item -LiteralPath $path -Recurse -Force
        }
    }
}
