# Matter Core 0.1.0 - EXTERNAL Windows validation (Phase 6)
param(
    [Parameter(Mandatory = $true)][string]$ZipPath,
    [string]$ExpectedSha256 = "0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2",
    [string]$WorkRoot = "",
    [switch]$AllowNonIndependent
)
$ErrorActionPreference = "Continue"
$ExpectedSha256 = $ExpectedSha256.ToUpperInvariant()
if (-not $WorkRoot) {
    $WorkRoot = Join-Path $env:TEMP ("MatterExternalVal-" + [guid]::NewGuid().ToString("n").Substring(0, 8))
}
New-Item -ItemType Directory -Force -Path $WorkRoot | Out-Null
$LogDir = Join-Path $WorkRoot "logs"
$EvidenceDir = Join-Path $WorkRoot "evidence"
New-Item -ItemType Directory -Force -Path $LogDir, $EvidenceDir | Out-Null
$results = New-Object System.Collections.Generic.List[object]
function Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "o"), $msg
    Add-Content -LiteralPath (Join-Path $LogDir "validation.log") -Value $line -Encoding utf8
    Write-Host $msg
}
function Add-R([string]$id, [bool]$ok, [string]$status, $detail) {
    $script:results.Add([pscustomobject]@{ id = $id; ok = $ok; status = $status; detail = $detail })
    Log ("[{0}] {1}" -f $status, $id)
}
function Invoke-Capture([string]$File, [string[]]$ArgList, [string]$Tag) {
    $outF = Join-Path $LogDir ($Tag + ".stdout.txt")
    $errF = Join-Path $LogDir ($Tag + ".stderr.txt")
    $argLine = ($ArgList | ForEach-Object { '"' + ($_ -replace '"', '""') + '"' }) -join ' '
    cmd /c "`"$File`" $argLine 1>`"$outF`" 2>`"$errF`""
    $code = $LASTEXITCODE
    $stdout = ""
    $stderr = ""
    if (Test-Path -LiteralPath $outF) { $stdout = Get-Content -LiteralPath $outF -Raw -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $errF) { $stderr = Get-Content -LiteralPath $errF -Raw -ErrorAction SilentlyContinue }
    return @{ exit = $code; stdout = $stdout; stderr = $stderr }
}
function Hash16([string]$s) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $bytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($s))
    $hex = ($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
    return $hex.Substring(0, 16)
}
$os = Get-CimInstance Win32_OperatingSystem
$hostInfo = [ordered]@{
    os_caption = $os.Caption
    os_version = $os.Version
    os_build = $os.BuildNumber
    os_arch = $os.OSArchitecture
    computer_id_hash = (Hash16 $env:COMPUTERNAME)
    user_id_hash = (Hash16 $env:USERNAME)
    is_admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}
$hostInfo | ConvertTo-Json | Set-Content (Join-Path $EvidenceDir "windows-identity.json") -Encoding utf8
Add-R "windows_identity" $true "PASS" $hostInfo
if ($hostInfo.is_admin) { Add-R "non_admin_user" $false "WARN" @{ note = "elevated" } }
else { Add-R "non_admin_user" $true "PASS" @{ note = "standard user" } }

$tools = [ordered]@{
    rustc = [bool](Get-Command rustc -ErrorAction SilentlyContinue)
    cargo = [bool](Get-Command cargo -ErrorAction SilentlyContinue)
    gcc = [bool](Get-Command gcc -ErrorAction SilentlyContinue)
    gpp = [bool](Get-Command g++ -ErrorAction SilentlyContinue)
    python = [bool](Get-Command python -ErrorAction SilentlyContinue) -or [bool](Get-Command python3 -ErrorAction SilentlyContinue)
    node = [bool](Get-Command node -ErrorAction SilentlyContinue)
    mingw_dir = (Test-Path "D:\mingw64") -or (Test-Path "C:\mingw64") -or (Test-Path "C:\msys64")
    d_matter = (Test-Path "D:\Matter")
}
$tools | ConvertTo-Json | Set-Content (Join-Path $EvidenceDir "dev-tools-absence.json") -Encoding utf8
$independent = -not ($tools.rustc -or $tools.cargo -or $tools.gcc -or $tools.gpp -or $tools.python -or $tools.node -or $tools.mingw_dir -or $tools.d_matter)

