# Start Matter Language Server (stdio) without hardcoded drive letters.
# Prefers dedicated matter-lsp.exe (delivery binary). Does NOT use "matter-cli lsp"
# (language-only CLI has no lsp subcommand).
#
# Resolution order:
#   1) $env:MATTER_LSP
#   2) matter-lsp.exe / matter-lsp on PATH
#   3) same directory as matter-cli on PATH
#   4) relative package / install / dev tree bin/
#   5) $env:MATTER_HOME\bin\matter-lsp.exe
#   6) $env:LOCALAPPDATA\Matter\bin\matter-lsp.exe
#
# VS Code: set "matter.lsp.path" to matter-lsp.exe or leave empty for auto-discover.
# Do not hardcode C:/D:/F: personal paths in settings.

$ErrorActionPreference = "Stop"

function Find-MatterLsp {
    if ($env:MATTER_LSP -and (Test-Path -LiteralPath $env:MATTER_LSP)) {
        return (Resolve-Path -LiteralPath $env:MATTER_LSP).Path
    }

    foreach ($name in @("matter-lsp.exe", "matter-lsp")) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd -and $cmd.Source) { return $cmd.Source }
    }

    # Adjacent to matter-cli on PATH
    foreach ($cliName in @("matter-cli.exe", "matter-cli", "matter.exe", "matter")) {
        $cmd = Get-Command $cliName -ErrorAction SilentlyContinue
        if ($cmd -and $cmd.Source) {
            $dir = Split-Path -Parent $cmd.Source
            $cand = Join-Path $dir "matter-lsp.exe"
            if (Test-Path -LiteralPath $cand) {
                return (Resolve-Path -LiteralPath $cand).Path
            }
        }
    }

    $scriptDir = $PSScriptRoot
    $relatives = @(
        (Join-Path $scriptDir "..\bin\matter-lsp.exe"),
        (Join-Path $scriptDir "..\target\release\matter-lsp.exe"),
        (Join-Path $scriptDir "..\target\x86_64-pc-windows-gnu\release\matter-lsp.exe")
    )
    foreach ($c in $relatives) {
        if (Test-Path -LiteralPath $c) {
            return (Resolve-Path -LiteralPath $c).Path
        }
    }

    if ($env:MATTER_HOME) {
        $c = Join-Path $env:MATTER_HOME "bin\matter-lsp.exe"
        if (Test-Path -LiteralPath $c) { return (Resolve-Path -LiteralPath $c).Path }
    }

    if ($env:LOCALAPPDATA) {
        $local = Join-Path $env:LOCALAPPDATA "Matter\bin\matter-lsp.exe"
        if (Test-Path -LiteralPath $local) {
            return (Resolve-Path -LiteralPath $local).Path
        }
    }

    return $null
}

$lsp = Find-MatterLsp
if (-not $lsp) {
    [Console]::Error.WriteLine("matter-lsp.exe not found. Build: cargo build -p matter-lsp --release --bin matter-lsp")
    [Console]::Error.WriteLine("Or install Matter Core package with bin/matter-lsp.exe, or set MATTER_LSP.")
    [Console]::Error.WriteLine("Note: language-only matter-cli does not implement the 'lsp' subcommand.")
    exit 1
}

# LSP speaks over stdin/stdout — do not print banners.
& $lsp
exit $LASTEXITCODE
