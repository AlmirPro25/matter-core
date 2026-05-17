param(
    [switch]$Release,
    [string]$CliPath,
    [string]$JsonOut
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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

function Invoke-CliJson {
    param(
        [string[]]$CliArgs,
        [switch]$AllowFailure
    )

    if ($script:CliExecutable) {
        $output = & $script:CliExecutable @CliArgs
    }
    else {
        $output = & cargo run -q -p matter-cli -- @CliArgs
    }
    if (($LASTEXITCODE -ne 0) -and (-not $AllowFailure)) {
        throw "matter-cli failed: $($CliArgs -join ' ')"
    }
    if (-not $output) {
        throw "matter-cli returned no JSON output: $($CliArgs -join ' ')"
    }

    return ($output -join "`n") | ConvertFrom-Json
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

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Manifest = Join-Path $RepoRoot "examples\rust_ffi_plugin\Cargo.toml"
$ProfileDir = if ($Release) { "release" } else { "debug" }
$script:CliExecutable = $null

if ($CliPath) {
    $resolvedCli = Resolve-Path $CliPath
    $script:CliExecutable = $resolvedCli.Path
    Assert-True (Test-Path $script:CliExecutable) "CLI executable was not found at $script:CliExecutable"
}

$BuildArgs = @("build", "--manifest-path", $Manifest)
$TestArgs = @("test", "--manifest-path", $Manifest)
if ($Release) {
    $BuildArgs += "--release"
}

Invoke-Checked "Build Rust FFI plugin" "cargo" $BuildArgs
Invoke-Checked "Run Rust FFI plugin ABI tests" "cargo" $TestArgs

$MetadataJson = & cargo metadata --manifest-path $Manifest --format-version 1 --no-deps
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed"
}

$TargetDir = (($MetadataJson -join "`n") | ConvertFrom-Json).target_directory
$Platform = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
if ($Platform -match "Windows") {
    $LibraryName = "matter_rust_ffi_plugin.dll"
} elseif ($Platform -match "Darwin") {
    $LibraryName = "libmatter_rust_ffi_plugin.dylib"
} else {
    $LibraryName = "libmatter_rust_ffi_plugin.so"
}

$LibraryPath = Join-Path $TargetDir (Join-Path $ProfileDir $LibraryName)
Assert-True (Test-Path $LibraryPath) "Dynamic library was not found at $LibraryPath"

$ArgsAdd = Join-Path $RepoRoot "examples\rust_ffi_plugin\args_add.json"
$ArgsDescribe = Join-Path $RepoRoot "examples\rust_ffi_plugin\args_describe.json"
$ArgsStats = Join-Path $RepoRoot "examples\rust_ffi_plugin\args_stats.json"

Write-Host "== Check CLI FFI capabilities"
$capabilities = Invoke-CliJson @("capabilities-json")
Assert-True ($capabilities.json_commands -contains "rust-ffi-call-json") "CLI capabilities missing rust-ffi-call-json in json_commands"
Assert-True ($capabilities.json_commands -contains "rust-ffi-validate-args-json") "CLI capabilities missing rust-ffi-validate-args-json in json_commands"
Assert-True ($capabilities.source_commands -contains "rust-ffi-call-json") "CLI capabilities missing rust-ffi-call-json in source_commands"
Assert-True ($capabilities.source_commands -contains "rust-ffi-validate-args-json") "CLI capabilities missing rust-ffi-validate-args-json in source_commands"

Write-Host "== Validate typed argument payload"
$validation = Invoke-CliJson @("rust-ffi-validate-args-json", "@$ArgsAdd")
Assert-True ($validation.ok -eq $true) "Argument validation did not return ok=true"
Assert-True ($validation.args_count -eq 2) "Argument validation returned unexpected args_count"

Write-Host "== Call add"
$add = Invoke-CliJson @("rust-ffi-call-json", $LibraryPath, "add", "@$ArgsAdd")
Assert-True ($add.ok -eq $true) "add did not return ok=true"
Assert-True ($add.result.type -eq "int") "add did not return an int"
Assert-True ($add.result.value -eq 42) "add returned unexpected value"

Write-Host "== Call describe"
$describe = Invoke-CliJson @("rust-ffi-call-json", $LibraryPath, "describe", "@$ArgsDescribe")
Assert-True ($describe.ok -eq $true) "describe did not return ok=true"
Assert-True ($describe.result.type -eq "string") "describe did not return a string"
Assert-True ($describe.result.value -eq "hello from Rust FFI, Matter") "describe returned unexpected value"

Write-Host "== Call stats"
$stats = Invoke-CliJson @("rust-ffi-call-json", $LibraryPath, "stats", "@$ArgsStats")
Assert-True ($stats.ok -eq $true) "stats did not return ok=true"
Assert-True ($stats.result.type -eq "map") "stats did not return a map"
Assert-True ($stats.result.value.count.value -eq 3) "stats returned unexpected count"
Assert-True ($stats.result.value.total.value -eq 42) "stats returned unexpected total"

Write-Host "== Call fail"
$fail = Invoke-CliJson @("rust-ffi-call-json", $LibraryPath, "fail", "[]") -AllowFailure
Assert-True ($fail.ok -eq $false) "fail should return ok=false"
Assert-True ($fail.stage -eq "call") "fail should report call stage"

$summary = [ordered]@{
    ok = $true
    timestamp = (Get-Date).ToString("o")
    library = $LibraryPath
    cli = if ($script:CliExecutable) { $script:CliExecutable } else { "cargo run -q -p matter-cli --" }
    checked_capabilities = @("rust-ffi-call-json", "rust-ffi-validate-args-json")
    checked_symbols = @("add", "describe", "stats", "fail")
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
