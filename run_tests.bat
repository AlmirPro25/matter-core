@echo off
echo ========================================
echo MATTER CORE - SUITE DE TESTES
echo ========================================
echo.

echo [1/5] Testando Biofisica...
cargo run --bin matter-cli -- test_biophysics.matter
echo.

echo [2/5] Testando Fisica Avancada...
cargo run --bin matter-cli -- test_physics.matter
echo.

echo [3/5] Testando Compilador Nativo...
cargo run --bin matter-cli -- test_native.matter
echo.

echo [4/5] Testando Quantum Computing...
cargo run --bin matter-cli -- test_quantum.matter
echo.

echo [5/5] Testando TUDO...
cargo run --bin matter-cli -- test_all.matter
echo.

echo ========================================
echo TESTES COMPLETOS!
echo ========================================
pause
