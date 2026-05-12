# Matter Core - Aplicações Práticas

Esta pasta contém aplicações práticas completas que demonstram casos de uso reais do Matter Core.

## 📱 Aplicações Disponíveis

### 1. Counter App (`counter_app.matter`)
**Descrição:** Aplicação de contador com persistência usando store backend.

**Funcionalidades:**
- Incrementar/decrementar contador
- Persistência de estado
- Reset de contador
- Restauração automática ao iniciar

**Conceitos demonstrados:**
- Store backend (persistência)
- Event handlers (on boot)
- Funções
- Estado global

**Como executar:**
```bash
matter run examples/apps/counter_app.matter
```

---

### 2. Weather App (`weather_app.matter`)
**Descrição:** Aplicação de clima que busca e exibe dados meteorológicos.

**Funcionalidades:**
- Buscar clima por cidade
- Exibir relatório formatado
- Salvar histórico de consultas
- Serialização JSON

**Conceitos demonstrados:**
- Maps (estruturas de dados)
- JSON backend
- Time backend
- Store backend
- Formatação de output

**Como executar:**
```bash
matter run examples/apps/weather_app.matter
```

---

### 3. Task Manager (`task_manager.matter`)
**Descrição:** Gerenciador de tarefas completo com CRUD.

**Funcionalidades:**
- Criar tarefas
- Listar tarefas
- Completar tarefas
- Remover tarefas
- Contar estatísticas

**Conceitos demonstrados:**
- Structs (Task)
- Lists (coleção de tarefas)
- CRUD operations
- Loops e iteração
- Estado mutável

**Como executar:**
```bash
matter run examples/apps/task_manager.matter
```

---

### 4. Chat Bot (`chat_bot.matter`)
**Descrição:** Bot de chat simples com respostas baseadas em padrões.

**Funcionalidades:**
- Processar mensagens
- Responder baseado em keywords
- Aprender novas respostas
- Histórico de conversas
- Estatísticas

**Conceitos demonstrados:**
- Maps (base de conhecimento)
- Lists (histórico)
- Funções de processamento
- Estado mutável
- Pattern matching básico

**Como executar:**
```bash
matter run examples/apps/chat_bot.matter
```

---

### 5. Data Analyzer (`data_analyzer.matter`)
**Descrição:** Ferramenta de análise de dados com estatísticas.

**Funcionalidades:**
- Calcular média, máximo, mínimo
- Somar valores
- Filtrar dados
- Análise completa de datasets
- Range e estatísticas

**Conceitos demonstrados:**
- Funções matemáticas
- Iteração sobre listas
- Algoritmos de análise
- Funções de ordem superior (filter)
- Formatação de relatórios

**Como executar:**
```bash
matter run examples/apps/data_analyzer.matter
```

---

### 6. Ride App (`ride_app.matter`)
**Descrição:** Aplicação de mobilidade urbana estilo Uber com matching de corridas.

**Funcionalidades:**
- Cadastro de motoristas
- Solicitação de corrida
- Match automático de motorista disponível
- Encerramento de corrida
- Dashboard com receita e corridas concluídas

**Conceitos demonstrados:**
- Structs (`Driver`, `Ride`)
- Lists e estado global
- Regras de negócio (tarifa por distância)
- Busca e atualização de estado em coleções
- Fluxo ponta a ponta de domínio real

**Como executar:**
```bash
matter run examples/apps/ride_app.matter
```

---

### 7. Uber Real Architecture (`uber_real/main.matter`)
**Descrição:** Arquitetura modular de mobilidade urbana (estilo Uber) em camadas.

**Funcionalidades:**
- Seed de motoristas e rider
- Matching por proximidade
- Tarifa dinâmica com surge
- Ciclo de vida completo da corrida
- Dashboard de métricas da plataforma

**Conceitos demonstrados:**
- Modularização por domínio
- Separação de responsabilidades
- Orquestração de fluxo em `main`
- Regras de negócio desacopladas
- Base pronta para evolução em produção

**Como executar:**
```bash
matter run examples/apps/uber_real/main.matter
```

---

### 8. Uber Real Prod (`uber_real_prod/main.matter`)
**Descrição:** Arquitetura de produção com API, segurança, matching, pricing, eventos e analytics.

**Funcionalidades:**
- Pipeline completo de despacho de corrida
- Regras de segurança de request
- Forecast de demanda (hook polyglot)
- Evento de corrida criada/concluída
- Métricas operacionais agregadas

**Como executar:**
```bash
matter run examples/apps/uber_real_prod/main.matter
```

**Simulador avançado:**
```bash
matter run examples/apps/uber_real_prod/simulator.matter
```

**Orquestrador operacional:**
```bash
matter run examples/apps/uber_real_prod/orchestrator_sim.matter
```

---

## 🎯 Casos de Uso

### Aplicações de Negócio
- **Counter App:** Contadores de visitantes, cliques, eventos
- **Task Manager:** Gerenciamento de projetos, TODOs, workflows
- **Data Analyzer:** Análise de vendas, métricas, KPIs

### Aplicações de Utilidade
- **Weather App:** Integração com APIs externas, dashboards
- **Chat Bot:** Atendimento automatizado, FAQs, assistentes

### Aplicações Educacionais
- Todos os exemplos servem como templates para aprendizado
- Demonstram padrões de design em Matter
- Mostram boas práticas de organização de código

## 🔧 Estrutura Comum

Todas as aplicações seguem padrões similares:

```matter
# 1. Definição de estruturas de dados
struct MyData { ... }

# 2. Estado global
let global_state = []

# 3. Funções de negócio
fn create_item() { ... }
fn list_items() { ... }
fn update_item() { ... }
fn delete_item() { ... }

# 4. Funções auxiliares
fn format_output() { ... }
fn validate_input() { ... }

# 5. Execução principal
# Código que usa as funções acima
```

## 📚 Aprendizado Progressivo

**Nível Iniciante:**
1. Counter App (mais simples)
2. Weather App (APIs e formatação)

**Nível Intermediário:**
3. Chat Bot (maps e lógica)
4. Data Analyzer (algoritmos)

**Nível Avançado:**
5. Task Manager (CRUD completo)

## 🚀 Próximos Passos

Após estudar estas aplicações, você pode:

1. **Modificar:** Adicione novas funcionalidades
2. **Combinar:** Crie apps híbridos
3. **Expandir:** Integre com backends reais
4. **Criar:** Desenvolva suas próprias aplicações

## 💡 Dicas

- Use `print` para debug
- Teste incrementalmente
- Reutilize funções entre apps
- Organize código em módulos
- Documente suas funções

## 🐛 Troubleshooting

**Erro de sintaxe:**
```bash
matter check examples/apps/your_app.matter
```

**Ver bytecode:**
```bash
matter compile examples/apps/your_app.matter -o app.mbc
matter inspect app.mbc
```

**Debug:**
- Adicione `print` statements
- Verifique tipos de dados
- Teste funções isoladamente

## 📖 Documentação

Para mais informações:
- `docs/SPEC.md` - Especificação da linguagem
- `docs/ARCHITECTURE.md` - Arquitetura do sistema
- `examples/README.md` - Outros exemplos

---

**Desenvolvido com Matter Core v0.4.0**
