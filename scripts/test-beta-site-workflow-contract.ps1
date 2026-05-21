param(
    [string]$WorkflowPath = ".github\workflows\beta-site.yml"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not (Test-Path $WorkflowPath -PathType Leaf)) {
    throw "Beta site workflow not found: $WorkflowPath"
}

$workflow = Get-Content $WorkflowPath -Raw

foreach ($requiredText in @(
    "Publish Beta Download Site",
    "workflow_dispatch:",
    "pages: write",
    "id-token: write",
    "runs-on: windows-latest",
    ".\scripts\build-download-site.ps1 -Version `"0.1.0-beta`" -Channel `"beta`"",
    ".\scripts\test-download-site-contract.ps1",
    ".\scripts\test-beta-readiness-contract.ps1",
    "actions/configure-pages@v5",
    "actions/upload-pages-artifact@v3",
    "path: site",
    "actions/deploy-pages@v4"
)) {
    if (-not $workflow.Contains($requiredText)) {
        throw "Beta site workflow missing required content: $requiredText"
    }
}

$buildIndex = $workflow.IndexOf("Build download site")
$downloadContractIndex = $workflow.IndexOf("Test download site contract")
$betaContractIndex = $workflow.IndexOf("Test beta readiness contract")
$deployIndex = $workflow.IndexOf("Deploy to GitHub Pages")

if ($buildIndex -lt 0 -or $downloadContractIndex -lt 0 -or $betaContractIndex -lt 0 -or $deployIndex -lt 0) {
    throw "Beta site workflow missing required step ordering markers"
}

if (-not ($buildIndex -lt $downloadContractIndex -and $downloadContractIndex -lt $betaContractIndex -and $betaContractIndex -lt $deployIndex)) {
    throw "Beta site workflow must build, test download site, test beta readiness, then deploy"
}

[ordered]@{
    ok = $true
    workflow = $WorkflowPath
    checked = @(
        "manual dispatch enabled",
        "pages permissions present",
        "site build runs before deploy",
        "download site contract runs before deploy",
        "beta readiness contract runs before deploy",
        "GitHub Pages artifact uses site directory"
    )
} | ConvertTo-Json -Depth 4
