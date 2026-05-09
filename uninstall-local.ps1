# Matter Language Local Uninstaller

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Matter Language Uninstaller v0.1.5   " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$installDir = "$env:LOCALAPPDATA\Matter"
$binDir = "$installDir\bin"

Write-Host "Desinstalando Matter..." -ForegroundColor Yellow
Write-Host ""

# Remover do PATH
Write-Host "[1/2] Removendo do PATH..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -like "*$binDir*") {
    $newPath = $currentPath -replace [regex]::Escape(";$binDir"), ""
    $newPath = $newPath -replace [regex]::Escape("$binDir;"), ""
    $newPath = $newPath -replace [regex]::Escape("$binDir"), ""
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "  OK" -ForegroundColor Green
} else {
    Write-Host "  - Nao estava no PATH" -ForegroundColor Gray
}

# Remover arquivos
Write-Host "[2/2] Removendo arquivos..." -ForegroundColor Yellow
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
