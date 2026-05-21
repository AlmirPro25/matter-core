param(
    [string]$SiteRoot = "site",
    [string]$TemplatePath = ".github\ISSUE_TEMPLATE\beta_feedback.yml"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

foreach ($required in @(
    $TemplatePath,
    (Join-Path $SiteRoot "TESTER_GUIDE.md"),
    (Join-Path $SiteRoot "BETA_NOTES.md"),
    (Join-Path $SiteRoot "index.html")
)) {
    if (-not (Test-Path $required -PathType Leaf)) {
        throw "Beta feedback contract missing required file: $required"
    }
}

$template = Get-Content $TemplatePath -Raw
foreach ($requiredText in @(
    "name: Beta feedback",
    "labels: [`"beta`", `"feedback`"]",
    "Windows version",
    "PowerShell version",
    "Diagnosis output",
    "Uninstall failed",
    "matter capabilities-json"
)) {
    if (-not $template.Contains($requiredText)) {
        throw "Beta feedback template missing required content: $requiredText"
    }
}

$guide = Get-Content (Join-Path $SiteRoot "TESTER_GUIDE.md") -Raw
$guideLower = $guide.ToLowerInvariant()
foreach ($requiredText in @(
    "Rust is not required",
    "download these files",
    "install-matter-beta.cmd",
    "Install",
    "First Run",
    "Diagnose",
    "Uninstall",
    "Beta feedback"
)) {
    if (-not $guideLower.Contains($requiredText.ToLowerInvariant())) {
        throw "Beta tester guide missing required content: $requiredText"
    }
}

$notes = Get-Content (Join-Path $SiteRoot "BETA_NOTES.md") -Raw
if (-not $notes.Contains("Beta feedback")) {
    throw "Beta notes must point testers to the Beta feedback template"
}

$html = Get-Content (Join-Path $SiteRoot "index.html") -Raw
if (-not $html.Contains("TESTER_GUIDE.md")) {
    throw "Download site must link to TESTER_GUIDE.md"
}

[ordered]@{
    ok = $true
    checked = @(
        "beta feedback issue template exists",
        "tester guide explains install first-run diagnose uninstall",
        "beta notes point to feedback",
        "site links tester guide"
    )
} | ConvertTo-Json -Depth 4
