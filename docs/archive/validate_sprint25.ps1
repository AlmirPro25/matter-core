# Sprint 25 Validation Script
# Validates LLVM backend implementation

Write-Host "=== Sprint 25 Validation ===" -ForegroundColor Cyan
Write-Host ""

# Check if LLVM is installed
Write-Host "Checking LLVM installation..." -ForegroundColor Yellow
$llvmConfig = Get-Command llvm-config -ErrorAction SilentlyContinue

if (-not $llvmConfig) {
    Write-Host "❌ LLVM not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install LLVM 17:" -ForegroundColor Yellow
    Write-Host "1. Download from: https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6"
    Write-Host "2. Run installer (check 'Add to PATH')"
    Write-Host "3. Set environment variable:"
    Write-Host "   setx LLVM_SYS_170_PREFIX `"C:\Program Files\LLVM`""
    Write-Host "4. Restart terminal"
    Write-Host ""
    Write-Host "See: crates\matter-llvm\LLVM_WINDOWS_INSTALL.md"
    exit 1
}

$llvmVersion = & llvm-config --version
Write-Host "✓ LLVM found: $llvmVersion" -ForegroundColor Green

if (-not $llvmVersion.StartsWith("17.")) {
    Write-Host "⚠ Warning: Expected LLVM 17.x, found $llvmVersion" -ForegroundColor Yellow
}

Write-Host ""

# Check environment variable
Write-Host "Checking LLVM_SYS_170_PREFIX..." -ForegroundColor Yellow
$llvmPrefix = $env:LLVM_SYS_170_PREFIX

if (-not $llvmPrefix) {
    Write-Host "⚠ LLVM_SYS_170_PREFIX not set" -ForegroundColor Yellow
    Write-Host "Setting to default: C:\Program Files\LLVM"
    $env:LLVM_SYS_170_PREFIX = "C:\Program Files\LLVM"
} else {
    Write-Host "✓ LLVM_SYS_170_PREFIX: $llvmPrefix" -ForegroundColor Green
}

Write-Host ""

# Format code
Write-Host "Formatting code..." -ForegroundColor Yellow
cargo fmt
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Format failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Code formatted" -ForegroundColor Green
Write-Host ""

# Check workspace
Write-Host "Checking workspace..." -ForegroundColor Yellow
cargo check --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Workspace check failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Workspace check passed" -ForegroundColor Green
Write-Host ""

# Build matter-llvm
Write-Host "Building matter-llvm..." -ForegroundColor Yellow
cargo build -p matter-llvm
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Build succeeded" -ForegroundColor Green
Write-Host ""

# Test matter-llvm
Write-Host "Testing matter-llvm..." -ForegroundColor Yellow
cargo test -p matter-llvm
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Tests failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Tests passed" -ForegroundColor Green
Write-Host ""

# Test workspace
Write-Host "Testing workspace..." -ForegroundColor Yellow
cargo test --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Workspace tests failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Workspace tests passed" -ForegroundColor Green
Write-Host ""

# Build CLI with LLVM feature
Write-Host "Building CLI with LLVM..." -ForegroundColor Yellow
cargo build -p matter-cli --features llvm
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ CLI build failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ CLI built with LLVM support" -ForegroundColor Green
Write-Host ""

# Test examples
Write-Host "=== Testing Examples ===" -ForegroundColor Cyan
Write-Host ""

$examples = @(
    @{
        Name = "Simple Test"
        File = "examples\sprint25_simple.matter"
        Expected = "30"
    },
    @{
        Name = "Function Test"
        File = "examples\sprint25_test.matter"
        Expected = "60"
    }
)

$allPassed = $true

foreach ($example in $examples) {
    Write-Host "Testing: $($example.Name)" -ForegroundColor Yellow
    Write-Host "File: $($example.File)"
    
    # Test with bytecode
    Write-Host "  Running bytecode..." -NoNewline
    $bytecodeOutput = & cargo run -p matter-cli --quiet -- run $example.File 2>&1 | Select-Object -Last 1
    if ($bytecodeOutput -eq $example.Expected) {
        Write-Host " ✓" -ForegroundColor Green
    } else {
        Write-Host " ❌ (got: $bytecodeOutput, expected: $($example.Expected))" -ForegroundColor Red
        $allPassed = $false
    }
    
    # Test with native (if LLVM available)
    Write-Host "  Running native..." -NoNewline
    $nativeOutput = & cargo run -p matter-cli --features llvm --quiet -- run-native $example.File 2>&1 | Select-Object -Last 1
    if ($nativeOutput -eq $example.Expected) {
        Write-Host " ✓" -ForegroundColor Green
    } else {
        Write-Host " ❌ (got: $nativeOutput, expected: $($example.Expected))" -ForegroundColor Red
        $allPassed = $false
    }
    
    Write-Host ""
}

# Benchmark test
Write-Host "Testing benchmark..." -ForegroundColor Yellow
Write-Host "File: examples\sprint25_benchmark.matter"
$benchmarkOutput = & cargo run -p matter-cli --features llvm --quiet -- benchmark examples\sprint25_benchmark.matter --iterations 3 2>&1
Write-Host $benchmarkOutput
Write-Host ""

# Summary
Write-Host "=== Validation Summary ===" -ForegroundColor Cyan
Write-Host ""

if ($allPassed) {
    Write-Host "✓ All validations passed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Sprint 25 Status: 80% VALIDATED" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "1. Review performance metrics"
    Write-Host "2. Complete remaining 20% (for loops, optimization flags)"
    Write-Host "3. Start Sprint 26 (JIT Compilation)"
    exit 0
} else {
    Write-Host "❌ Some validations failed" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please review errors above and fix issues."
    exit 1
}