if (-not $independent -and -not $AllowNonIndependent) {
    Add-R "environment_independence" $false "BLOCKED" @{ tools = $tools; reason = "Host has build/dev tooling or D:\Matter - not independent per Phase 6" }
    $summary = [pscustomobject]@{
        schema_version = 1
        phase = 6
        at = (Get-Date).ToString("o")
        verdict = "BLOCKED"
        expected_sha256 = $ExpectedSha256
        results = $results
        host = $hostInfo
        tools = $tools
        independent = $false
        work_root = $WorkRoot
        note = "Official EXTERNAL_VALIDATION_PASS requires a clean Windows without Rust/Cargo/GCC/Python/Node and without D:\Matter"
    }
    $summary | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $EvidenceDir "external-windows-validation-v1.json") -Encoding utf8
    Copy-Item (Join-Path $EvidenceDir "external-windows-validation-v1.json") (Join-Path $WorkRoot "external-windows-validation-v1.json") -Force
    Log "VERDICT=BLOCKED"
    Write-Host "Evidence: $EvidenceDir"
    exit 2
}
if (-not $independent) {
    Add-R "environment_independence" $false "WARN" @{ tools = $tools; note = "AllowNonIndependent - not official PASS" }
} else {
    Add-R "environment_independence" $true "PASS" @{ tools = $tools }
}

if (-not (Test-Path -LiteralPath $ZipPath)) {
    Add-R "zip_present" $false "FAIL" @{ path = $ZipPath }
    Write-Host "VERDICT=EXTERNAL_VALIDATION_FAIL"
    exit 1
}
$ZipPath = (Resolve-Path -LiteralPath $ZipPath).Path
Add-R "zip_present" $true "PASS" @{ path = $ZipPath; bytes = (Get-Item -LiteralPath $ZipPath).Length }
$got = (Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath).Hash.ToUpperInvariant()
$match = ($got -eq $ExpectedSha256)
Add-R "zip_sha256" $match $(if ($match) { "PASS" } else { "FAIL" }) @{ expected = $ExpectedSha256; got = $got }
if (-not $match) {
    Log "VERDICT=EXTERNAL_VALIDATION_FAIL hash mismatch - stop before extract"
    $summary = [pscustomobject]@{ schema_version=1; phase=6; at=(Get-Date).ToString("o"); verdict="EXTERNAL_VALIDATION_FAIL"; results=$results; host=$hostInfo; tools=$tools }
    $summary | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $EvidenceDir "external-windows-validation-v1.json") -Encoding utf8
    exit 1
}

$extractRoot = Join-Path $WorkRoot "Matter Core Extract"
New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($ZipPath, $extractRoot)
    Add-R "extract_spaces_path" $true "PASS" @{ path = $extractRoot }
} catch {
    Add-R "extract_spaces_path" $false "FAIL" @{ error = "$_" }
    exit 1
}

$cli = Join-Path $extractRoot "bin\matter.exe"
if (-not (Test-Path -LiteralPath $cli)) { $cli = Join-Path $extractRoot "bin\matter-cli.exe" }
if (-not (Test-Path -LiteralPath $cli)) { Add-R "cli_in_package" $false "FAIL" @{ extract = $extractRoot }; exit 1 }
Add-R "cli_in_package" $true "PASS" @{ cli = $cli }

$oldPath = $env:PATH
$env:PATH = Join-Path $env:SystemRoot "System32"

