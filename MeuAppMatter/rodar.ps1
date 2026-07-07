param(
    [switch]$Interactive
)

# Caminho para o repositório Matter Core
$MatterCorePath = "F:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
$AppFile = "app.matter"

Write-Host "Executando $AppFile via Matter CLI..." -ForegroundColor Cyan

# Executa o aplicativo através do workspace Matter
Push-Location $MatterCorePath
cargo run -q -p matter-cli -- run "F:\Users\almir\Desktop\MeuAppMatter\$AppFile"
Pop-Location

if (-not $Interactive) {
    Write-Host "`nPressione qualquer tecla para sair..." -ForegroundColor Yellow
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}
