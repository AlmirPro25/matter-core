param(
    [string]$CliPath = "",
    [switch]$SkipRun,
    [switch]$JsonSummary
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$examples = @(
    "examples\first_run.matter",
    "examples\language_tour.matter",
    "examples\fibonacci.matter",
    "examples\hello.matter",
    "examples\simple.matter",
    "examples\test_functions.matter",
    "examples\test_recursion.matter",
    "examples\test_loops.matter",
    "examples\test_lists.matter",
    "examples\test_maps.matter",
    "examples\test_structs.matter"
)

function Invoke-MatterJson {
    param([string[]]$Arguments)

    $output = if ($CliPath) {
        & $CliPath @Arguments 2>&1
    }
    else {
        & cargo run -q -p matter-cli -- @Arguments 2>&1
    }
    $text = ($output | Out-String).Trim()
    $start = $text.IndexOf("{")
    if ($start -gt 0) {
        $text = $text.Substring($start)
    }

    return [PSCustomObject]@{
        ExitCode = $LASTEXITCODE
        Text     = $text
    }
}

function Test-MatterJsonOk {
    param(
        [string]$Example,
        [string]$Command,
        [string[]]$Arguments
    )

    $result = Invoke-MatterJson -Arguments $Arguments
    $parsed = $null
    $ok = $false
    $details = ""

    if ($result.ExitCode -eq 0) {
        try {
            $parsed = $result.Text | ConvertFrom-Json
            $ok = [bool]$parsed.ok
            if (-not $ok -and $parsed.status) {
                $ok = $parsed.status -eq "pass" -or $parsed.status -eq "warn"
            }
            $details = if ($parsed.status) { "status=$($parsed.status)" } else { "ok=$($parsed.ok)" }
        }
        catch {
            $details = "invalid JSON: $($_.Exception.Message)"
        }
    }
    else {
        $details = "exit=$($result.ExitCode): $($result.Text)"
    }

    return [PSCustomObject]@{
        example = $Example
        command = $Command
        passed  = $ok
        details = $details
    }
}

$results = New-Object System.Collections.Generic.List[Object]

foreach ($example in $examples) {
    if (-not (Test-Path $example)) {
        $results.Add([PSCustomObject]@{
                example = $example
                command = "exists"
                passed  = $false
                details = "missing file"
            })
        continue
    }

    $results.Add((Test-MatterJsonOk -Example $example -Command "check-json" -Arguments @("check-json", $example)))
    $results.Add((Test-MatterJsonOk -Example $example -Command "perf-diagnose-json" -Arguments @("perf-diagnose-json", $example)))

    if (-not $SkipRun) {
        $results.Add((Test-MatterJsonOk -Example $example -Command "run-json" -Arguments @("run-json", $example)))
    }
}

$failed = @($results | Where-Object { -not $_.passed })
$summary = [PSCustomObject]@{
    ok       = $failed.Count -eq 0
    checked  = $results.Count
    examples = $examples.Count
    failed   = $failed.Count
    results  = $results
}

if ($JsonSummary) {
    $summary | ConvertTo-Json -Depth 6
}
else {
    foreach ($result in $results) {
        $status = if ($result.passed) { "PASS" } else { "FAIL" }
        $color = if ($result.passed) { "Green" } else { "Red" }
        Write-Host ("[{0}] {1} {2} - {3}" -f $status, $result.example, $result.command, $result.details) -ForegroundColor $color
    }
}

if ($failed.Count -gt 0) {
    exit 1
}
exit 0
