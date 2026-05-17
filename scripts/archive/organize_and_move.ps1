# ============================================================================
# SCRIPT DE ORGANIZAÇÃO E MOVIMENTAÇÃO DO MATTER CORE
# ============================================================================
# Este script:
# 1. Cria estrutura organizada de documentação
# 2. Move arquivos para pastas apropriadas
# 3. Move o projeto inteiro para caminho sem espaços
# ============================================================================

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "ORGANIZANDO MATTER CORE" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$projectRoot = "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
$newRoot = "f:\Users\almir\Desktop\matter-core"

# ============================================================================
# FASE 1: CRIAR ESTRUTURA DE PASTAS
# ============================================================================

Write-Host "[1/4] Criando estrutura de pastas..." -ForegroundColor Yellow

$folders = @(
    "docs\archive",
    "docs\sprints",
    "docs\sessions",
    "docs\vision",
    "docs\technical",
    "docs\guides"
)

foreach ($folder in $folders) {
    $path = Join-Path $projectRoot $folder
    if (-not (Test-Path $path)) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        Write-Host "  ✓ Criado: $folder" -ForegroundColor Green
    }
}

# ============================================================================
# FASE 2: MOVER ARQUIVOS PARA PASTAS APROPRIADAS
# ============================================================================

Write-Host ""
Write-Host "[2/4] Movendo arquivos..." -ForegroundColor Yellow

# Arquivos que devem ficar na raiz
$keepInRoot = @(
    "README.md",
    "LICENSE",
    "Cargo.toml",
    "Cargo.lock",
    ".gitignore",
    "matter.toml",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "PROGRESS.md",
    "REALIDADE_ATUAL_HONESTA.md"
)

# Padrões de arquivos para mover
$movePatterns = @{
    "docs\sprints" = @("SPRINT_*.md")
    "docs\sessions" = @("SESSION_*.md", "SESSAO_*.md")
    "docs\vision" = @(
        "MATTER_VISION_*.md",
        "MATTER_POLYGLOT_*.md",
        "MATTER_FOR_*.md",
        "MANIFESTO_*.md",
        "STRATEGIC_VISION.md",
        "MATTER_ROADMAP_*.md",
        "ROADMAP_*.md",
        "VISUAL_ROADMAP.md"
    )
    "docs\technical" = @(
        "MATTER_CORE_*.md",
        "MATTER_NATIVE_*.md",
        "MATTER_COMPILER_*.md",
        "MATTER_TECHNICAL_*.md",
        "MATTER_SCIENTIFIC_*.md",
        "NATIVE_COMPILER_*.md",
        "OPTIMIZATION_*.md"
    )
    "docs\guides" = @(
        "START_HERE*.md",
        "QUICK_*.md",
        "INSTALL_*.md",
        "DEPLOYMENT_*.md",
        "MATTER_QUICK_*.md",
        "LAUNCH_*.md",
        "VALIDATION_*.md"
    )
    "docs\archive" = @(
        "MATTER_V*.md",
        "MATTER_COMPLETE_*.md",
        "MATTER_FINAL_*.md",
        "MATTER_ULTIMATE_*.md",
        "MATTER_ABSOLUTE_*.md",
        "MATTER_REVOLUTIONARY_*.md",
        "MATTER_ACHIEVEMENT*.md",
        "MATTER_MAGNUM_*.md",
        "PROJECT_COMPLETE.md",
        "PROJECT_STATUS_*.md",
        "FINAL_*.md",
        "CELEBRATION_*.md",
        "ACHIEVEMENT_*.md",
        "EXECUTIVE_SUMMARY*.md",
        "MATTER_EXECUTIVE_*.md",
        "MATTER_PITCH_*.md",
        "MATTER_SHOWCASE.md",
        "MATTER_IN_ACTION.md",
        "MATTER_CAPABILITIES.md",
        "MATTER_ENTERPRISE_*.md",
        "MATTER_GLOBAL_*.md",
        "MATTER_UNIVERSAL_*.md",
        "MATTER_REAL_WORLD_*.md",
        "MATTER_FRONTIER_*.md",
        "RELEASE_NOTES_*.md",
        "README_*.md",
        "DOCUMENTATION_*.md",
        "ALL_DOCUMENTS.md",
        "EVOLUTION_*.md",
        "JOURNEY.md",
        "THE_MATTER_STORY.md",
        "CURRENT_STATUS.md",
        "STATUS*.md",
        "NEXT_ACTION.md",
        "ACTION_PLAN_*.md",
        "PLANO_*.md",
        "RESUMO_*.md",
        "ANALISE_*.md",
        "ORGANIZACAO_*.md",
        "PROJECT_STRUCTURE.md"
    )
}

$movedCount = 0

foreach ($destFolder in $movePatterns.Keys) {
    $patterns = $movePatterns[$destFolder]
    $destPath = Join-Path $projectRoot $destFolder
    
    foreach ($pattern in $patterns) {
        $files = Get-ChildItem -Path $projectRoot -Filter $pattern -File -ErrorAction SilentlyContinue
        
        foreach ($file in $files) {
            # Não mover se estiver na lista de manter na raiz
            if ($keepInRoot -contains $file.Name) {
                continue
            }
            
            $destFile = Join-Path $destPath $file.Name
            
            # Se arquivo já existe no destino, adicionar timestamp
            if (Test-Path $destFile) {
                $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
                $baseName = [System.IO.Path]::GetFileNameWithoutExtension($file.Name)
                $extension = [System.IO.Path]::GetExtension($file.Name)
                $destFile = Join-Path $destPath "$baseName`_$timestamp$extension"
            }
            
            Move-Item -Path $file.FullName -Destination $destFile -Force
            $movedCount++
        }
    }
}

