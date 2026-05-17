# Resultados Reais - Matter Core

Atualizado em 2026-05-15.

## Build

Validado:

```powershell
cargo check -p matter-cli
```

Resultado: sucesso.

Observacao: o workspace usa `.cargo/config.toml` para enviar a saida de build para `F:/Users/almir/Desktop/matter_target`, evitando problemas com espacos no caminho do workspace.

## Execucao

Validado:

```powershell
cargo run -q -p matter-cli -- run examples\first_run.matter
```

Saida observada:

```text
Matter Core
language -> bytecode -> VM -> output
fib(8)
21
scores
[95, 82, 67]
classification
excellent
solid
needs-work
event: boot
runtime: event handler executed
```

## Teste Focado

Validado:

```powershell
cargo test -p matter-kernel-vm
cargo run -q -p matter-cli -- sentinel-mbc1-kernel-check-json examples\sentinel_boot.matter --budget 10000
```

Resultado: 11 testes passaram, e o comando Sentinel/MBC1 executou `examples\sentinel_boot.matter` no kernel VM subset com `fact(5) = 120` e cinco syscalls `sentinel.*` capturadas.

## Avanco Recente

`matter-kernel-vm` agora propaga `print` executado dentro de funcoes para o resultado principal e aceita operadores escalares seguros dentro de funcoes, incluindo negacao numerica, operadores booleanos, `print`, `pop` e `halt`.

## Interpretacao

O nucleo executavel existe e funciona para o caminho testado. Claims mais amplos sobre performance, FFI, compilacao nativa completa ou hardware especializado devem ser validados separadamente antes de serem tratados como entregues.

## FFI Bridge Audit

Validado em separado:

```powershell
cargo test -p matter-bridge-python
cargo test -p matter-bridge-nodejs
cargo test -p matter-bridge-nodejs-native
cargo test -p matter-bridge-rust -p matter-bridge-go -p matter-bridge-java -p matter-bridge-go-native -p matter-bridge-java-native
cargo test -p matter-bridge-nodejs -p matter-bridge-go -p matter-bridge-java
cargo test -p matter-cli rust_ffi_
cargo test -p matter-bridge-go-native --features cgo-native
cargo test -p matter-bridge-java-native --features jni-native
cargo run -q -p matter-cli -- rust-ffi-validate-args-json @examples\rust_ffi_plugin\args_add.json
cargo run -q -p matter-cli -- rust-ffi-call-json <temporary.dll> add_one @args.json
cargo build --manifest-path examples\rust_ffi_plugin\Cargo.toml
cargo run -q -p matter-cli -- rust-ffi-call-json <example_plugin.dll> add @examples\rust_ffi_plugin\args_add.json
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\rust-ffi-plugin-smoke.ps1 -Release -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\native-ffi-smoke.ps1 -IncludeJava
```

Resultado: os testes existentes passaram, mas a classificacao e mista. Python tem chamada real simples validada; Node subprocess valida chamada real para modulo built-in (`path.basename`); Go subprocess valida `math.Sqrt(2.25)`; Java subprocess valida `java.lang.String.isEmpty()`; Rust bridge valida codec ABI para todos os tipos Matter suportados, erros formais, payloads invalidos, carregamento de uma `cdylib` temporaria com `libloading`, testes de codec/arquivo no CLI, validacao de args pelo CLI, chamada E2E pelo CLI `rust-ffi-call-json`, e exemplo compilavel em `examples/rust_ffi_plugin`. O caminho Rust FFI tambem esta coberto por smoke test local, pelo script de validacao completa, pelo `test_all.ps1`, pelo CI e pelo workflow Windows de release contra o `matter-cli.exe` compilado. Node native agora carrega o addon em um host Node real e valida exports N-API mais uma chamada JSON tipada que retorna int 42; Go native agora compila uma DLL Go real e chama simbolos via `libloading`; Java native compila com `jni-native` e tem smoke JVM preparado para CI/release com JDK e opt-in local via `-IncludeJavaNativeSmoke`, mas esse runtime smoke nao foi executado localmente porque nao ha `java`/`javac` no PATH. Mesmo assim, chamadas Rust arbitrarias exigem simbolos exportados no ABI JSON do bridge, e testes native mais amplos ainda dependem de hosts, features e toolchains especificos. Ver [FFI_BRIDGE_AUDIT.md](FFI_BRIDGE_AUDIT.md) e [RUST_FFI_ABI.md](../technical/RUST_FFI_ABI.md).