$r = Invoke-Capture $cli @("--help") "pkg-help"
Add-R "pkg_help" ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $r.exit; stderr = $r.stderr }
$r = Invoke-Capture $cli @("--version") "pkg-version"
Add-R "pkg_version" ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $r.exit; stdout = $r.stdout }
$r = Invoke-Capture $cli @("core-status-json") "pkg-core-status"
$coreOk = ($r.exit -eq 0) -and ($r.stdout -match '"ok"\s*:\s*true')
Add-R "pkg_core_status" $coreOk $(if ($coreOk) { "PASS" } else { "FAIL" }) @{ exit = $r.exit }

$hello = Join-Path $extractRoot "examples\hello.matter"
$mbc = Join-Path $WorkRoot "hello-external.mbc"
if (Test-Path -LiteralPath $hello) {
    $r = Invoke-Capture $cli @("run", $hello) "pkg-run"
    Add-R "pkg_run_example" ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $r.exit; stderr = $r.stderr }
    $r = Invoke-Capture $cli @("compile", $hello, "-o", $mbc) "pkg-compile"
    Add-R "pkg_compile" ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $r.exit }
    $r = Invoke-Capture $cli @("run-bytecode", $mbc) "pkg-run-bytecode"
    Add-R "pkg_run_bytecode" ($r.exit -eq 0) $(if ($r.exit -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $r.exit }
} else {
    Add-R "pkg_run_example" $false "FAIL" @{ reason = "hello.matter missing" }
}
$env:PATH = $oldPath

