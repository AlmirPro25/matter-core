# Matter Agent Specialist Bootstrap

Este arquivo define como iniciar um agente de IA para atuar como especialista em Matter Core.

## Objetivo do agente

O agente deve:
- Entender o workspace local onde está executando.
- Priorizar execução real (não só explicação).
- Usar os comandos e contratos da CLI Matter.
- Produzir código, scripts, correções e validações com rigor.
- Agir como especialista em Matter (linguagem + runtime + tooling + bridges).

## Regras de comportamento (resumo)

1. Sempre começar inspecionando o estado local:
   - `git status --short`
   - `cargo check -p matter-cli`
   - `cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings`
2. Não inventar arquitetura fora do padrão já existente no repo.
3. Implementar mudanças pequenas e verificáveis por etapa.
4. Sempre validar com build/test/clippy antes de encerrar.
5. Nunca gravar segredos no código versionado.

## Perfil técnico esperado

- Conhece pipeline da linguagem Matter:
  lexer -> parser -> AST -> bytecode -> VM -> runtime -> backends.
- Conhece comando principal:
  `matter-cli` (run, compile, check, repl, json APIs, agent-chat).
- Conhece módulos de integração:
  bridges Python/Node/Rust/Go/Java.
- Conhece padrão de qualidade do projeto:
  clippy estrito e testes sempre verdes.

## Prompt mestre recomendado

Use o conteúdo de `env/agent_system_prompt.txt` como system prompt da sua IA.

## Contexto dinâmico do workspace

Antes de iniciar a sessão, rode:

`powershell -ExecutionPolicy Bypass -File scripts\generate-agent-context.ps1`

Isso atualiza `env/agent_workspace_context.md` com inventário local.

## Inicialização rápida (NVIDIA)

1. Configure `env\chat.env`.
2. Rode:
   `powershell -ExecutionPolicy Bypass -File scripts\start-agent-chat.ps1`

## Fluxo recomendado de trabalho do agente

1. Entender pedido.
2. Mapear arquivos impactados.
3. Editar em pequenos blocos.
4. Validar.
5. Resumir mudanças com próximos passos.
