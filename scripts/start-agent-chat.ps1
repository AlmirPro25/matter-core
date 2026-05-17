param(
    [string]$Profile = "coding",
    [string]$Model = ""
)

$envFile = Join-Path $PSScriptRoot "..\env\chat.env"
if (-not (Test-Path $envFile)) {
    Write-Error "Arquivo nao encontrado: $envFile"
    exit 1
}

Get-Content $envFile | ForEach-Object {
    if ($_ -match '^\s*#' -or $_ -match '^\s*$') { return }
    $parts = $_ -split '=', 2
    if ($parts.Length -eq 2) {
        [Environment]::SetEnvironmentVariable($parts[0], $parts[1], 'Process')
    }
}

$args = @("run", "-p", "matter-cli", "--", "agent-chat", "--provider", "nvidia", "--profile", $Profile)
if ($Model -ne "") {
    $args += @("--model", $Model)
}

cargo @args
