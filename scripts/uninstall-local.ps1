param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Matter",
    [switch]$NoPath,
    [switch]$NoPause,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$installRoot = [System.IO.Path]::GetFullPath($InstallDir)
$manifestPath = Join-Path $installRoot "INSTALL_MANIFEST.json"
$binDir = Join-Path $installRoot "bin"

function Assert-SafeInstallRoot {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        return
    }

    $root = [System.IO.Path]::GetPathRoot($Path)
    if ($Path.TrimEnd('\') -eq $root.TrimEnd('\')) {
        throw "Refusing to uninstall from filesystem root: $Path"
    }

    if (-not (Test-Path $manifestPath -PathType Leaf)) {
        if ($Force) {
            Write-Host "  - Manifesto ausente; prosseguindo por -Force" -ForegroundColor Yellow
            return
        }
        throw "Refusing to remove '$Path' because INSTALL_MANIFEST.json was not found. Use -Force only if this is intentionally a Matter install directory."
    }

    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.schema -ne "matter.release.install.v1") {
        if ($Force) {
            Write-Host "  - Manifesto inesperado; prosseguindo por -Force" -ForegroundColor Yellow
            return
        }
        throw "Refusing to remove '$Path' because INSTALL_MANIFEST.json has unexpected schema: $($manifest.schema)"
    }

    $manifestInstallDir = [System.IO.Path]::GetFullPath([string]$manifest.install_dir)
    if ($manifestInstallDir.TrimEnd('\') -ne $Path.TrimEnd('\')) {
        if ($Force) {
            Write-Host "  - Manifesto aponta para outro diretorio; prosseguindo por -Force" -ForegroundColor Yellow
            return
        }
        throw "Refusing to remove '$Path' because manifest install_dir is '$manifestInstallDir'"
    }
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Matter Language Uninstaller v0.1.5   " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Desinstalando Matter..." -ForegroundColor Yellow
Write-Host ""

Assert-SafeInstallRoot $installRoot

# Remover do PATH
Write-Host "[1/2] Removendo do PATH..." -ForegroundColor Yellow
if ($NoPath) {
    Write-Host "  - PATH preservado por -NoPath" -ForegroundColor Gray
}
else {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @()
    if ($currentPath) {
        $pathEntries = $currentPath -split ';' | Where-Object { $_ }
    }

    if ($pathEntries -contains $binDir) {
        $newPath = ($pathEntries | Where-Object { $_ -ne $binDir }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "  OK" -ForegroundColor Green
    } else {
        Write-Host "  - Nao estava no PATH" -ForegroundColor Gray
    }
}

# Remover arquivos
Write-Host "[2/2] Removendo arquivos..." -ForegroundColor Yellow
if (Test-Path $installRoot) {
    Remove-Item -LiteralPath $installRoot -Recurse -Force
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "  - Arquivos nao encontrados" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Matter foi desinstalado com sucesso!  " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
if (-not $NoPause) {
    pause
}
