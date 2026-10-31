#!/bin/bash
# Script para ejecutar todos los benchmarks y tests de RELIC

set -e

echo "=========================================="
echo "RELIC - Ejecución Completa de Benchmarks y Tests"
echo "=========================================="
echo ""

echo "1. Verificando compilación..."
cargo check --all-targets
echo "✓ Compilación exitosa"
echo ""

echo "2. Ejecutando clippy..."
cargo clippy --all-targets -- -D warnings
echo "✓ Clippy sin warnings"
echo ""

echo "3. Ejecutando tests unitarios..."
cargo test --release -p relic_core --lib
echo "✓ Tests unitarios exitosos"
echo ""

echo "4. Ejecutando test de arquitectura..."
cargo test --release -p relic_core --test architecture_evolution_benchmarks
echo "✓ Test de arquitectura exitoso"
echo ""

echo "5. Ejecutando suite completa de benchmarks..."
cargo run --release -p relic_benchmarks --bin relic_benchmarks
echo "✓ Benchmarks completados"
echo ""

echo "=========================================="
echo "✓ Todas las verificaciones completadas exitosamente"
echo "=========================================="
