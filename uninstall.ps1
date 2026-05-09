# Matter Language Uninstaller for Windows

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Matter Language Uninstaller v0.1.5   " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Verificar se está rodando como administrador
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERRO: Este desinstalador precisa ser executado como Administrador!" -ForegroundColor Red
    Write-Host ""
    pause
    exit 1
}

$installDir = "C:\Program Files\Matter"
$binDir = "$installDir\bin"

Write-Host "Desinstalando Matter..." -ForegroundColor Yellow
Write-Host ""

# Remover do PATH
Write-Host "[1/3] Removendo do PATH..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($currentPath -like "*$binDir*") {
    $newPath = $currentPath -replace [regex]::Escape(";$binDir"), ""
    $newPath = $newPath -replace [regex]::Escape("$binDir;"), ""
    [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "  - Nao estava no PATH" -ForegroundColor Gray
}

# Remover atalhos
Write-Host "[2/3] Removendo atalhos..." -ForegroundColor Yellow
$startMenuDir = "$env:ProgramData\Microsoft\Windows\Start Menu\Programs\Matter"
if (Test-Path $startMenuDir) {
    Remove-Item -Path $startMenuDir -Recurse -Force
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "  - Atalhos nao encontrados" -ForegroundColor Gray
}

# Remover arquivos
Write-Host "[3/3] Removendo arquivos..." -ForegroundColor Yellow
if (Test-Path $installDir) {
    Remove-Item -Path $installDir -Recurse -Force
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "  - Arquivos nao encontrados" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Matter foi desinstalado com sucesso!  " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
pause
