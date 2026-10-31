#!/bin/bash
# Script para ejecutar todos los tests de RELIC

set -e

echo "=========================================="
echo "RELIC - Suite de Tests"
echo "=========================================="
echo ""

echo "1. Ejecutando tests unitarios..."
cargo test --release -p relic_core --lib
echo "✓ Tests unitarios exitosos"
echo ""

echo "2. Ejecutando test de arquitectura..."
cargo test --release -p relic_core --test architecture_evolution_benchmarks
echo "✓ Test de arquitectura exitoso"
echo ""

echo "=========================================="
echo "✓ Todos los tests completados exitosamente"
echo "=========================================="
