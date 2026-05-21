param(
    [string]$SetupPath = "dist\matter-core-beta-setup.exe",
    [string]$CertificatePath,
    [string]$TimestampUrl = "http://timestamp.digicert.com"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not (Test-Path $SetupPath -PathType Leaf)) {
    throw "Setup exe not found: $SetupPath"
}

$signtool = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
if (-not $signtool) {
    throw "signtool.exe not found. Install Windows SDK and make signtool available in PATH."
}

$args = @("sign", "/fd", "SHA256", "/tr", $TimestampUrl, "/td", "SHA256")
if ($CertificatePath) {
    if (-not (Test-Path $CertificatePath -PathType Leaf)) {
        throw "Certificate file not found: $CertificatePath"
    }
    $args += @("/f", (Resolve-Path $CertificatePath).Path)
} else {
    $args += "/a"
}
$args += (Resolve-Path $SetupPath).Path

& $signtool.Source @args
if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed with exit code $LASTEXITCODE"
}

& $signtool.Source verify /pa /v (Resolve-Path $SetupPath).Path
if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed with exit code $LASTEXITCODE"
}

[ordered]@{
    ok = $true
    setup = (Resolve-Path $SetupPath).Path
    sha256 = (Get-FileHash -LiteralPath $SetupPath -Algorithm SHA256).Hash.ToLowerInvariant()
    signed = $true
} | ConvertTo-Json -Depth 4
