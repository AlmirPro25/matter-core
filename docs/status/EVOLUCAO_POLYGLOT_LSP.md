# Evolução continuada — polyglot + disco + LSP

## Disco C:
- Antes: ~0.07 GB livres
- Depois de limpeza (Temp, caches, pycache): ~0.30 GB livres
- Ainda **crítico**. Maiores pastas:
  - .local\share\opencode ~3.2 GB (app — não apaguei)
  - AppData\Local\Programs (VS Code/ZCode) ~1.9 GB
  - OpenAI/Codex ~0.65 GB

## Polyglot
- python ready=true, node ready=true, rust ready=true
- go/java: não instalados
- polyglot_runtime_smoke.matter: PASS
- python_math_smoke.matter: novo smoke stdlib
- python_numpy.matter: reescrito para API runtime (python.import_module / python.call)
- Syntax antiga import \"x\" from python ainda **não parseia** (gap de linguagem)

## LSP
- crate matter-lsp compila/testa
- CLI: matter-cli lsp
- Launcher: scripts\start-matter-lsp.ps1

## Install
- D:\Matter slim install OK
