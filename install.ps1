# Matter Language Installer for Windows
# Instala Matter no sistema como Node.js

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   Matter Language Installer v0.1.5    " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Verificar se está rodando como administrador
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERRO: Este instalador precisa ser executado como Administrador!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Clique com botao direito no PowerShell e escolha 'Executar como Administrador'" -ForegroundColor Yellow
    Write-Host ""
    pause
    exit 1
}

# Diretório de instalação
$installDir = "C:\Program Files\Matter"
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

# Adicionar ao PATH
Write-Host "[4/5] Adicionando ao PATH do sistema..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($currentPath -notlike "*$binDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$binDir", "Machine")
    Write-Host "  - PATH atualizado" -ForegroundColor Gray
} else {
    Write-Host "  - PATH ja configurado" -ForegroundColor Gray
}
Write-Host "  OK" -ForegroundColor Green

# Criar atalhos
Write-Host "[5/5] Criando atalhos..." -ForegroundColor Yellow

# Atalho no Menu Iniciar
$startMenuDir = "$env:ProgramData\Microsoft\Windows\Start Menu\Programs\Matter"
New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null

$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$startMenuDir\Matter REPL.lnk")
$Shortcut.TargetPath = "powershell.exe"
$Shortcut.Arguments = "-NoExit -Command `"& '$binDir\matter.exe' repl`""
$Shortcut.WorkingDirectory = "$env:USERPROFILE"
$Shortcut.Description = "Matter Language REPL"
$Shortcut.Save()

Write-Host "  - Atalho no Menu Iniciar criado" -ForegroundColor Gray
Write-Host "  OK" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "   Instalacao concluida com sucesso!   " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Matter foi instalado em: $installDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "Para usar, abra um NOVO terminal e digite:" -ForegroundColor Yellow
Write-Host "  matter --help" -ForegroundColor White
Write-Host ""
Write-Host "Exemplos disponiveis em:" -ForegroundColor Yellow
Write-Host "  $examplesDir" -ForegroundColor White
Write-Host ""
Write-Host "Documentacao em:" -ForegroundColor Yellow
Write-Host "  $docsDir" -ForegroundColor White
Write-Host ""
Write-Host "IMPORTANTE: Feche e abra um novo terminal para o PATH funcionar!" -ForegroundColor Red
Write-Host ""
pause
