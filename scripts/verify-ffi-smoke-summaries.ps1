param(
    [string]$RustSummary = "target\ffi\rust-smoke.json",
    [string]$NativeSummary = "target\ffi\native-smoke.json",
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json",
    [switch]$CheckMatrix,
    [switch]$RequireJava
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

function Read-JsonFile {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        throw "Missing summary: $Path"
    }

    return Get-Content $Path -Raw | ConvertFrom-Json
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

$rust = Read-JsonFile $RustSummary
Assert-True ($rust.ok -eq $true) "Rust FFI summary is not ok"
Assert-True ([bool]$rust.timestamp) "Rust FFI summary missing timestamp"
Assert-True ($rust.checked_capabilities -contains "rust-ffi-call-json") "Rust FFI summary missing rust-ffi-call-json capability"
Assert-True ($rust.checked_capabilities -contains "rust-ffi-validate-args-json") "Rust FFI summary missing rust-ffi-validate-args-json capability"
Assert-True ($rust.checked_symbols -contains "add") "Rust FFI summary missing add symbol"
Assert-True ($rust.checked_symbols -contains "stats") "Rust FFI summary missing stats symbol"

$native = Read-JsonFile $NativeSummary
Assert-True ($native.ok -eq $true) "Native FFI summary is not ok"
Assert-True ([bool]$native.timestamp) "Native FFI summary missing timestamp"
Assert-True ($native.checks -contains "node-native-addon") "Native FFI summary missing node-native-addon check"
Assert-True ($native.checks -contains "go-cgo-native") "Native FFI summary missing go-cgo-native check"
Assert-True ($native.node.exports -contains "matterBridgeAddIntsJson") "Native FFI summary missing Node typed JSON export"
Assert-True ([bool]$native.go.example) "Native FFI summary missing Go example path"

if ($RequireJava) {
    Assert-True ($native.checks -contains "java-jni-native") "Native FFI summary missing java-jni-native check"
    Assert-True ($native.java -eq "checked") "Native FFI summary did not check Java"
}

if ($CheckMatrix) {
    $matrixJson = Read-JsonFile $MatrixPath
    Assert-True ($matrixJson.PSObject.Properties.Name -contains "bridges") "FFI validation matrix missing bridges"
    Assert-True ($matrixJson.PSObject.Properties.Name -contains '$schema') "FFI validation matrix missing schema reference"
    $schemaRef = $matrixJson.PSObject.Properties['$schema'].Value
    Assert-True ($schemaRef -eq "schemas/ffi-validation-matrix.schema.json") "FFI validation matrix has unexpected schema reference"
    Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "FFI validation matrix schema reference must be repo-relative"
    $schemaPath = Join-Path $RepoRoot $schemaRef
    Assert-True (Test-Path $schemaPath) "FFI validation matrix schema file is missing: $schemaRef"
    $schemaJson = Read-JsonFile $schemaPath
    Assert-True ($schemaJson.PSObject.Properties.Name -contains '$id') "FFI validation schema missing id"
    Assert-True ($schemaJson.PSObject.Properties.Name -contains "properties") "FFI validation schema missing properties"
    $bridges = @($matrixJson.bridges)
    $bridgeIds = @($bridges | ForEach-Object { $_.id })

    Assert-True ($matrixJson.schema_version -eq 1) "FFI validation matrix has unexpected schema version"
    Assert-True ([bool]$matrixJson.rule) "FFI validation matrix missing rule"
    Assert-True ($bridgeIds -contains "rust-dynamic-json-abi") "FFI validation matrix missing Rust bridge"
    Assert-True ($bridgeIds -contains "node-native-napi") "FFI validation matrix missing Node native bridge"
    Assert-True ($bridgeIds -contains "go-native-cgo") "FFI validation matrix missing Go native bridge"
    Assert-True ($bridgeIds -contains "java-native-jni") "FFI validation matrix missing Java native bridge"

    foreach ($bridge in $bridges) {
        Assert-True ($bridge.production_claim_allowed -eq $false) "FFI validation matrix allows production claim for $($bridge.id)"
        Assert-True ([bool]$bridge.production_blocker) "FFI validation matrix missing production blocker for $($bridge.id)"
    }

    $rustBridge = $bridges | Where-Object { $_.id -eq "rust-dynamic-json-abi" } | Select-Object -First 1
    $nodeBridge = $bridges | Where-Object { $_.id -eq "node-native-napi" } | Select-Object -First 1
    $goBridge = $bridges | Where-Object { $_.id -eq "go-native-cgo" } | Select-Object -First 1
    $javaBridge = $bridges | Where-Object { $_.id -eq "java-native-jni" } | Select-Object -First 1

    Assert-True ($rustBridge.status -eq "validated_smoke") "Rust bridge matrix status is not validated_smoke"
    Assert-True ($nodeBridge.status -eq "validated_smoke") "Node native bridge matrix status is not validated_smoke"
    Assert-True ($goBridge.status -eq "validated_smoke") "Go native bridge matrix status is not validated_smoke"

    if ($RequireJava) {
        Assert-True ($javaBridge.status -eq "validated_smoke") "Java native bridge matrix status is not validated_smoke"
    }
    else {
        Assert-True (($javaBridge.status -eq "validated_smoke") -or ($javaBridge.status -eq "environment_required")) "Java native bridge matrix status is unexpected"
    }
}

[ordered]@{
    ok = $true
    rust_summary = $RustSummary
    native_summary = $NativeSummary
    matrix = if ($CheckMatrix) { $MatrixPath } else { $null }
    require_java = [bool]$RequireJava
} | ConvertTo-Json -Depth 4
