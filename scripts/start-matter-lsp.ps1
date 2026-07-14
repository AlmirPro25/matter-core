# Start Matter Language Server (stdio) without hardcoded drive letters.
# Resolution order:
#   1) $env:MATTER_CLI
#   2) matter-cli.exe / matter.exe on PATH
#   3) relative to this script (dev tree or installed package)
#   4) $env:MATTER_HOME\bin\matter-cli.exe
#   5) $env:LOCALAPPDATA\Matter\bin\matter-cli.exe
#
# VS Code: set "matter.lsp.path" to matter-cli (on PATH) or absolute path you choose.
# Do not hardcode C:/D:/F: in settings for portability.

$ErrorActionPreference = "Stop"

function Find-MatterCli {
    if ($env:MATTER_CLI -and (Test-Path -LiteralPath $env:MATTER_CLI)) {
        return (Resolve-Path -LiteralPath $env:MATTER_CLI).Path
    }

    foreach ($name in @("matter-cli.exe", "matter-cli", "matter.exe", "matter")) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd -and $cmd.Source) { return $cmd.Source }
    }

    $scriptDir = $PSScriptRoot
    $relatives = @(
        (Join-Path $scriptDir "..\bin\matter-cli.exe"),
        (Join-Path $scriptDir "..\bin\matter.exe"),
        (Join-Path $scriptDir "..\target\x86_64-pc-windows-gnu\release\matter-cli.exe"),
        (Join-Path $scriptDir "..\target\release\matter-cli.exe")
    )
    foreach ($c in $relatives) {
        if (Test-Path -LiteralPath $c) {
            return (Resolve-Path -LiteralPath $c).Path
        }
    }

    if ($env:MATTER_HOME) {
        $c = Join-Path $env:MATTER_HOME "bin\matter-cli.exe"
        if (Test-Path -LiteralPath $c) { return (Resolve-Path -LiteralPath $c).Path }
    }

    $local = Join-Path $env:LOCALAPPDATA "Matter\bin\matter-cli.exe"
    if (Test-Path -LiteralPath $local) {
        return (Resolve-Path -LiteralPath $local).Path
    }

    return $null
}

$cli = Find-MatterCli
if (-not $cli) {
    [Console]::Error.WriteLine("matter-cli.exe not found. Install Matter Core or set MATTER_CLI / PATH / MATTER_HOME.")
    exit 1
}

# LSP speaks over stdin/stdout — do not print banners.
& $cli lsp
exit $LASTEXITCODE
