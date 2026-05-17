param(
    [string]$RustSummary = "target\ffi\rust-smoke.json",
    [string]$NativeSummary = "target\ffi\native-smoke.json",
    [string]$Out = "target\ffi\ffi-validation-matrix.json"
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

function Has-Item {
    param(
        [object[]]$Items,
        [string]$Value
    )

    return ($Items -contains $Value)
}

function Convert-ToRepoRelativePath {
    param([string]$Path)

    if (-not $Path) {
        return $Path
    }

    try {
        $resolved = [System.IO.Path]::GetFullPath($Path)
        $root = [System.IO.Path]::GetFullPath($RepoRoot)
        if (-not $root.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
            $root = $root + [System.IO.Path]::DirectorySeparatorChar
        }

        if ($resolved.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $resolved.Substring($root.Length)
        }
    }
    catch {
        return $Path
    }

    return $Path
}

$rust = Read-JsonFile $RustSummary
$native = Read-JsonFile $NativeSummary

$rustCapabilities = @($rust.checked_capabilities)
$rustSymbols = @($rust.checked_symbols)
$nativeChecks = @($native.checks)
$nodeExports = @($native.node.exports)
$nodeExample = Convert-ToRepoRelativePath $native.node.example
$goExample = Convert-ToRepoRelativePath $native.go.example

$javaRuntimeChecked = ($native.java -eq "checked")

$matrix = [ordered]@{
    '$schema' = "schemas/ffi-validation-matrix.schema.json"
    schema_version = 1
    generated_at = (Get-Date).ToString("o")
    source_summaries = [ordered]@{
        rust = $RustSummary
        native = $NativeSummary
    }
    rule = "A bridge is not production-ready until it has CI integration tests against real external packages/classes/functions for its intended production surface."
    bridges = @(
        [ordered]@{
            id = "rust-dynamic-json-abi"
            crate = "matter-bridge-rust"
            status = "validated_smoke"
            evidence = @(
                "rust summary ok: $($rust.ok)",
                "capability rust-ffi-call-json: $(Has-Item $rustCapabilities 'rust-ffi-call-json')",
                "capability rust-ffi-validate-args-json: $(Has-Item $rustCapabilities 'rust-ffi-validate-args-json')",
                "symbols: $($rustSymbols -join ', ')"
            )
            production_claim_allowed = $false
            production_blocker = "Only libraries exporting the documented JSON ABI are validated; arbitrary Rust crate calls still require explicit exported symbols and broader integration tests."
        },
        [ordered]@{
            id = "node-native-napi"
            crate = "matter-bridge-nodejs-native"
            status = if (Has-Item $nativeChecks "node-native-addon") { "validated_smoke" } else { "missing_smoke" }
            evidence = @(
                "node version: $($native.node.version)",
                "example: $nodeExample",
                "exports: $($nodeExports -join ', ')",
                "typed JSON add export: $(Has-Item $nodeExports 'matterBridgeAddIntsJson')"
            )
            production_claim_allowed = $false
            production_blocker = "The addon loads and validates one typed JSON call; broader module/function conversion tests are still needed."
        },
        [ordered]@{
            id = "go-native-cgo"
            crate = "matter-bridge-go-native"
            status = if (Has-Item $nativeChecks "go-cgo-native") { "validated_smoke" } else { "missing_smoke" }
            evidence = @(
                "go version: $($native.go.version)",
                "feature: $($native.go.feature)",
                "example: $goExample"
            )
            production_claim_allowed = $false
            production_blocker = "The cgo bridge is validated against a generated/shared example library; real package/function coverage must be added case by case."
        },
        [ordered]@{
            id = "java-native-jni"
            crate = "matter-bridge-java-native"
            status = if ($javaRuntimeChecked) { "validated_smoke" } else { "environment_required" }
            evidence = @(
                "java status: $($native.java)",
                "java check present: $(Has-Item $nativeChecks 'java-jni-native')"
            )
            production_claim_allowed = $false
            production_blocker = "Runtime JNI smoke requires a JDK/JNI host and broader real class/function integration tests."
        }
    )
}

$outDir = Split-Path -Parent $Out
if ($outDir) {
    New-Item -ItemType Directory -Force $outDir | Out-Null
}

$matrix | ConvertTo-Json -Depth 10 | Set-Content -Path $Out -Encoding UTF8

$matrix | ConvertTo-Json -Depth 10
