# Script para mover projeto para caminho sem espacos
$ErrorActionPreference = "Stop"

$source = "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
$dest = "f:\Users\almir\Desktop\matter-core"

Write-Host "Movendo projeto..." -ForegroundColor Cyan
Write-Host "De: $source"
Write-Host "Para: $dest"
Write-Host ""

if (Test-Path $dest) {
    Write-Host "ERRO: Destino ja existe!" -ForegroundColor Red
    Write-Host "Delete manualmente: $dest" -ForegroundColor Yellow
    exit 1
}

Write-Host "Movendo... (pode levar alguns minutos)" -ForegroundColor Yellow
Move-Item -Path $source -Destination $dest -Force

Write-Host ""
Write-Host "SUCESSO!" -ForegroundColor Green
Write-Host ""
Write-Host "Proximos passos:" -ForegroundColor Yellow
Write-Host "  cd $dest"
Write-Host "  cargo build --release"
Write-Host "  cargo test"
Write-Host "  .\target\release\matter-cli.exe run examples\first_run.matter"
Write-Host ""
