param(
    [string]$CliPath,
    [string]$QualityPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

if ($QualityPath) {
    Assert-True (Test-Path $QualityPath -PathType Leaf) "Frontier simulation quality file not found: $QualityPath"
    $raw = Get-Content -Path $QualityPath -Raw
}
elseif ($CliPath) {
    Assert-True (Test-Path $CliPath -PathType Leaf) "CLI not found: $CliPath"
    $stderrPath = [System.IO.Path]::GetTempFileName()
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $CliPath frontier-sim-quality-json 2> $stderrPath
    $ErrorActionPreference = $previousErrorActionPreference
    $stderr = Get-Content -Path $stderrPath -Raw -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $stderrPath -Force -ErrorAction SilentlyContinue
    if ($LASTEXITCODE -ne 0) {
        throw "frontier-sim-quality-json failed: $stderr $output"
    }
    $raw = ($output -join "`n")
}
else {
    $stderrPath = [System.IO.Path]::GetTempFileName()
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & cargo run -q -p matter-cli -- frontier-sim-quality-json 2> $stderrPath
    $ErrorActionPreference = $previousErrorActionPreference
    $stderr = Get-Content -Path $stderrPath -Raw -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $stderrPath -Force -ErrorAction SilentlyContinue
    if ($LASTEXITCODE -ne 0) {
        throw "frontier-sim-quality-json failed: $stderr $output"
    }
    $raw = ($output -join "`n")
}

$payload = $raw | ConvertFrom-Json
$schemaRef = $payload.PSObject.Properties['$schema'].Value

Assert-True ($schemaRef -eq "schemas/frontier-simulation-quality.schema.json") "Unexpected frontier simulation quality schema reference"
Assert-True (-not [System.IO.Path]::IsPathRooted($schemaRef)) "Schema reference must be repo-relative"
Assert-True (Test-Path (Join-Path $repoRoot $schemaRef) -PathType Leaf) "Schema file is missing"
Assert-True ($payload.schema_version -eq 1) "Unexpected schema version"
Assert-True ($payload.kind -eq "frontier_simulation_quality") "Unexpected kind"
Assert-True ($payload.ok -eq $true) "Quality payload must report ok=true"
Assert-True ($payload.summary.all_simulated -eq $true) "Quality payload must report all_simulated=true"
Assert-True ($payload.summary.any_hardware -eq $false) "Quality payload must report any_hardware=false"
Assert-True ($payload.summary.quality_level -ge 1) "Quality level must be >= 1"

$implemented = @($payload.summary.implemented_modules)
Assert-True ($implemented -contains "quantum") "Quantum must be implemented in quality contract"
Assert-True ($implemented -contains "neuromorphic") "Neuromorphic must be implemented in quality contract"
Assert-True ($implemented -contains "photonic") "Photonic must be implemented in quality contract"
Assert-True ($implemented -contains "wetware") "Wetware must be implemented in quality contract"

$bell = $payload.evidence.quantum.bell_stats
Assert-True ($null -ne $bell) "Missing quantum bell_stats evidence"
Assert-True ($bell.shots -eq 1000) "Bell stats must use 1000 shots"
Assert-True ($bell.seed -eq 42) "Bell stats must use seed 42"
Assert-True ($bell.passed -eq $true) "Bell stats must pass"
Assert-True ($bell.correlated_rate -ge 0.95) "Bell correlated rate is too low"
Assert-True ($bell.forbidden_rate -le 0.05) "Bell forbidden rate is too high"
Assert-True ($bell.balance_error -le 0.10) "Bell balance error is too high"

$quantumChecks = @($payload.checks.quantum)
Assert-True ($quantumChecks.Count -ge 1) "Missing quantum checks"
Assert-True ($quantumChecks[0].name -eq "bell_distribution") "Missing bell_distribution check"
Assert-True ($quantumChecks[0].passed -eq $true) "bell_distribution check must pass"

$lif = $payload.evidence.neuromorphic.lif_threshold_probe
Assert-True ($null -ne $lif) "Missing neuromorphic lif_threshold_probe evidence"
Assert-True ($lif.input_current -eq 20) "LIF probe must use input_current=20"
Assert-True ($lif.steps -eq 200) "LIF probe must use 200 steps"
Assert-True ($lif.spiked -eq $true) "LIF probe must spike under strong current"
Assert-True ($lif.spike_count -gt 0) "LIF probe must report at least one spike"
Assert-True ($lif.latency_ms -ge 0) "LIF probe must report non-negative latency"
Assert-True ($lif.passed -eq $true) "LIF threshold probe must pass"

$neuromorphicChecks = @($payload.checks.neuromorphic)
Assert-True ($neuromorphicChecks.Count -ge 1) "Missing neuromorphic checks"
Assert-True ($neuromorphicChecks[0].name -eq "lif_threshold_response") "Missing lif_threshold_response check"
Assert-True ($neuromorphicChecks[0].passed -eq $true) "lif_threshold_response check must pass"

$truthTable = $payload.evidence.photonic.truth_table
Assert-True ($null -ne $truthTable) "Missing photonic truth_table evidence"
Assert-True ($truthTable.row_count -eq 4) "Photonic truth table must contain four rows"
Assert-True ($truthTable.truth_table_accuracy -eq 1) "Photonic truth table must be exact"
Assert-True ($truthTable.passed -eq $true) "Photonic truth table must pass"

$waveguideLoss = $payload.evidence.photonic.waveguide_loss
Assert-True ($null -ne $waveguideLoss) "Missing photonic waveguide_loss evidence"
Assert-True ($waveguideLoss.length_m -eq 10) "Waveguide loss probe must use length_m=10"
Assert-True ($waveguideLoss.input_intensity -eq 1) "Waveguide loss probe must use intensity=1"
Assert-True ($waveguideLoss.output_intensity -le $waveguideLoss.input_intensity) "Waveguide loss must not amplify input"
Assert-True ($waveguideLoss.attenuation_db -ge 0) "Waveguide attenuation must be non-negative"
Assert-True ($waveguideLoss.passed -eq $true) "Waveguide loss probe must pass"

$photonicChecks = @($payload.checks.photonic)
Assert-True ($photonicChecks.Count -ge 2) "Missing photonic checks"
Assert-True ($photonicChecks[0].name -eq "truth_table") "Missing truth_table check"
Assert-True ($photonicChecks[0].passed -eq $true) "truth_table check must pass"
Assert-True ($photonicChecks[1].name -eq "waveguide_loss") "Missing waveguide_loss check"
Assert-True ($photonicChecks[1].passed -eq $true) "waveguide_loss check must pass"

$boundedState = $payload.evidence.wetware.bounded_state_probe
Assert-True ($null -ne $boundedState) "Missing wetware bounded_state_probe evidence"
Assert-True ($boundedState.bounded -eq $true) "Wetware state must remain bounded"
Assert-True ($boundedState.rewarded_dopamine -gt $boundedState.initial_dopamine) "Wetware reward must increase dopamine"
Assert-True ($boundedState.punished_dopamine -le $boundedState.rewarded_dopamine) "Wetware punishment must not increase dopamine"
Assert-True ($boundedState.decayed_dopamine -lt $boundedState.punished_dopamine) "Wetware dopamine must decay after tick"
Assert-True ($boundedState.passed -eq $true) "Wetware bounded-state probe must pass"

$wetwareChecks = @($payload.checks.wetware)
Assert-True ($wetwareChecks.Count -ge 1) "Missing wetware checks"
Assert-True ($wetwareChecks[0].name -eq "bounded_state_adaptation") "Missing bounded_state_adaptation check"
Assert-True ($wetwareChecks[0].passed -eq $true) "bounded_state_adaptation check must pass"

$performance = $payload.performance
Assert-True ($null -ne $performance) "Missing frontier probe performance metrics"
Assert-True ($performance.unit -eq "ns") "Frontier probe performance unit must be ns"
Assert-True ($performance.probe_count -eq 5) "Expected five frontier probe timings"
$probeTimings = @($performance.probes)
Assert-True ($probeTimings.Count -eq $performance.probe_count) "Frontier probe timing count mismatch"
$elapsedTotal = 0
foreach ($probeTiming in $probeTimings) {
    Assert-True (-not [string]::IsNullOrWhiteSpace($probeTiming.name)) "Frontier probe timing name is missing"
    Assert-True ($probeTiming.elapsed_ns -gt 0) "Frontier probe timing must be positive for $($probeTiming.name)"
    $elapsedTotal += [long]$probeTiming.elapsed_ns
}
Assert-True ($performance.total_elapsed_ns -eq $elapsedTotal) "Frontier probe total elapsed time mismatch"
Assert-True ($performance.slowest_probe.elapsed_ns -gt 0) "Frontier slowest probe timing must be positive"
Assert-True ($probeTimings.name -contains $performance.slowest_probe.name) "Frontier slowest probe must belong to measured probes"

[ordered]@{
    ok = $true
    schema = $schemaRef
    quality_level = $payload.summary.quality_level
    checked = @(
        "schema reference",
        "honest simulated/no-hardware flags",
        "all four implemented frontier modules",
        "bell stats evidence",
        "bell distribution check",
        "LIF threshold evidence",
        "LIF threshold response check",
        "photonic truth table evidence",
        "photonic waveguide loss evidence",
        "photonic Level 1 checks",
        "wetware bounded-state evidence",
        "wetware bounded-state adaptation check",
        "frontier probe performance timings"
    )
} | ConvertTo-Json -Depth 4
