param(
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json",
    [string]$Out = "target\ffi\release-readiness.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path $MatrixPath -PathType Leaf)) {
    throw "Missing FFI validation matrix: $MatrixPath"
}

function Convert-ToArtifactPath {
    param([string]$Path)

    if (-not [System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    $fileName = Split-Path -Leaf $Path
    $parent = Split-Path -Parent $Path
    $parentName = Split-Path -Leaf $parent
    $grandParent = Split-Path -Parent $parent
    $grandParentName = Split-Path -Leaf $grandParent
    if ($parentName -eq "ffi" -and $grandParentName -eq "target") {
        return "target\ffi\$fileName"
    }

    return $fileName
}

$matrix = Get-Content $MatrixPath -Raw | ConvertFrom-Json
$bridges = @($matrix.bridges)
if ($bridges.Count -eq 0) {
    throw "FFI validation matrix has no bridge entries"
}

$requiredSmokeBridgeIds = @(
    "rust-dynamic-json-abi",
    "node-native-napi",
    "go-native-cgo"
)

$missingRequiredSmoke = @()
foreach ($id in $requiredSmokeBridgeIds) {
    $bridge = $bridges | Where-Object { $_.id -eq $id } | Select-Object -First 1
    if (-not $bridge) {
        $missingRequiredSmoke += "missing bridge row: $id"
        continue
    }

    if ($bridge.status -ne "validated_smoke") {
        $missingRequiredSmoke += "$id status is $($bridge.status)"
    }
}

$productionClaimLeaks = @($bridges | Where-Object { $_.production_claim_allowed -eq $true })
if ($productionClaimLeaks.Count -gt 0) {
    throw "Release readiness refuses production claim allowance: $(@($productionClaimLeaks | ForEach-Object { $_.id }) -join ', ')"
}

$missingBlockers = @($bridges | Where-Object { -not $_.production_blocker })
if ($missingBlockers.Count -gt 0) {
    throw "Release readiness requires production blockers for: $(@($missingBlockers | ForEach-Object { $_.id }) -join ', ')"
}

$javaBridge = $bridges | Where-Object { $_.id -eq "java-native-jni" } | Select-Object -First 1
$javaRuntimeValidated = ($javaBridge -and $javaBridge.status -eq "validated_smoke")

$canPublishExperimental = ($missingRequiredSmoke.Count -eq 0)
$readinessTier = if ($canPublishExperimental) {
    "experimental_release_candidate"
}
else {
    "blocked"
}

$result = [ordered]@{
    generated_at = (Get-Date).ToString("o")
    matrix = Convert-ToArtifactPath $MatrixPath
    matrix_generated_at = $matrix.generated_at
    matrix_schema = $matrix.PSObject.Properties['$schema'].Value
    readiness_tier = $readinessTier
    can_publish_experimental_release = $canPublishExperimental
    can_claim_general_production = $false
    required_smoke_bridges = $requiredSmokeBridgeIds
    required_smoke_statuses = @($requiredSmokeBridgeIds | ForEach-Object {
        $id = $_
        $bridge = $bridges | Where-Object { $_.id -eq $id } | Select-Object -First 1
        [ordered]@{
            bridge = $id
            status = if ($bridge) { $bridge.status } else { "missing" }
        }
    })
    missing_required_smoke = $missingRequiredSmoke
    java_runtime_validated = [bool]$javaRuntimeValidated
    production_blockers = @($bridges | ForEach-Object {
        [ordered]@{
            bridge = $_.id
            blocker = $_.production_blocker
        }
    })
    release_positioning = "Experimental runtime release candidate with validated smoke paths for Rust dynamic JSON ABI, Node native N-API, and Go cgo. General production claims remain blocked."
}

$outDir = Split-Path -Parent $Out
if ($outDir) {
    New-Item -ItemType Directory -Force $outDir | Out-Null
}

$result | ConvertTo-Json -Depth 8 | Set-Content -Path $Out -Encoding UTF8

if (-not $canPublishExperimental) {
    throw "Release readiness blocked: $($missingRequiredSmoke -join '; ')"
}

$result | ConvertTo-Json -Depth 8
