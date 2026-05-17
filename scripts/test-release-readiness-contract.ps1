param(
    [string]$MatrixPath = "target\ffi\ffi-validation-matrix.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot
$script:LastVerifierOutput = ""

function Invoke-Readiness {
    param([string[]]$ReadinessArgs)

    $stdout = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_readiness_stdout_" + [guid]::NewGuid().ToString("N") + ".log")
    $stderr = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_readiness_stderr_" + [guid]::NewGuid().ToString("N") + ".log")

    try {
        $argumentList = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\export-release-readiness.ps1") + $ReadinessArgs
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & powershell @argumentList > $stdout 2> $stderr
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = $oldErrorActionPreference

        $script:LastVerifierOutput = @(
            if (Test-Path $stdout) { Get-Content $stdout -Raw }
            if (Test-Path $stderr) { Get-Content $stderr -Raw }
        ) -join "`n"

        return $exitCode
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
    param([string]$Name, [int]$ExitCode)
    if ($ExitCode -ne 0) {
        if ($script:LastVerifierOutput) {
            Write-Host $script:LastVerifierOutput
        }
        throw "$Name should have passed, got exit code $ExitCode"
    }
}

function Expect-Failure {
    param([string]$Name, [int]$ExitCode)
    if ($ExitCode -eq 0) {
        throw "$Name should have failed"
    }
}

if (-not (Test-Path $MatrixPath -PathType Leaf)) {
    throw "Missing input for release readiness contract test: $MatrixPath"
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_release_readiness_contract_" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force $tempDir | Out-Null

try {
    $validMatrix = Join-Path $tempDir "ffi-validation-matrix.json"
    $validOut = Join-Path $tempDir "release-readiness.json"
    $badClaimMatrix = Join-Path $tempDir "ffi-validation-matrix-bad-claim.json"
    $missingSmokeMatrix = Join-Path $tempDir "ffi-validation-matrix-missing-smoke.json"
    $missingBlockerMatrix = Join-Path $tempDir "ffi-validation-matrix-missing-blocker.json"

    Copy-Item $MatrixPath $validMatrix
    Expect-Success "valid release readiness" (Invoke-Readiness @("-MatrixPath", $validMatrix, "-Out", $validOut))

    $readiness = Get-Content $validOut -Raw | ConvertFrom-Json
    if (-not $readiness.can_publish_experimental_release) {
        throw "Valid readiness should allow experimental release"
    }
    if ($readiness.can_claim_general_production) {
        throw "Valid readiness must not allow general production claim"
    }
    if ($readiness.readiness_tier -ne "experimental_release_candidate") {
        throw "Valid readiness has unexpected tier: $($readiness.readiness_tier)"
    }
    $matrix = Get-Content $validMatrix -Raw | ConvertFrom-Json
    if ($readiness.matrix_generated_at -ne $matrix.generated_at) {
        throw "Valid readiness should include the matrix timestamp"
    }
    if ([System.IO.Path]::IsPathRooted([string]$readiness.matrix)) {
        throw "Valid readiness should not include an absolute matrix path"
    }
    if (@($readiness.required_smoke_statuses).Count -ne 3) {
        throw "Valid readiness should include required smoke statuses"
    }

    $badClaim = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $badClaim.bridges[0].production_claim_allowed = $true
    $badClaim | ConvertTo-Json -Depth 10 | Set-Content -Path $badClaimMatrix -Encoding UTF8
    Expect-Failure "readiness with production claim allowed" (Invoke-Readiness @("-MatrixPath", $badClaimMatrix))

    $missingSmoke = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    ($missingSmoke.bridges | Where-Object { $_.id -eq "go-native-cgo" }).status = "missing_smoke"
    $missingSmoke | ConvertTo-Json -Depth 10 | Set-Content -Path $missingSmokeMatrix -Encoding UTF8
    Expect-Failure "readiness with missing required smoke" (Invoke-Readiness @("-MatrixPath", $missingSmokeMatrix))

    $missingBlocker = Get-Content $MatrixPath -Raw | ConvertFrom-Json
    $missingBlocker.bridges[0].production_blocker = ""
    $missingBlocker | ConvertTo-Json -Depth 10 | Set-Content -Path $missingBlockerMatrix -Encoding UTF8
    Expect-Failure "readiness without production blocker" (Invoke-Readiness @("-MatrixPath", $missingBlockerMatrix))
}
finally {
    if (Test-Path $tempDir) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force
    }
}

[ordered]@{
    ok = $true
    checked = @(
        "valid readiness allows experimental release",
        "valid readiness blocks general production claim",
        "valid readiness records matrix timestamp",
        "valid readiness keeps relative matrix path",
        "valid readiness records required smoke statuses",
        "production claim allowance fails",
        "missing required smoke fails",
        "missing production blocker fails"
    )
} | ConvertTo-Json -Depth 4
