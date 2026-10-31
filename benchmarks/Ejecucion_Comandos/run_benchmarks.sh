#!/bin/bash
# Script para ejecutar la suite completa de benchmarks de RELIC

set -e

echo "=========================================="
echo "RELIC - Suite Completa de Benchmarks"
echo "=========================================="
echo ""

echo "[1/9] Ejecutando suite científica de benchmarks..."
cargo run --release -p relic_benchmarks --bin relic_benchmarks
echo ""

echo "[2/9] Ejecutando benchmark de escala..."
cargo run --release -p relic_benchmarks --bin measure_scale
echo ""

echo "[3/9] Ejecutando benchmark de horizonte..."
cargo run --release -p relic_benchmarks --bin measure_horizon
echo ""

echo "[4/9] Ejecutando benchmark de commit breakdown..."
cargo run --release -p relic_benchmarks --bin measure_commit_breakdown
echo ""

echo "[5/9] Ejecutando benchmark de SPRT validation..."
cargo run --release -p relic_benchmarks --bin measure_sprt_validation
echo ""

echo "[6/9] Ejecutando benchmark de MQuAKE edits..."
cargo run --release -p relic_benchmarks --bin measure_mquake_edits
echo ""

echo "[7/9] Ejecutando benchmark de estructuras..."
cargo run --release -p relic_benchmarks --bin measure_structures
echo ""

echo "[8/9] Ejecutando benchmark de MS-BFS batch..."
cargo run --release -p relic_benchmarks --bin measure_ms_bfs_batch
echo ""

echo "[9/9] Ejecutando benchmark de originalidad..."
cargo run --release -p relic_benchmarks --bin measure_relic_originality
echo ""

echo "=========================================="
echo "✓ Todos los benchmarks completados exitosamente"
echo "=========================================="