$installRoot = Join-Path $WorkRoot "Install Root Alpha"
$installScript = Join-Path $extractRoot "scripts\install-matter-core.ps1"
if (Test-Path -LiteralPath $installScript) {
    $rOut = Join-Path $LogDir "install.stdout.txt"
    $rErr = Join-Path $LogDir "install.stderr.txt"
    cmd /c "powershell -NoProfile -ExecutionPolicy Bypass -File `"$installScript`" -PackageRoot `"$extractRoot`" -InstallRoot `"$installRoot`" -SkipPath 1>`"$rOut`" 2>`"$rErr`""
    Add-R "install_non_D_Matter" ($LASTEXITCODE -eq 0) $(if ($LASTEXITCODE -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $LASTEXITCODE; install_root = $installRoot }
} else {
    Add-R "install_non_D_Matter" $false "FAIL" @{ reason = "install script missing" }
}

$verifyScript = Join-Path $extractRoot "scripts\verify-matter-core.ps1"
if (Test-Path -LiteralPath $verifyScript) {
    $vOut = Join-Path $LogDir "verify.stdout.txt"
    $vErr = Join-Path $LogDir "verify.stderr.txt"
    cmd /c "powershell -NoProfile -ExecutionPolicy Bypass -File `"$verifyScript`" -InstallRoot `"$installRoot`" -MinimalPath 1>`"$vOut`" 2>`"$vErr`""
    Add-R "verify_script" ($LASTEXITCODE -eq 0) $(if ($LASTEXITCODE -eq 0) { "PASS" } else { "FAIL" }) @{ exit = $LASTEXITCODE }
} else {
    Add-R "verify_script" $false "FAIL" @{ reason = "verify script missing" }
}

$userProj = Join-Path $installRoot "projects\user-owned.txt"
New-Item -ItemType Directory -Force -Path (Split-Path $userProj) | Out-Null
Set-Content -LiteralPath $userProj -Value "user-owned-marker-do-not-delete" -Encoding utf8
$foreign = Join-Path $installRoot "foreign-not-matter.txt"
Set-Content -LiteralPath $foreign -Value "should-survive-uninstall" -Encoding utf8
Add-R "user_project_created" (Test-Path -LiteralPath $userProj) "PASS" @{ path = "projects/user-owned.txt" }

$updateScript = Join-Path $extractRoot "scripts\update-matter-core.ps1"
if (Test-Path -LiteralPath $updateScript) {
    $uOut = Join-Path $LogDir "update.stdout.txt"
    $uErr = Join-Path $LogDir "update.stderr.txt"
    cmd /c "powershell -NoProfile -ExecutionPolicy Bypass -File `"$updateScript`" -PackageRoot `"$extractRoot`" -InstallRoot `"$installRoot`" -SkipPath 1>`"$uOut`" 2>`"$uErr`""
    $uok = ($LASTEXITCODE -eq 0) -and (Test-Path -LiteralPath $userProj)
    Add-R "update_preserves_user_project" $uok $(if ($uok) { "PASS" } else { "FAIL" }) @{ exit = $LASTEXITCODE }
} else {
    Add-R "update_preserves_user_project" $false "FAIL" @{ reason = "update script missing" }
}

$uninstallScript = Join-Path $extractRoot "scripts\uninstall-matter-core.ps1"
if (Test-Path -LiteralPath $uninstallScript) {
    $xOut = Join-Path $LogDir "uninstall.stdout.txt"
    $xErr = Join-Path $LogDir "uninstall.stderr.txt"
    cmd /c "powershell -NoProfile -ExecutionPolicy Bypass -File `"$uninstallScript`" -InstallRoot `"$installRoot`" 1>`"$xOut`" 2>`"$xErr`""
    $cliGone = -not (Test-Path -LiteralPath (Join-Path $installRoot "bin\matter-cli.exe"))
    $projKept = Test-Path -LiteralPath $userProj
    $foreignKept = Test-Path -LiteralPath $foreign
    $uok = $cliGone -and $projKept -and $foreignKept
    Add-R "uninstall_selective" $uok $(if ($uok) { "PASS" } else { "FAIL" }) @{ exit = $LASTEXITCODE; cli_gone = $cliGone; project_kept = $projKept; foreign_kept = $foreignKept }
} else {
    Add-R "uninstall_selective" $false "FAIL" @{ reason = "uninstall script missing" }
}

foreach ($cmdName in @("agent-ui", "shell", "package-install", "polyglot-status-json")) {
    $r = Invoke-Capture $cli @($cmdName) ("deny-" + $cmdName)
    $ok = ($r.exit -eq 2) -and ($r.stderr -match "NOT executed|not available")
    Add-R ("deny_" + $cmdName) $ok $(if ($ok) { "PASS" } else { "FAIL" }) @{ exit = $r.exit; stderr = $r.stderr }
}
Add-R "runtime_policy_notes" $true "INFO" @{ note = "Record SmartScreen/AV/PowerShell policy failures if observed" }

$fails = @($results | Where-Object { $_.status -eq "FAIL" }).Count
$blocked = @($results | Where-Object { $_.status -eq "BLOCKED" }).Count
if ($blocked -gt 0) { $verdict = "BLOCKED" }
elseif ($fails -gt 0) { $verdict = "EXTERNAL_VALIDATION_FAIL" }
else { $verdict = "EXTERNAL_VALIDATION_PASS" }
if ($AllowNonIndependent -and $verdict -eq "EXTERNAL_VALIDATION_PASS") {
    $verdict = "BLOCKED"
    Add-R "official_pass_disallowed" $false "BLOCKED" @{ reason = "AllowNonIndependent cannot yield official PASS" }
}

$summary = [pscustomobject]@{
    schema_version = 1
    phase = 6
    at = (Get-Date).ToString("o")
    verdict = $verdict
    expected_sha256 = $ExpectedSha256
    zip_path = $ZipPath
    work_root = $WorkRoot
    host = $hostInfo
    tools = $tools
    independent = $independent
    results = $results
    passed = @($results | Where-Object { $_.status -eq "PASS" }).Count
    failed = $fails
    blocked = $blocked
}
$jsonOut = Join-Path $EvidenceDir "external-windows-validation-v1.json"
$summary | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $jsonOut -Encoding utf8
Copy-Item $jsonOut (Join-Path $WorkRoot "external-windows-validation-v1.json") -Force
Log ("VERDICT=" + $verdict)
Write-Host ("Evidence: " + $EvidenceDir)
if ($verdict -eq "EXTERNAL_VALIDATION_PASS") { exit 0 }
if ($verdict -eq "BLOCKED") { exit 2 }
exit 1