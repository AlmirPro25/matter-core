param(
    [string]$CliPath = ".\target\release\matter-cli.exe"
)

# Test: API Bridge JSON commands
Write-Host "=== Teste da Ponte API/JSON ===" -ForegroundColor Cyan
Write-Host ""

$cli = $CliPath
$allPassed = $true

function Fail($message) {
    Write-Host "  FALHOU: $message" -ForegroundColor Red
    $script:allPassed = $false
}

function Pass($message) {
    Write-Host "  PASSOU: $message" -ForegroundColor Green
}

function Invoke-JsonCommand($sourceText, [string[]]$cliArgs, $expectExitCode) {
    $output = $sourceText | & $script:cli @cliArgs 2>&1 | Out-String
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne $expectExitCode) {
        Fail "exit code esperado $expectExitCode, recebido $exitCode para: matter-cli $($cliArgs -join ' ')"
        Write-Host $output
        return $null
    }

    try {
        return $output | ConvertFrom-Json
    }
    catch {
        Fail "saida nao e JSON valido para: matter-cli $($cliArgs -join ' ')"
        Write-Host $output
        return $null
    }
}

function Invoke-JsonNoInput([string[]]$cliArgs, $expectExitCode) {
    $output = & $script:cli @cliArgs 2>&1 | Out-String
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne $expectExitCode) {
        Fail "exit code esperado $expectExitCode, recebido $exitCode para: matter-cli $($cliArgs -join ' ')"
        Write-Host $output
        return $null
    }

    try {
        return $output | ConvertFrom-Json
    }
    catch {
        Fail "saida nao e JSON valido para: matter-cli $($cliArgs -join ' ')"
        Write-Host $output
        return $null
    }
}

function Assert-Equal($actual, $expected, $message) {
    if ($actual -ne $expected) {
        Fail "$message (esperado: $expected, recebido: $actual)"
    }
}

if (-not (Test-Path $cli)) {
    Fail "CLI release nao encontrada em $cli. Rode cargo build --release primeiro."
    exit 1
}

$bytecodeFile = "target\api_bridge_test.mbc"
$eventBytecodeFile = "target\api_bridge_event_test.mbc"
$projectManifestFile = "target\api_bridge_project.toml"
$projectEntryFile = "target\api_bridge_project_entry.matter"
$projectDependencyFile = "target\api_bridge_project_dependency.matter"
$projectEventManifestFile = "target\api_bridge_project_event.toml"
$projectEventEntryFile = "target\api_bridge_project_event_entry.matter"
$projectBytecodeFile = "target\api_bridge_project.mbc"
$projectBuildFile = "target\api_bridge_project_build.mbc"
$projectRunBuildFile = "target\api_bridge_project_run_build.mbc"
$projectEmitBuildFile = "target\api_bridge_project_emit_build.mbc"

$rootManifest = Get-Content "matter.toml" -Raw
$rootEntry = if ($rootManifest -match '(?m)^\s*entry\s*=\s*"([^"]+)"') { $Matches[1] } else { "" }

