param(
    [string]$RustSummary = "target\ffi\rust-smoke.json",
    [string]$NativeSummary = "target\ffi\native-smoke.json",
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

function Invoke-Verifier {
    param(
        [string]$RustPath,
        [string]$NativePath,
        [string]$MatrixFile
    )

    $stdout = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_ffi_matrix_contract_stdout_" + [guid]::NewGuid().ToString("N") + ".log")
    $stderr = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_ffi_matrix_contract_stderr_" + [guid]::NewGuid().ToString("N") + ".log")

    try {
        $process = Start-Process `
            -FilePath "powershell" `
            -ArgumentList @(
                "-ExecutionPolicy", "Bypass",
                "-File", ".\scripts\verify-ffi-smoke-summaries.ps1",
                "-RustSummary", $RustPath,
                "-NativeSummary", $NativePath,
                "-MatrixPath", $MatrixFile,
                "-CheckMatrix"
            ) `
            -Wait `
            -PassThru `
            -WindowStyle Hidden `
            -RedirectStandardOutput $stdout `
            -RedirectStandardError $stderr

        return $process.ExitCode
    }
    finally {
        foreach ($path in @($stdout, $stderr)) {
            if (Test-Path $path) {
                Remove-Item -LiteralPath $path -Force
            }
        }
    }
}

function Expect-Success {
    param(
        [string]$Name,
        [int]$ExitCode
    )

    if ($ExitCode -ne 0) {
        throw "$Name should have passed, got exit code $ExitCode"
    }
}

function Expect-Failure {
    param(
        [string]$Name,
        [int]$ExitCode
    )

    if ($ExitCode -eq 0) {
        throw "$Name should have failed"
    }
}

foreach ($path in @($RustSummary, $NativeSummary, $MatrixPath)) {
    if (-not (Test-Path $path)) {
        throw "Missing input for matrix contract test: $path"
    }
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_ffi_matrix_contract_" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force $tempDir | Out-Null

try {
    $rustCopy = Join-Path $tempDir "rust-smoke.json"
    $nativeCopy = Join-Path $tempDir "native-smoke.json"
    $validMatrix = Join-Path $tempDir "ffi-validation-matrix.json"
    $badClaimMatrix = Join-Path $tempDir "ffi-validation-matrix-bad-claim.json"
    $missingBlockerMatrix = Join-Path $tempDir "ffi-validation-matrix-missing-blocker.json"
    $missingSchemaMatrix = Join-Path $tempDir "ffi-validation-matrix-missing-schema.json"
    $badSchemaMatrix = Join-Path $tempDir "ffi-validation-matrix-bad-schema.json"

    Copy-Item $RustSummary $rustCopy
    Copy-Item $NativeSummary $nativeCopy
    Copy-Item $MatrixPath $validMatrix

    Expect-Success "valid matrix" (Invoke-Verifier $rustCopy $nativeCopy $validMatrix)

    $badClaim = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $badClaim.bridges[0].production_claim_allowed = $true
    $badClaim | ConvertTo-Json -Depth 10 | Set-Content -Path $badClaimMatrix -Encoding UTF8
    Expect-Failure "matrix with production claim allowed" (Invoke-Verifier $rustCopy $nativeCopy $badClaimMatrix)

    $missingBlocker = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $missingBlocker.bridges[0].production_blocker = ""
    $missingBlocker | ConvertTo-Json -Depth 10 | Set-Content -Path $missingBlockerMatrix -Encoding UTF8
    Expect-Failure "matrix without production blocker" (Invoke-Verifier $rustCopy $nativeCopy $missingBlockerMatrix)

    $missingSchema = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $missingSchema.PSObject.Properties.Remove('$schema')
    $missingSchema | ConvertTo-Json -Depth 10 | Set-Content -Path $missingSchemaMatrix -Encoding UTF8
    Expect-Failure "matrix without schema reference" (Invoke-Verifier $rustCopy $nativeCopy $missingSchemaMatrix)

    $badSchema = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $badSchema.PSObject.Properties['$schema'].Value = "schemas/missing-ffi-validation-matrix.schema.json"
    $badSchema | ConvertTo-Json -Depth 10 | Set-Content -Path $badSchemaMatrix -Encoding UTF8
    Expect-Failure "matrix with invalid schema reference" (Invoke-Verifier $rustCopy $nativeCopy $badSchemaMatrix)
}
finally {
    if (Test-Path $tempDir) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force
    }
}

[ordered]@{
    ok = $true
    checked = @(
        "valid matrix passes",
        "production claim allowed fails",
        "missing production blocker fails",
        "missing schema reference fails",
        "invalid schema reference fails"
    )
} | ConvertTo-Json -Depth 4
