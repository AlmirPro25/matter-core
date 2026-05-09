# Test: API Bridge JSON commands
Write-Host "=== Teste da Ponte API/JSON ===" -ForegroundColor Cyan
Write-Host ""

$cli = ".\target\release\matter-cli.exe"
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

try {
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
        Pass "capabilities-json"
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
        Assert-Equal @($json.output)[1] "9" "stdlib abs deveria funcionar"
        Assert-Equal @($json.output)[4] "10" "stdlib clamp deveria funcionar"
        Assert-Equal @($json.output)[8] "27" "stdlib cube deveria funcionar"
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
