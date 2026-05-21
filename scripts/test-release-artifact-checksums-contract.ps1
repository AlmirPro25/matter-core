Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot
$script:LastVerifierOutput = ""

function Invoke-ChecksumVerifier {
    param(
        [string]$JsonPath,
        [string]$Sha256Path
    )

    $stdout = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_checksum_contract_stdout_" + [guid]::NewGuid().ToString("N") + ".log")
    $stderr = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_checksum_contract_stderr_" + [guid]::NewGuid().ToString("N") + ".log")

    try {
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & powershell -ExecutionPolicy Bypass -File ".\scripts\verify-release-artifact-checksums.ps1" -JsonPath $JsonPath -Sha256Path $Sha256Path > $stdout 2> $stderr
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = $oldErrorActionPreference
        $script:LastVerifierOutput = @(
            if (Test-Path $stdout) { Get-Content $stdout -Raw }
            if (Test-Path $stderr) { Get-Content $stderr -Raw }
        ) -join "`n"
        return $exitCode
    }
    finally {
        if (Get-Variable -Name oldErrorActionPreference -Scope Local -ErrorAction SilentlyContinue) {
            $ErrorActionPreference = $oldErrorActionPreference
        }
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

$workRoot = Join-Path "target" ("matter_checksum_contract_" + [guid]::NewGuid().ToString("N"))

try {
    $artifactDir = Join-Path $workRoot "dist"
    New-Item -ItemType Directory -Force $artifactDir | Out-Null
    $artifactPath = Join-Path $artifactDir "matter-core-windows-x64.zip"
    Set-Content -Path $artifactPath -Value "matter artifact contract" -Encoding UTF8

    $jsonPath = Join-Path $artifactDir "release-checksums.json"
    $shaPath = Join-Path $artifactDir "SHA256SUMS.txt"
    & powershell -ExecutionPolicy Bypass -File ".\scripts\export-release-artifact-checksums.ps1" `
        -ArtifactPaths @($artifactPath) `
        -JsonOut $jsonPath `
        -Sha256Out $shaPath
    if ($LASTEXITCODE -ne 0) {
        throw "Checksum export failed with exit code $LASTEXITCODE"
    }

    Expect-Success "valid checksum files" (Invoke-ChecksumVerifier $jsonPath $shaPath)

    $badHashJson = Join-Path $artifactDir "bad-hash.json"
    $doc = Get-Content $jsonPath -Raw | ConvertFrom-Json
    $doc.artifacts[0].sha256 = ("0" * 64)
    $doc | ConvertTo-Json -Depth 8 | Set-Content -Path $badHashJson -Encoding UTF8
    Expect-Failure "checksum json with bad hash" (Invoke-ChecksumVerifier $badHashJson $shaPath)

    $badSizeJson = Join-Path $artifactDir "bad-size.json"
    $doc = Get-Content $jsonPath -Raw | ConvertFrom-Json
    $doc.artifacts[0].size_bytes = 1
    $doc | ConvertTo-Json -Depth 8 | Set-Content -Path $badSizeJson -Encoding UTF8
    Expect-Failure "checksum json with bad size" (Invoke-ChecksumVerifier $badSizeJson $shaPath)

    $absolutePathJson = Join-Path $artifactDir "absolute-path.json"
    $doc = Get-Content $jsonPath -Raw | ConvertFrom-Json
    $doc.artifacts[0].path = (Resolve-Path $artifactPath).Path
    $doc | ConvertTo-Json -Depth 8 | Set-Content -Path $absolutePathJson -Encoding UTF8
    Expect-Failure "checksum json with absolute artifact path" (Invoke-ChecksumVerifier $absolutePathJson $shaPath)

    $badShaFile = Join-Path $artifactDir "BAD-SHA256SUMS.txt"
    Set-Content -Path $badShaFile -Value "not-a-valid-sha-line" -Encoding ASCII
    Expect-Failure "invalid sha256sums line" (Invoke-ChecksumVerifier $jsonPath $badShaFile)

    $missingShaPath = Join-Path $artifactDir "MISSING-SHA256SUMS.txt"
    Set-Content -Path $missingShaPath -Value ("{0}  {1}" -f ("0" * 64), "dist\other.zip") -Encoding ASCII
    Expect-Failure "sha256sums missing artifact path" (Invoke-ChecksumVerifier $jsonPath $missingShaPath)
}
finally {
    if (Test-Path $workRoot) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}

[ordered]@{
    ok = $true
    checked = @(
        "valid checksum files pass",
        "bad json hash fails",
        "bad json size fails",
        "absolute artifact path fails",
        "invalid sha256sums line fails",
        "missing sha256sums artifact path fails"
    )
} | ConvertTo-Json -Depth 4
