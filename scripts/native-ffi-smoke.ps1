param(
    [switch]$IncludeJava,
    [string]$JsonOut
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

function Invoke-Checked {
    param(
        [string]$Name,
        [string]$Exe,
        [string[]]$CommandArgs
    )

    Write-Host "== $Name"
    & $Exe @CommandArgs
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

Write-Host "== Check Node host"
$nodeVersion = & node --version
if ($LASTEXITCODE -ne 0) {
    throw "Node.js is required for Node native FFI smoke"
}

Write-Host "== Check Go host"
$goVersion = & go version
if ($LASTEXITCODE -ne 0) {
    throw "Go is required for Go native FFI smoke"
}

Write-Host "== Check native FFI examples"
$NodeSmoke = Join-Path $RepoRoot "examples\node_native_host\smoke.js"
$GoPlugin = Join-Path $RepoRoot "examples\go_native_plugin\plugin.go"
Assert-True (Test-Path $NodeSmoke) "Node native smoke script was not found at $NodeSmoke"
Assert-True (Test-Path $GoPlugin) "Go native plugin example was not found at $GoPlugin"

Invoke-Checked "Build Node native bridge" "cargo" @("build", "-p", "matter-bridge-nodejs-native")

$TargetDir = if ($env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR
} else {
    (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory
}

$NodeDll = Join-Path $TargetDir "debug\matter_bridge_nodejs_native.dll"
Assert-True (Test-Path $NodeDll) "Node native bridge DLL was not found at $NodeDll"

$NodeSmokeDir = Join-Path $env:TEMP ("matter_node_native_smoke_" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force $NodeSmokeDir | Out-Null
$NodeAddon = Join-Path $NodeSmokeDir "matter_bridge_nodejs_native.node"
Copy-Item $NodeDll $NodeAddon -Force

Write-Host "== Load Node native bridge through Node.js"
$nodeOutput = & node $NodeSmoke $NodeAddon
if ($LASTEXITCODE -ne 0) {
    throw "Node native addon smoke failed"
}
$nodeJson = ($nodeOutput -join "`n") | ConvertFrom-Json
Assert-True ($nodeJson.keys -contains "matterBridgeInit") "Node native addon did not expose matterBridgeInit"
Assert-True ($nodeJson.keys -contains "matterBridgeVersion") "Node native addon did not expose matterBridgeVersion"
Assert-True ($nodeJson.keys -contains "matterBridgeAddIntsJson") "Node native addon did not expose matterBridgeAddIntsJson"
Assert-True ($nodeJson.added.value -eq 42) "Node native addon typed JSON call returned unexpected value"

Invoke-Checked "Run Go native bridge feature tests" "cargo" @("test", "-p", "matter-bridge-go-native", "--features", "cgo-native")

$javaStatus = "skipped"
if ($IncludeJava) {
    Write-Host "== Check Java host"
    $java = Get-Command java -ErrorAction SilentlyContinue
    $javac = Get-Command javac -ErrorAction SilentlyContinue
    if (-not $java -or -not $javac) {
        throw "Java native FFI smoke requires java and javac on PATH"
    }
    if ($env:JAVA_HOME) {
        $javaBin = Join-Path $env:JAVA_HOME "bin"
        $javaServer = Join-Path $javaBin "server"
        if (Test-Path $javaServer) {
            $env:PATH = "$javaServer;$env:PATH"
        }
        if (Test-Path $javaBin) {
            $env:PATH = "$javaBin;$env:PATH"
        }
        $env:LIB = if ($env:LIB) { "$javaServer;$env:LIB" } else { $javaServer }
    }
    Invoke-Checked "Run Java native bridge feature tests" "cargo" @("test", "-p", "matter-bridge-java-native", "--features", "jni-native")
    Invoke-Checked "Run Java native JVM runtime smoke" "cargo" @("test", "-p", "matter-bridge-java-native", "--features", "jni-native", "calls_real_jvm_static_string", "--", "--ignored")
    $javaStatus = "checked"
}

$summary = [ordered]@{
    ok = $true
    timestamp = (Get-Date).ToString("o")
    checks = @("node-native-addon", "go-cgo-native")
    node = @{
        version = $nodeVersion
        addon = $NodeAddon
        exports = $nodeJson.keys
        example = $NodeSmoke
    }
    go = @{
        version = $goVersion
        feature = "cgo-native"
        example = $GoPlugin
    }
    java = $javaStatus
}

if ($IncludeJava) {
    $summary.checks += "java-jni-native"
}

$summaryJson = $summary | ConvertTo-Json -Depth 5
if ($JsonOut) {
    $jsonPath = Join-Path $RepoRoot $JsonOut
    $jsonParent = Split-Path -Parent $jsonPath
    if ($jsonParent) {
        New-Item -ItemType Directory -Force $jsonParent | Out-Null
    }
    Set-Content -Path $jsonPath -Value $summaryJson -Encoding UTF8
}

$summaryJson
