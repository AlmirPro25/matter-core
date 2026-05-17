$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$outFile = Join-Path $root "env\agent_workspace_context.md"

$gitStatus = git -C $root status --short 2>$null
if (-not $gitStatus) { $gitStatus = "(clean or git unavailable)" }

$crateDirs = Get-ChildItem (Join-Path $root "crates") -Directory -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty Name

$topFiles = @("Cargo.toml", "README.md", "docs\AGENT_SPECIALIST_BOOTSTRAP.md", "env\agent_system_prompt.txt") |
    ForEach-Object {
        $p = Join-Path $root $_
        if (Test-Path $p) { $_ }
    }

$content = @()
$content += "# Agent Workspace Context"
$content += ""
$content += "Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$content += "Root: $root"
$content += ""
$content += "## Git Status"
$content += "~~~"
$content += $gitStatus
$content += "~~~"
$content += ""
$content += "## Core Files"
$content += ($topFiles | ForEach-Object { "- $_" })
$content += ""
$content += "## Crates"
if ($crateDirs) {
    $content += ($crateDirs | ForEach-Object { "- crates/$_" })
} else {
    $content += "- (none found)"
}
$content += ""
$content += "## Matter CLI Quick Commands"
$content += "- cargo run -p matter-cli -- help"
$content += "- cargo run -p matter-cli -- repl"
$content += "- cargo run -p matter-cli -- agent-chat --help"
$content += "- cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings"
$content += ""
$content += "## Specialist Focus"
$content += "- Preserve architecture and conventions"
$content += "- Prefer small validated iterations"
$content += "- Keep secrets out of versioned files"

$content -join "`r`n" | Set-Content -Path $outFile -Encoding UTF8
Write-Output "Generated: $outFile"
