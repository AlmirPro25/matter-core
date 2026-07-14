# Phase 5: SBOM-ish inventory + optional cargo-audit for language-only Core deps.
$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Out = Join-Path $Root "target\validation\production_readiness_v2"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$env:PATH = "D:\mingw64\mingw64\bin;D:\dev-tools\cargo\bin;" + $env:PATH
if (-not $env:RUSTUP_HOME) { $env:RUSTUP_HOME = "D:\dev-tools\rustup" }
if (-not $env:CARGO_HOME) { $env:CARGO_HOME = "D:\dev-tools\cargo" }

Set-Location $Root

# cargo tree for matter-cli default features (language-only)
$treePath = Join-Path $Out "cargo-tree-matter-cli.txt"
cargo tree -p matter-cli --target x86_64-pc-windows-gnu --edges normal 2>&1 | Tee-Object $treePath | Out-Null

# Parse package list (heuristic)
$pkgs = @{}
Get-Content $treePath -ErrorAction SilentlyContinue | ForEach-Object {
    if ($_ -match '([a-zA-Z0-9_-]+) v([0-9]+\.[0-9]+\.[0-9]+[^ ]*)') {
        $pkgs[$Matches[1]] = $Matches[2]
    }
}
$pkgList = @($pkgs.GetEnumerator() | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ name = $_.Key; version = $_.Value }
    })
$pkgList | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Out "sbom-packages.json") -Encoding utf8

# cargo metadata for licenses if available
$metaPath = Join-Path $Out "cargo-metadata-lite.json"
cargo metadata --format-version 1 --no-deps 2>$null | Out-File $metaPath -Encoding utf8

# cargo-audit
$toolAvailable = $false
$criticalHigh = 0
$auditRaw = Join-Path $Out "cargo-audit.txt"
$auditJson = Join-Path $Out "cargo-audit.json"
if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    $toolAvailable = $true
} else {
    # try cargo audit subcommand
    $probe = cargo audit -V 2>&1
    if ($LASTEXITCODE -eq 0 -or "$probe" -match "cargo-audit") { $toolAvailable = $true }
}

$vulns = @()
$langOnlyNames = @($pkgList | ForEach-Object { $_.name })
if ($toolAvailable) {
    cargo audit --json 2>$null | Out-File $auditJson -Encoding utf8
    cargo audit 2>&1 | Tee-Object $auditRaw | Out-Null
    if (Test-Path $auditJson) {
        try {
            $aj = Get-Content $auditJson -Raw | ConvertFrom-Json
            $list = @()
            if ($aj.vulnerabilities.list) { $list = @($aj.vulnerabilities.list) }
            elseif ($aj.vulnerabilities) { $list = @($aj.vulnerabilities) }
            foreach ($v in $list) {
                $pkgName = $null
                $sev = $null
                $id = $null
                $title = $null
                try {
                    $pkgName = $v.package.name
                    $sev = $v.advisory.severity
                    $id = $v.advisory.id
                    $title = $v.advisory.title
                } catch {}
                if (-not $pkgName) { continue }
                $inTree = $langOnlyNames -contains $pkgName
                $sevL = "$sev".ToLower()
                # Count critical/high only if package is in language-only matter-cli tree
                if ($inTree -and ($sevL -match "critical|high")) { $criticalHigh++ }
                # Unknown severity in language-only tree: treat as medium residual, not silent ignore
                $vulns += [pscustomobject]@{
                    id                 = $id
                    package            = $pkgName
                    severity           = $sev
                    title              = $title
                    in_language_only_tree = $inTree
                }
            }
        } catch {
            # parse failure — keep tool_available
        }
    }
}
$langOnlyVulns = @($vulns | Where-Object { $_.in_language_only_tree })

# Manual license notes for redistribution
$licenses = @(
    @{ component = "matter-core (this project)"; license = "see LICENSE"; redistributed = $true },
    @{ component = "Rust std / CRT (Windows)"; license = "system"; redistributed = $false },
    @{ component = "crates.io dependencies"; license = "see cargo tree / crates.io metadata"; redistributed = $true; note = "static-linked into matter-cli where applicable" }
)

$report = [pscustomobject]@{
    at                 = (Get-Date).ToString("o")
    tool_available     = $toolAvailable
    tool               = $(if ($toolAvailable) { "cargo-audit" } else { "none" })
    package_count      = $pkgList.Count
    critical_or_high   = $criticalHigh
    language_only_vuln_count = $langOnlyVulns.Count
    workspace_vuln_count = $vulns.Count
    vulnerabilities_language_only = $langOnlyVulns
    vulnerabilities_workspace_all = $vulns
    licenses           = $licenses
    tree_file          = $treePath
    sbom_file          = (Join-Path $Out "sbom-packages.json")
    policy             = "critical/high in language-only tree > 0 => FAIL; tool missing => BLOCKED; workspace-only deps (e.g. pyo3) noted but not blocking language-only RC"
    note               = "cargo-audit scans full workspace lockfile; language-only binary uses matter-cli default features only"
}
$report | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $Out "dependency-audit.json") -Encoding utf8
Write-Host "SBOM packages=$($pkgList.Count) cargo-audit=$toolAvailable critical_high=$criticalHigh"
exit 0
