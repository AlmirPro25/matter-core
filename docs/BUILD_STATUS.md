# Matter Core Build Status

Atualizado em 2026-05-12.

## Estado Validado

O workspace compila e a suite completa passa em Windows. O repositorio agora fixa `target-dir` em `.cargo/config.toml`, fora do caminho com espacos, entao nao e mais necessario exportar `CARGO_TARGET_DIR` manualmente:

```powershell
cargo test --workspace --all-targets
```

Resultado validado: exit code `0`.

Tambem validado:

```powershell
cargo test --workspace --all-targets --no-run
```

## Diagnostico Local

A CLI agora inclui comandos de saude do workspace:

```powershell
cargo run -q -p matter-cli -- doctor
cargo run -q -p matter-cli -- doctor-json
```

Esses comandos verificam manifestos locais, configuracao segura de `target-dir`, exemplos basicos e o pipeline essencial `parse -> bytecode -> VM`.

## Criacao de Projetos

A CLI tambem inclui scaffold inicial:

```powershell
cargo run -q -p matter-cli -- init my-app
cargo run -q -p matter-cli -- init-json my-app
cargo run -q -p matter-cli -- init-json my-event-app --template event
```

O comando cria `matter.toml` e `src/main.matter`, sem sobrescrever arquivos existentes. Templates suportados: `basic` e `event`.

## Observacoes Tecnicas

- O `target-dir` externo evita falhas de toolchain causadas por caminhos com espacos.
- O backend LLVM agora tem fallback padrao sem depender de LLVM instalado; a implementacao real fica atras da feature `llvm-sys`.
- Bridges Java/Go nativos tambem usam fallback por padrao, evitando quebrar builds locais sem JDK/JNI/cgo completo.
- `matter-security` corrigiu permissao de arquivo para impedir que `FileRead` autorize `FileWrite`.
- Alguns testes cientificos/quanticos foram estabilizados para validar invariantes deterministas em vez de resultados aleatorios ou dependentes de ambiente.

## Avisos Restantes

- `matter-bridge-nodejs-native` ainda imprime avisos de N-API quando testado fora de um host Node; os testes passam mesmo assim.
- Existem muitos arquivos e diretorios nao rastreados no workspace. Eles parecem ser material de documentacao, exemplos e crates adicionados anteriormente; nao foram reorganizados automaticamente para evitar perda de contexto.
- Ainda ha warnings de `dead_code`, imports nao usados e profiles declarados em pacote nao raiz.
