# Matter Core - Scripts

Scripts de automação e testes para o Matter Core.

---

## 📦 Instalação

### install.ps1
Instala o Matter CLI globalmente no sistema (requer admin).

```powershell
.\scripts\install.ps1
```

### install-local.ps1
Instala o Matter CLI localmente no diretório do usuário.

```powershell
.\scripts\install-local.ps1
```

### uninstall.ps1
Remove a instalação global do Matter CLI.

```powershell
.\scripts\uninstall.ps1
```

### uninstall-local.ps1
Remove a instalação local do Matter CLI.

```powershell
.\scripts\uninstall-local.ps1
```

---

## 🧪 Testes

### test_all.ps1
Executa todos os testes do projeto.

```powershell
.\scripts\test_all.ps1
```

### test_api_bridge.ps1
Testa a ponte entre API e CLI.

```powershell
.\scripts\test_api_bridge.ps1
```

### test_bytecode_equivalence.ps1
Testa equivalência entre source e bytecode.

```powershell
.\scripts\test_bytecode_equivalence.ps1
```

### test_repl_simple.ps1
Testa funcionalidade básica do REPL.

```powershell
.\scripts\test_repl_simple.ps1
```

### test_repl_persistent.ps1
Testa estado persistente do REPL.

```powershell
.\scripts\test_repl_persistent.ps1
```

### validate-full-workspace.ps1
Validação completa do workspace com hardening (`fmt --check`, `clippy -D warnings`, `tests`) e etapa LLVM quando disponível.

```powershell
.\scripts\validate-full-workspace.ps1
```

Opções úteis:

```powershell
# Exigir LLVM 17 (falha se não estiver pronto)
.\scripts\validate-full-workspace.ps1 -RequireLLVM

# Gerar resumo JSON
.\scripts\validate-full-workspace.ps1 -JsonSummary

# Rodar preflight antes da validação (fail-fast)
.\scripts\validate-full-workspace.ps1 -RunPreflight

# Rodar preflight com requisito maior de espaço livre (ex.: 20 GB)
.\scripts\validate-full-workspace.ps1 -RunPreflight -PreflightMinFreeGB 20
```

### preflight-env.ps1
Pré-checagem rápida de ambiente (ferramentas, espaço em disco e prontidão LLVM 17) antes da validação completa.

```powershell
.\scripts\preflight-env.ps1
```

Opções úteis:

```powershell
# Exigir mais espaço livre no disco de build (ex.: 20 GB)
.\scripts\preflight-env.ps1 -MinFreeGB 20
```

---

## 📝 Uso

### Executar Script

```powershell
# Navegar até a pasta do projeto
cd "caminho\para\matter-core"

# Executar script
.\scripts\nome_do_script.ps1
```

### Permissões

Se encontrar erro de execução, habilite scripts:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

## 🔧 Desenvolvimento

### Adicionar Novo Script

1. Criar arquivo `.ps1` na pasta `scripts/`
2. Adicionar documentação neste README
3. Testar o script
4. Commit

### Convenções

- Usar nomes descritivos: `test_feature.ps1`, `install_component.ps1`
- Adicionar comentários no código
- Tratar erros adequadamente
- Documentar parâmetros e uso

---

**Última atualização:** 11 de Maio de 2026
