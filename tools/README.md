# RELIC — Herramientas y Scripts del Ecosistema (`tools/`)

Este directorio contiene las herramientas de soporte, procesamiento de datos, entrenamiento, benchmarking y evaluación estadística para **RELIC**.

---

## Estructura de Directorios

```
tools/
├── benchmarks/         # Benchmarks de rendimiento, transferencia y comparativas con LLMs
├── data/               # Preparación de datasets, control de fugas (3-gramas) y auditoría
├── eval/               # Evaluación empírica caso a caso (Wilson 95%, McNemar, hold-outs)
├── training/           # Scripts de pre-entrenamiento, tokenización BPE y ablaciones (H1-H3)
└── [raíz]              # Utilidades nucleares compartidas e inferencia básica
```

---

## 1. Utilidades en Raíz (`tools/`)
- **`common.py`**: Constantes globales de arquitectura (vocabulario 8k, capas, dimensiones) y rutas canónicas del filesystem.
- **`download_weights.py`**: Script de descarga de artefactos binarios de pesos con verificación criptográfica SHA-256 (evita inflar el historial de Git).
- **`evaluate_harness.py`**: Funciones matemáticas puras para intervalos de confianza de Wilson al 95% y prueba pareada de McNemar con corrección de Edwards.
- **`run_model_inference.py`**: Inferencia local del modelo autorregresivo en PyTorch con tokenizador por bytes.
- **`export_safetensors.py`**: Conversor y exportador de checkpoints PyTorch al formato binario Safetensors FP16 compatible con `relic_core::loader`.
- **`verify_parity.py`**: Validador de paridad numérica exacta entre tensores PyTorch y `safe_open` (tolerancia $10^{-7}$).
- **`validate_tokenizer.py`**: Pruebas de roundtrip de tokenización y cobertura de caracteres especiales en español.
- **`smoke_test.py` / `smoke_test_gateway.py`**: Pruebas rápidas de humo para validar salud y latencia del servidor HTTP y la pasarela SSE.

---

## 2. Benchmarks (`tools/benchmarks/`)
- **`benchmark.py`**: Medición de latencia y throughput de inferencia.
- **`benchmark_llm_comparison.py`**: Comparativa de exactitud y tasa de alucinación frente a LLMs comerciales/locales.
- **`benchmark_cognitive_primitives.py`**: Evaluación de primitivas simbólicas (inversión, concordancia, aritmética básica).
- **`benchmark_cross_domain_transfer.py`**: Medición de transferencia entre dominios (inventario vs. grafos).
- **`benchmark_episodic_reasoning.py`**: Pruebas de estrés para búsqueda BFS en memoria episódica.
- **`benchmark_semantic_interpreters.py`**: Evaluación comparativa de parsers semánticos.

---

## 3. Preparación y Control de Datos (`tools/data/`)
- **`dataset.py` / `prepare_dataset.py`**: Descarga, particionado 95/5 y tokenización de TinyStories en streaming.
- **`build_representative_corpus.py`**: Construcción de corpus representativo en español para calibración léxica.
- **`generate_semantic_dataset.py`**: Generador sintético de pares de lenguaje a RIR con balanceo de operadores.
- **`leak_control.py`**: Verificador de solapamiento estricto de 3-gramas entre conjuntos Train y Holdout.
- **`audit_numbers.py`**: Auditoría de concordancia numérica entre el texto de la consulta y los operandos del gold RIR.

---

## 4. Evaluación Empírica (`tools/eval/`)
- **`eval_holdout_75_per_case.py`**: Evaluación caso por caso de los 75 ejemplos de holdout enfrentando Reglas, Slot-Filler y Modelo Causal.
- **`eval_holdout_v2_breakdown.py`**: Evaluación masiva per-caso sobre los 4.500 casos de hold-out v2 con desglose por nivel y por plantilla.
- **`eval_slot_filler_4500.py`**: Evaluación especializada del baseline de slot-filler sobre los 3.000 casos resolubles.
- **`measure_baseline.py`**: Cálculo de líneas base sobre suites congeladas (`unseen_frozen_v2`, `known_failures_v2`).
- **`fix_holdout_75_gold.py`**: Script de auditoría y corrección de operandos en el banco de 75 casos.

---

## 5. Entrenamiento y Ablaciones (`tools/training/`)
- **`train.py`**: Bucle principal de pre-entrenamiento autoregresivo en PyTorch.
- **`train_bpe.py`**: Entrenamiento del tokenizador BPE en español de 8.192 tokens.
- **`train_colab.py`**: Adaptador ligero para ejecución de fine-tuning en Google Colab / Kaggle.
- **`train_ablation_h1_iterative.py` / `train_ablation_h1_recurrent.py`**: Ablaciones de recurrencia en compilación semántica.
- **`train_ablation_h2_toroidal.py` / `train_ablation_h3_consensus.py`**: Experimentos de embeddings sinusoidales toroidales y consenso CRT.