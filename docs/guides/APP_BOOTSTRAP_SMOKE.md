# App Bootstrap Smoke

Fluxo rapido para `criar app -> rodar -> validar` com um comando.

## Comando

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\app-bootstrap-smoke.ps1
```

## Opcoes

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\app-bootstrap-smoke.ps1 `
  -ProjectDir target\quick-app `
  -Template basic `
  -WithEnergy
```

## O que ele faz

1. `init-json`
2. `project-check-json`
3. `project-run-json`

Se alguma etapa falhar, o script interrompe com erro explicito.
No sucesso, imprime um JSON resumo com `ok=true`.
