# Matter Language Local Installer (Sem Admin)
# Instala Matter na pasta do usuário

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   Matter Language Installer v0.1.5    " -ForegroundColor Cyan
Write-Host "   (Instalacao Local - Sem Admin)      " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Diretório de instalação (pasta do usuário)
$installDir = "$env:LOCALAPPDATA\Matter"
$binDir = "$installDir\bin"

Write-Host "Instalando Matter em: $installDir" -ForegroundColor Green
Write-Host ""

# Criar diretórios
Write-Host "[1/5] Criando diretorios..." -ForegroundColor Yellow
if (Test-Path $installDir) {
    Write-Host "  - Removendo instalacao anterior..." -ForegroundColor Gray
    Remove-Item -Path $installDir -Recurse -Force
}
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
New-Item -ItemType Directory -Path $binDir -Force | Out-Null
Write-Host "  OK" -ForegroundColor Green

# Compilar em release
Write-Host "[2/5] Compilando Matter..." -ForegroundColor Yellow
Write-Host "  - Isso pode levar 1-2 minutos..." -ForegroundColor Gray
$buildOutput = cargo build --release 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  ERRO ao compilar!" -ForegroundColor Red
    Write-Host $buildOutput
    pause
    exit 1
}
Write-Host "  OK" -ForegroundColor Green

# Copiar executável
Write-Host "[3/5] Copiando arquivos..." -ForegroundColor Yellow
Copy-Item "target\release\matter-cli.exe" "$binDir\matter.exe" -Force
Write-Host "  - matter.exe copiado" -ForegroundColor Gray

# Copiar exemplos
$examplesDir = "$installDir\examples"
New-Item -ItemType Directory -Path $examplesDir -Force | Out-Null
Copy-Item "examples\*" $examplesDir -Recurse -Force
Write-Host "  - Exemplos copiados" -ForegroundColor Gray

# Copiar documentação
$docsDir = "$installDir\docs"
New-Item -ItemType Directory -Path $docsDir -Force | Out-Null
Copy-Item "docs\*" $docsDir -Recurse -Force
Copy-Item "README.md" $installDir -Force
Write-Host "  - Documentacao copiada" -ForegroundColor Gray
Write-Host "  OK" -ForegroundColor Green

# Adicionar ao PATH do usuário
Write-Host "[4/5] Adicionando ao PATH do usuario..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$binDir*") {
    if ($currentPath) {
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$binDir", "User")
    } else {
        [Environment]::SetEnvironmentVariable("Path", "$binDir", "User")
    }
    Write-Host "  - PATH atualizado" -ForegroundColor Gray
} else {
    Write-Host "  - PATH ja configurado" -ForegroundColor Gray
}
Write-Host "  OK" -ForegroundColor Green

# Criar arquivo de informações
Write-Host "[5/5] Criando arquivos de informacao..." -ForegroundColor Yellow
$infoContent = @"
Matter Language v0.1.5
Instalado em: $installDir

Para usar:
  matter --help
  matter run programa.matter

Exemplos em: $examplesDir
Documentacao em: $docsDir

Para desinstalar, execute: uninstall-local.ps1
"@
$infoContent | Out-File "$installDir\INFO.txt" -Encoding UTF8
Write-Host "  OK" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "   Instalacao concluida com sucesso!   " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Matter foi instalado em:" -ForegroundColor Cyan
Write-Host "  $installDir" -ForegroundColor White
Write-Host ""
Write-Host "Para usar, abra um NOVO terminal e digite:" -ForegroundColor Yellow
Write-Host "  matter --help" -ForegroundColor White
Write-Host ""
Write-Host "Testar agora? (s/n)" -ForegroundColor Yellow
$test = Read-Host
if ($test -eq "s" -or $test -eq "S") {
    Write-Host ""
    Write-Host "Testando instalacao..." -ForegroundColor Cyan
    & "$binDir\matter.exe" --help
    Write-Host ""
    Write-Host "Teste concluido!" -ForegroundColor Green
}
Write-Host ""
Write-Host "IMPORTANTE: Feche e abra um novo terminal para o PATH funcionar!" -ForegroundColor Red
Write-Host ""
Write-Host "Exemplos disponiveis em:" -ForegroundColor Yellow
Write-Host "  $examplesDir" -ForegroundColor White
Write-Host ""
pause
