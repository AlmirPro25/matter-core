# Script de Organizacao do Matter Core
$ErrorActionPreference = "Stop"

Write-Host "Organizando Matter Core..." -ForegroundColor Cyan

$root = "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"

# Criar pastas
Write-Host "Criando estrutura de pastas..."
$folders = @(
    "docs\archive",
    "docs\sprints", 
    "docs\sessions",
    "docs\vision",
    "docs\technical",
    "docs\guides"
)

foreach ($f in $folders) {
    $path = Join-Path $root $f
    if (-not (Test-Path $path)) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        Write-Host "  Criado: $f"
    }
}

# Mover sprints
Write-Host "Movendo sprints..."
Get-ChildItem -Path $root -Filter "SPRINT_*.md" -File | ForEach-Object {
    Move-Item $_.FullName -Destination (Join-Path $root "docs\sprints") -Force
}

# Mover sessions
Write-Host "Movendo sessions..."
Get-ChildItem -Path $root -Filter "SESSION_*.md" -File | ForEach-Object {
    Move-Item $_.FullName -Destination (Join-Path $root "docs\sessions") -Force
}
Get-ChildItem -Path $root -Filter "SESSAO_*.md" -File | ForEach-Object {
    Move-Item $_.FullName -Destination (Join-Path $root "docs\sessions") -Force
}

# Mover vision
Write-Host "Movendo vision..."
$visionPatterns = @("MATTER_VISION_*.md", "MATTER_POLYGLOT_*.md", "MANIFESTO_*.md", "STRATEGIC_VISION.md", "ROADMAP_*.md")
foreach ($pattern in $visionPatterns) {
    Get-ChildItem -Path $root -Filter $pattern -File | ForEach-Object {
        Move-Item $_.FullName -Destination (Join-Path $root "docs\vision") -Force
    }
}

# Mover guides
Write-Host "Movendo guides..."
$guidePatterns = @("START_HERE*.md", "QUICK_*.md", "INSTALL_*.md", "DEPLOYMENT_*.md")
foreach ($pattern in $guidePatterns) {
    Get-ChildItem -Path $root -Filter $pattern -File | ForEach-Object {
        Move-Item $_.FullName -Destination (Join-Path $root "docs\guides") -Force
    }
}

# Mover technical
Write-Host "Movendo technical..."
$techPatterns = @("MATTER_CORE_*.md", "MATTER_NATIVE_*.md", "MATTER_COMPILER_*.md", "MATTER_TECHNICAL_*.md", "MATTER_SCIENTIFIC_*.md", "NATIVE_COMPILER_*.md", "OPTIMIZATION_*.md")
foreach ($pattern in $techPatterns) {
    Get-ChildItem -Path $root -Filter $pattern -File | ForEach-Object {
        Move-Item $_.FullName -Destination (Join-Path $root "docs\technical") -Force
    }
}

# Mover archive (tudo que sobrou com MATTER_)
Write-Host "Movendo para archive..."
Get-ChildItem -Path $root -Filter "MATTER_*.md" -File | ForEach-Object {
    Move-Item $_.FullName -Destination (Join-Path $root "docs\archive") -Force
}

# Mover outros para archive
$archivePatterns = @("PROJECT_*.md", "FINAL_*.md", "CELEBRATION_*.md", "ACHIEVEMENT_*.md", "EXECUTIVE_*.md", "RELEASE_*.md", "DOCUMENTATION_*.md", "STATUS*.md", "ACTION_*.md", "PLANO_*.md", "RESUMO_*.md", "ANALISE_*.md")
foreach ($pattern in $archivePatterns) {
    Get-ChildItem -Path $root -Filter $pattern -File | ForEach-Object {
        if ($_.Name -ne "PROGRESS.md") {
            Move-Item $_.FullName -Destination (Join-Path $root "docs\archive") -Force
        }
    }
}

Write-Host "Organizacao completa!" -ForegroundColor Green
Write-Host ""
Write-Host "Agora execute: .\move_project.ps1" -ForegroundColor Yellow