try {
    'import "stdlib_demo"' | Set-Content -Path $projectEntryFile -Encoding UTF8
    @'
import "std/math.matter"

print square(4)
'@ | Set-Content -Path $projectDependencyFile -Encoding UTF8

    @'
[package]
name = "api-bridge-project"
version = "0.1.0"
entry = "api_bridge_project_entry.matter"

[paths]
stdlib = "..\stdlib"
store = "api_bridge_project_store.json"

[dependencies]
stdlib_demo = "api_bridge_project_dependency.matter"
'@ | Set-Content -Path $projectManifestFile -Encoding UTF8

    @'
on boot {
    print "project event boot"
}
'@ | Set-Content -Path $projectEventEntryFile -Encoding UTF8

    @'
[package]
name = "api-bridge-event-project"
version = "0.1.0"
entry = "api_bridge_project_event_entry.matter"
'@ | Set-Content -Path $projectEventManifestFile -Encoding UTF8

    Write-Host "Descobrindo capacidades via capabilities-json..."
    $json = Invoke-JsonNoInput @("capabilities-json") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "capabilities-json deveria retornar ok=true"
        Assert-Equal $json.name "matter-cli" "capabilities-json deveria identificar a CLI"
        Assert-Equal $json.bytecode "MBC1" "capabilities-json deveria declarar bytecode MBC1"
        if (@($json.json_commands) -notcontains "run-json") {
            Fail "capabilities-json deveria listar run-json"
        }
        if (@($json.json_commands) -notcontains "eval-json") {
            Fail "capabilities-json deveria listar eval-json"
        }
        if (@($json.json_commands) -notcontains "tokens-json") {
            Fail "capabilities-json deveria listar tokens-json"
        }
        if (@($json.json_commands) -notcontains "imports-json") {
            Fail "capabilities-json deveria listar imports-json"
        }
        if (@($json.json_commands) -notcontains "package-json") {
            Fail "capabilities-json deveria listar package-json"
        }
        if (@($json.json_commands) -notcontains "project-deps-json") {
            Fail "capabilities-json deveria listar project-deps-json"
        }
        if (@($json.json_commands) -notcontains "project-verify-json") {
            Fail "capabilities-json deveria listar project-verify-json"
        }
        if (@($json.json_commands) -notcontains "project-run-json") {
            Fail "capabilities-json deveria listar project-run-json"
        }
        if (@($json.json_commands) -notcontains "project-imports-json") {
            Fail "capabilities-json deveria listar project-imports-json"
        }
        if (@($json.json_commands) -notcontains "project-lock-json") {
            Fail "capabilities-json deveria listar project-lock-json"
        }
        if (@($json.json_commands) -notcontains "project-fingerprint-json") {
            Fail "capabilities-json deveria listar project-fingerprint-json"
        }
        if (@($json.json_commands) -notcontains "project-source-json") {
            Fail "capabilities-json deveria listar project-source-json"
        }
        if (@($json.json_commands) -notcontains "project-build-json") {
            Fail "capabilities-json deveria listar project-build-json"
        }
        if (@($json.json_commands) -notcontains "project-run-build-json") {
            Fail "capabilities-json deveria listar project-run-build-json"
        }
        if (@($json.json_commands) -notcontains "project-emit-build-json") {
            Fail "capabilities-json deveria listar project-emit-build-json"
        }
        if (@($json.language_features) -notcontains "events") {
            Fail "capabilities-json deveria listar events"
        }
        if (@($json.language_features) -notcontains "imports") {
            Fail "capabilities-json deveria listar imports"
        }
        if (@($json.language_features) -notcontains "stdlib") {
            Fail "capabilities-json deveria listar stdlib"
        }
        if (@($json.language_features) -notcontains "persistence") {
            Fail "capabilities-json deveria listar persistence"
        }
        if (@($json.language_features) -notcontains "network") {
            Fail "capabilities-json deveria listar network"
        }
        if (@($json.language_features) -notcontains "concurrency") {
            Fail "capabilities-json deveria listar concurrency"
        }
        if (@($json.language_features) -notcontains "packages") {
            Fail "capabilities-json deveria listar packages"
        }
        Pass "capabilities-json"
    }

    Write-Host "Inspecionando pacote via package-json..."
    $json = Invoke-JsonNoInput @("package-json") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "package-json deveria retornar ok=true"
        Assert-Equal $json.package.name "matter-core" "package-json deveria ler nome do pacote"
        Assert-Equal $json.package.entry $rootEntry "package-json deveria ler entrada"
        Assert-Equal $json.paths.stdlib "stdlib" "package-json deveria ler caminho da stdlib"
        Assert-Equal @($json.dependencies)[0].name "math_tools" "package-json deveria listar dependencia local"
        Pass "package-json"
    }

    Write-Host "Inspecionando dependencias do projeto via project-deps-json..."
    $json = Invoke-JsonNoInput @("project-deps-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-deps-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-deps-json deveria usar nome do pacote"
        Assert-Equal $json.count 1 "project-deps-json deveria listar uma dependencia"
        Assert-Equal @($json.dependencies)[0].name "stdlib_demo" "project-deps-json deveria preservar alias"
        if ([string]::IsNullOrWhiteSpace(@($json.dependencies)[0].fingerprint)) {
            Fail "project-deps-json deveria gerar fingerprint da dependencia"
        }
        Pass "project-deps-json"
    }

    Write-Host "Validando projeto via project-check-json..."
    $json = Invoke-JsonNoInput @("project-check-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-check-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-check-json deveria usar nome do pacote"
        Assert-Equal $json.manifest $projectManifestFile "project-check-json deveria devolver manifesto"
        Pass "project-check-json"
    }

    Write-Host "Verificando projeto via project-verify-json..."
    $json = Invoke-JsonNoInput @("project-verify-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-verify-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-verify-json deveria usar nome do pacote"
        Assert-Equal $json.dependencies_count 1 "project-verify-json deveria contar dependencias"
        Assert-Equal $json.imports_count 2 "project-verify-json deveria contar imports"
        Assert-Equal $json.files_count 4 "project-verify-json deveria contar arquivos"
        if ([string]::IsNullOrWhiteSpace($json.lock_fingerprint)) {
            Fail "project-verify-json deveria gerar lock_fingerprint"
        }
        Pass "project-verify-json"
    }

    Write-Host "Executando projeto via project-run-json..."
    $json = Invoke-JsonNoInput @("project-run-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-run-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-run-json deveria usar nome do pacote"
        Assert-Equal @($json.output)[0] "16" "project-run-json deveria resolver dependencia local pelo manifesto"
        Pass "project-run-json"
    }

    Write-Host "Inspecionando imports do projeto via project-imports-json..."
    $json = Invoke-JsonNoInput @("project-imports-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-imports-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-imports-json deveria usar nome do pacote"
        Assert-Equal $json.count 2 "project-imports-json deveria encontrar alias e stdlib"
        Assert-Equal @($json.imports)[0].path "stdlib_demo" "project-imports-json deveria preservar alias declarado"
        Assert-Equal @($json.imports)[0].source "dependency" "project-imports-json deveria marcar alias como dependency"
        Assert-Equal @($json.imports)[1].source "stdlib" "project-imports-json deveria marcar import stdlib transitivo"
        Pass "project-imports-json"
    }

    Write-Host "Gerando lock do projeto via project-lock-json..."
    $json = Invoke-JsonNoInput @("project-lock-json", $projectManifestFile) 0
    $lockFingerprint = $null
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-lock-json deveria retornar ok=true"
        Assert-Equal $json.package.name "api-bridge-project" "project-lock-json deveria usar nome do pacote"
        Assert-Equal $json.imports_count 2 "project-lock-json deveria preservar grafo de imports"
        Assert-Equal @($json.dependencies)[0].name "stdlib_demo" "project-lock-json deveria listar dependencias"
        Assert-Equal $json.files_count 4 "project-lock-json deveria listar manifesto, entry, dependency e stdlib"
        $lockFingerprint = $json.lock_fingerprint
        if ([string]::IsNullOrWhiteSpace($json.lock_fingerprint)) {
            Fail "project-lock-json deveria gerar lock_fingerprint"
        }
        if ([string]::IsNullOrWhiteSpace(@($json.files)[0].fingerprint)) {
            Fail "project-lock-json deveria gerar fingerprint para arquivos"
        }
        Pass "project-lock-json"
    }

    Write-Host "Gerando fingerprint do projeto via project-fingerprint-json..."
    $json = Invoke-JsonNoInput @("project-fingerprint-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-fingerprint-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-fingerprint-json deveria usar nome do pacote"
        Assert-Equal $json.files_count 4 "project-fingerprint-json deveria listar os mesmos arquivos do lock"
        Assert-Equal $json.imports_count 2 "project-fingerprint-json deveria preservar contagem de imports"
        Assert-Equal $json.lock_fingerprint $lockFingerprint "project-fingerprint-json deveria bater com lock_fingerprint"
        Pass "project-fingerprint-json"
    }

    Write-Host "Gerando source resolvido via project-source-json..."
    $json = Invoke-JsonNoInput @("project-source-json", $projectManifestFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-source-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-source-json deveria usar nome do pacote"
        if ($json.source -notlike "*fn square*") {
            Fail "project-source-json deveria expandir imports da stdlib"
        }
        if ($json.source -like '*import "stdlib_demo"*') {
            Fail "project-source-json nao deveria manter alias importado no source final"
        }
        if ([string]::IsNullOrWhiteSpace($json.fingerprint)) {
            Fail "project-source-json deveria gerar fingerprint do source final"
        }
        Pass "project-source-json"
    }

    Write-Host "Compilando projeto via project-compile-json..."
    $json = Invoke-JsonNoInput @("project-compile-json", $projectManifestFile, "-o", $projectBytecodeFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-compile-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-compile-json deveria usar nome do pacote"
        Assert-Equal $json.output $projectBytecodeFile "project-compile-json deveria devolver arquivo de saida"
        if (-not (Test-Path $projectBytecodeFile)) {
            Fail "project-compile-json nao criou $projectBytecodeFile"
        }
        else {
            Pass "project-compile-json"
        }
    }

    Write-Host "Construindo projeto via project-build-json..."
    $json = Invoke-JsonNoInput @("project-build-json", $projectManifestFile, "-o", $projectBuildFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-build-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-build-json deveria usar nome do pacote"
        Assert-Equal $json.output $projectBuildFile "project-build-json deveria devolver output"
        Assert-Equal $json.files_count 4 "project-build-json deveria preservar contagem de arquivos"
        Assert-Equal $json.imports_count 2 "project-build-json deveria preservar contagem de imports"
        if ([string]::IsNullOrWhiteSpace($json.bytecode_fingerprint)) {
            Fail "project-build-json deveria gerar bytecode_fingerprint"
        }
        if (-not (Test-Path $projectBuildFile)) {
            Fail "project-build-json nao criou $projectBuildFile"
        }
        else {
            Pass "project-build-json"
        }
    }

    Write-Host "Construindo e executando projeto via project-run-build-json..."
    $json = Invoke-JsonNoInput @("project-run-build-json", $projectManifestFile, "-o", $projectRunBuildFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-run-build-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-project" "project-run-build-json deveria usar nome do pacote"
        Assert-Equal $json.output_file $projectRunBuildFile "project-run-build-json deveria devolver output_file"
        Assert-Equal @($json.output)[0] "16" "project-run-build-json deveria executar bytecode do projeto"
        if ([string]::IsNullOrWhiteSpace($json.lock_fingerprint)) {
            Fail "project-run-build-json deveria gerar lock_fingerprint"
        }
        if (-not (Test-Path $projectRunBuildFile)) {
            Fail "project-run-build-json nao criou $projectRunBuildFile"
        }
        else {
            Pass "project-run-build-json"
        }
    }

    Write-Host "Construindo projeto e emitindo evento via project-emit-build-json..."
    $json = Invoke-JsonNoInput @("project-emit-build-json", $projectEventManifestFile, "boot", "-o", $projectEmitBuildFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "project-emit-build-json deveria retornar ok=true"
        Assert-Equal $json.package "api-bridge-event-project" "project-emit-build-json deveria usar nome do pacote"
        Assert-Equal $json.event "boot" "project-emit-build-json deveria devolver evento"
        Assert-Equal $json.output_file $projectEmitBuildFile "project-emit-build-json deveria devolver output_file"
        Assert-Equal @($json.output)[0] "project event boot" "project-emit-build-json deveria capturar output do evento"
        if ([string]::IsNullOrWhiteSpace($json.bytecode_fingerprint)) {
            Fail "project-emit-build-json deveria gerar bytecode_fingerprint"
        }
        if (-not (Test-Path $projectEmitBuildFile)) {
            Fail "project-emit-build-json nao criou $projectEmitBuildFile"
        }
        else {
            Pass "project-emit-build-json"
        }
    }

    Write-Host "Executando snippet via eval-json..."
    $json = Invoke-JsonNoInput @("eval-json", "print 40 + 2") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "eval-json deveria retornar ok=true"
        Assert-Equal $json.input "<eval>" "eval-json deveria marcar input eval"
        Assert-Equal @($json.output)[0] "42" "eval-json deveria capturar output"
        Pass "eval-json"
    }

    Write-Host "Inspecionando tokens via tokens-json..."
    $json = Invoke-JsonCommand "let x = 42" @("tokens-json", "-") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "tokens-json deveria retornar ok=true"
        Assert-Equal @($json.tokens)[0].kind "let" "primeiro token deveria ser let"
        Assert-Equal @($json.tokens)[1].kind "ident" "segundo token deveria ser ident"
        Assert-Equal @($json.tokens)[1].value "x" "ident deveria capturar valor x"
        Assert-Equal @($json.tokens)[3].kind "int" "quarto token deveria ser int"
        Assert-Equal @($json.tokens)[3].value "42" "int deveria capturar valor 42"
        Pass "tokens-json"
    }

    Write-Host "Inspecionando imports via imports-json..."
    $json = Invoke-JsonNoInput @("imports-json", "examples\test_imports.matter") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "imports-json deveria retornar ok=true"
        Assert-Equal $json.count 1 "imports-json deveria encontrar um import"
        Assert-Equal @($json.imports)[0].path "modules/math_tools.matter" "imports-json deveria preservar caminho declarado"
        Pass "imports-json"
    }

    Write-Host "Executando arquivo com import via run-json..."
    $json = Invoke-JsonNoInput @("run-json", "examples\test_imports.matter") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-json com import deveria retornar ok=true"
        Assert-Equal @($json.output)[1] "42" "import deveria disponibilizar funcao dobro"
        Assert-Equal @($json.output)[2] "6" "import deveria disponibilizar funcao soma_tres"
        Pass "run-json imports"
    }

    Write-Host "Executando arquivo com stdlib via run-json..."
    $json = Invoke-JsonNoInput @("run-json", "examples\test_stdlib.matter") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-json com stdlib deveria retornar ok=true"
        Assert-Equal @($json.output)[1] "10" "stdlib math.abs deveria funcionar"
        Assert-Equal @($json.output)[4] "256" "stdlib math.pow deveria funcionar"
        Assert-Equal @($json.output)[7] "HELLO WORLD" "stdlib string.upper deveria funcionar"
        Assert-Equal @($json.output)[17] "31" "stdlib list.sum deveria funcionar"
        Pass "run-json stdlib"
    }

    Write-Host "Executando arquivo com store persistente via run-json..."
    $env:MATTER_STORE_PATH = "target\api_bridge_store.json"
    if (Test-Path $env:MATTER_STORE_PATH) {
        Remove-Item -LiteralPath $env:MATTER_STORE_PATH -ErrorAction SilentlyContinue
    }
    $json = Invoke-JsonNoInput @("run-json", "examples\test_store.matter") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-json com store deveria retornar ok=true"
        Assert-Equal @($json.output)[1] "false" "store.has deveria iniciar falso"
        Assert-Equal @($json.output)[2] "41" "store.get deveria recuperar valor salvo"
        Assert-Equal @($json.output)[3] "42" "store.set deveria persistir valor atualizado"
        Assert-Equal @($json.output)[5] "true" "store.delete deveria informar remocao"
        Assert-Equal @($json.output)[6] "false" "store.has deveria refletir delete"
        Pass "run-json store"
    }
    Remove-Item -LiteralPath $env:MATTER_STORE_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:\MATTER_STORE_PATH -ErrorAction SilentlyContinue

    Write-Host "Executando fila spawn via run-json..."
    $json = Invoke-JsonNoInput @("run-json", "examples\test_spawn.matter") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-json com spawn deveria retornar ok=true"
        Assert-Equal @($json.output)[0] "main" "spawn deveria continuar main antes de drenar evento"
        Assert-Equal @($json.output)[1] "boot" "spawn deveria executar evento enfileirado"
        Assert-Equal @($json.output)[2] "after boot" "evento deveria continuar apos spawn interno"
        Assert-Equal @($json.output)[3] "tick" "spawn dentro de evento deveria enfileirar novo evento"
        Pass "run-json spawn"
    }

    Write-Host "Validando source via check-json..."
    $json = Invoke-JsonCommand "print 42" @("check-json", "-") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "check-json deveria retornar ok=true"
        Assert-Equal $json.input "<stdin>" "check-json deveria marcar input stdin"
        Assert-Equal $json.summary.constants 1 "check-json deveria contar constantes"
        Pass "check-json"
    }

    Write-Host "Executando source via run-json..."
    $json = Invoke-JsonCommand "print 1 print 2" @("run-json", "-") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-json deveria retornar ok=true"
        Assert-Equal @($json.output).Count 2 "run-json deveria capturar duas linhas"
        Assert-Equal @($json.output)[0] "1" "primeira linha capturada"
        Assert-Equal @($json.output)[1] "2" "segunda linha capturada"
        Pass "run-json"
    }

    Write-Host "Validando erro de runtime via run-json..."
    $json = Invoke-JsonCommand "print 1 print 10 / 0" @("run-json", "-") 1
    if ($null -ne $json) {
        Assert-Equal $json.ok $false "run-json com erro deveria retornar ok=false"
        Assert-Equal $json.stage "runtime" "run-json deveria marcar stage runtime"
        Assert-Equal @($json.output)[0] "1" "run-json deveria preservar output parcial"
        Pass "run-json runtime error"
    }

    Write-Host "Emitindo evento source via emit-json..."
    $json = Invoke-JsonCommand 'on boot { print "ok" }' @("emit-json", "-", "boot") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "emit-json deveria retornar ok=true"
        Assert-Equal $json.event "boot" "emit-json deveria devolver nome do evento"
        Assert-Equal @($json.output)[0] "ok" "emit-json deveria capturar print do evento"
        Pass "emit-json"
    }

    Write-Host "Compilando source via compile-json..."
    $json = Invoke-JsonCommand "print 99" @("compile-json", "-", "-o", $bytecodeFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "compile-json deveria retornar ok=true"
        Assert-Equal $json.output $bytecodeFile "compile-json deveria devolver arquivo de saida"
        if (-not (Test-Path $bytecodeFile)) {
            Fail "compile-json nao criou $bytecodeFile"
        }
        else {
            Pass "compile-json"
        }
    }

    Write-Host "Inspecionando bytecode via inspect-json..."
    $json = Invoke-JsonNoInput @("inspect-json", $bytecodeFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "inspect-json deveria retornar ok=true"
        Assert-Equal $json.magic "MBC1" "inspect-json deveria ler magic MBC1"
        Assert-Equal $json.summary.constants 1 "inspect-json deveria contar constantes"
        Pass "inspect-json"
    }

    Write-Host "Executando bytecode via run-bytecode-json..."
    $json = Invoke-JsonNoInput @("run-bytecode-json", $bytecodeFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "run-bytecode-json deveria retornar ok=true"
        Assert-Equal @($json.output)[0] "99" "run-bytecode-json deveria capturar print"
        Pass "run-bytecode-json"
    }

    Write-Host "Compilando evento e emitindo via emit-bytecode-json..."
    $json = Invoke-JsonCommand 'on boot { print "bytecode boot" }' @("compile-json", "-", "-o", $eventBytecodeFile) 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "compile-json de evento deveria retornar ok=true"
    }

    $json = Invoke-JsonNoInput @("emit-bytecode-json", $eventBytecodeFile, "boot") 0
    if ($null -ne $json) {
        Assert-Equal $json.ok $true "emit-bytecode-json deveria retornar ok=true"
        Assert-Equal $json.event "boot" "emit-bytecode-json deveria devolver evento"
        Assert-Equal @($json.output)[0] "bytecode boot" "emit-bytecode-json deveria capturar print"
        Pass "emit-bytecode-json"
    }

    Write-Host "Validando erro de load estruturado..."
    $json = Invoke-JsonNoInput @("run-bytecode-json", "missing_api_bridge_file.mbc") 1
    if ($null -ne $json) {
        Assert-Equal $json.ok $false "erro de load deveria retornar ok=false"
        Assert-Equal $json.stage "load" "erro de load deveria marcar stage load"
        Pass "load error JSON"
    }
}
finally {
    Remove-Item -LiteralPath $bytecodeFile -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $eventBytecodeFile -ErrorAction SilentlyContinue
}

Write-Host ""
if ($allPassed) {
    Write-Host "=== Ponte API/JSON passou! ===" -ForegroundColor Green
    exit 0
}
else {
    Write-Host "=== Ponte API/JSON falhou ===" -ForegroundColor Red
    exit 1
}