Write-Host "  ✓ Movidos: $movedCount arquivos" -ForegroundColor Green

# ============================================================================
# FASE 3: CRIAR README DE NAVEGAÇÃO
# ============================================================================

Write-Host ""
Write-Host "[3/4] Criando índice de documentação..." -ForegroundColor Yellow

$indexContent = @"
# Matter Core - Índice de Documentação

## 📁 Estrutura Organizada

### Raiz (Arquivos Essenciais)
- **README.md** - Documentação principal do projeto
- **PROGRESS.md** - Progresso atual do desenvolvimento
- **REALIDADE_ATUAL_HONESTA.md** - Status honesto e realista
- **Cargo.toml** - Configuração do workspace Rust
- **LICENSE** - Licença MIT

### 📂 docs/guides/ - Guias de Início Rápido
Guias práticos para começar a usar o Matter:
- Instalação
- Quick start
- Comandos rápidos
- Deploy

### 📂 docs/sprints/ - Documentação de Sprints
Histórico completo de desenvolvimento (Sprints 1-54):
- Cada sprint com objetivos, implementação e resultados
- Sprints organizados cronologicamente

### 📂 docs/sessions/ - Resumos de Sessões
Resumos de sessões de desenvolvimento:
- Sessões de trabalho
- Marcos alcançados
- Decisões técnicas

### 📂 docs/vision/ - Visão e Estratégia
Documentos de visão de longo prazo:
- Manifesto
- Roadmaps
- Visão estratégica
- Planos polyglot

### 📂 docs/technical/ - Documentação Técnica
Documentação técnica detalhada:
- Arquitetura do compilador
- Otimizações
- Benchmarks científicos
- Especificações técnicas

### 📂 docs/archive/ - Arquivo Histórico
Versões antigas e documentos históricos:
- Versões anteriores (V0.x, V1.x, V2.x, V3.x)
- Summaries antigos
- Documentos "FINAL" e "COMPLETE" históricos

## 🚀 Por Onde Começar?

1. **Novo no projeto?** → Leia `README.md`
2. **Quer status atual?** → Leia `REALIDADE_ATUAL_HONESTA.md`
3. **Quer começar rápido?** → Veja `docs/guides/`
4. **Quer entender a visão?** → Veja `docs/vision/`
5. **Quer detalhes técnicos?** → Veja `docs/technical/`
6. **Quer histórico?** → Veja `docs/sprints/` e `docs/archive/`

## 📊 Estatísticas

- **Crates:** 61
- **Testes:** 374 (100% passing)
- **Linhas de código:** ~213,000
- **Sprints completos:** 54
- **Arquiteturas suportadas:** 3 (x86-64, ARM64, RISC-V)

## ⚠️ Status Atual

**BLOQUEADO:** Projeto não compila devido a espaços no caminho.

**Solução:** Este script move o projeto para `matter-core` (sem espaços).

Após a movimentação:
``````powershell
cd f:\Users\almir\Desktop\matter-core
cargo build --release
``````

---

**Última atualização:** $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
"@

$indexPath = Join-Path $projectRoot "docs\INDEX.md"
Set-Content -Path $indexPath -Value $indexContent -Encoding UTF8
Write-Host "  ✓ Criado: docs\INDEX.md" -ForegroundColor Green

# ============================================================================
# FASE 4: MOVER PROJETO PARA CAMINHO SEM ESPAÇOS
# ============================================================================

Write-Host ""
Write-Host "[4/4] Movendo projeto para caminho sem espaços..." -ForegroundColor Yellow

if (Test-Path $newRoot) {
    Write-Host "  ⚠ Destino já existe: $newRoot" -ForegroundColor Red
    Write-Host "  Pulando movimentação. Delete manualmente se necessário." -ForegroundColor Yellow
} else {
    Write-Host "  Movendo de:" -ForegroundColor Cyan
    Write-Host "    $projectRoot" -ForegroundColor White
    Write-Host "  Para:" -ForegroundColor Cyan
    Write-Host "    $newRoot" -ForegroundColor White
    Write-Host ""
    Write-Host "  Isso pode levar alguns minutos..." -ForegroundColor Yellow
    
    Move-Item -Path $projectRoot -Destination $newRoot -Force
    
    Write-Host ""
    Write-Host "  ✓ Projeto movido com sucesso!" -ForegroundColor Green
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "ORGANIZAÇÃO COMPLETA!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Próximos passos:" -ForegroundColor Yellow
    Write-Host "  1. cd $newRoot" -ForegroundColor White
    Write-Host "  2. cargo build --release" -ForegroundColor White
    Write-Host "  3. cargo test" -ForegroundColor White
    Write-Host "  4. .\target\release\matter-cli.exe run examples\first_run.matter" -ForegroundColor White
    Write-Host ""
}
