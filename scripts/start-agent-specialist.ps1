param(
    [string]$Provider = "nvidia",
    [string]$Profile = "coding",
    [string]$Model = ""
)

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$envFile = Join-Path $root "env\chat.env"
$promptFile = Join-Path $root "env\agent_system_prompt.txt"

if (-not (Test-Path $envFile)) {
    Write-Error "Arquivo nao encontrado: $envFile"
    exit 1
}
if (-not (Test-Path $promptFile)) {
    Write-Error "Arquivo nao encontrado: $promptFile"
    exit 1
}

Get-Content $envFile | ForEach-Object {
    if ($_ -match '^\s*#' -or $_ -match '^\s*$') { return }
    $parts = $_ -split '=', 2
    if ($parts.Length -eq 2) {
        [Environment]::SetEnvironmentVariable($parts[0], $parts[1], 'Process')
    }
}

$systemPrompt = Get-Content $promptFile -Raw

$args = @(
    "run", "-p", "matter-cli", "--", "agent-chat",
    "--provider", $Provider,
    "--profile", $Profile,
    "--system", $systemPrompt
)

if ($Model -ne "") {
    $args += @("--model", $Model)
}

cargo @args
