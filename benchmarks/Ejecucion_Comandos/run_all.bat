@echo off
REM Script para ejecutar todos los benchmarks y tests de RELIC

echo ==========================================
echo RELIC - Ejecución Completa de Benchmarks y Tests
echo ==========================================
echo.

echo 1. Verificando compilación...
cargo check --all-targets
if %errorlevel% neq 0 exit /b %errorlevel%
echo [OK] Compilación exitosa
echo.

echo 2. Ejecutando clippy...
cargo clippy --all-targets -- -D warnings
if %errorlevel% neq 0 exit /b %errorlevel%
echo [OK] Clippy sin warnings
echo.

echo 3. Ejecutando tests unitarios...
cargo test --release -p relic_core --lib
if %errorlevel% neq 0 exit /b %errorlevel%
echo [OK] Tests unitarios exitosos
echo.

echo 4. Ejecutando test de arquitectura...
cargo test --release -p relic_core --test architecture_evolution_benchmarks
if %errorlevel% neq 0 exit /b %errorlevel%
echo [OK] Test de arquitectura exitoso
echo.

echo 5. Ejecutando suite completa de benchmarks...
cargo run --release -p relic_benchmarks --bin relic_benchmarks
if %errorlevel% neq 0 exit /b %errorlevel%
echo [OK] Benchmarks completados
echo.

echo ==========================================
echo [OK] Todas las verificaciones completadas exitosamente
echo ==========================================
