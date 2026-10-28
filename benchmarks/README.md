# RELIC — Batería Offline de Benchmarks y Evaluación Empírica

Este directorio contiene los arneses de benchmarking offline en Python para evaluar el comportamiento del motor neuro-simbólico RELIC frente a LLMs, inferencia episódica, transferencia entre dominios y análisis de errores:

## Contenido de la Carpeta

| Script | Propósito y Fenómeno Evaluado |
| :--- | :--- |
| [`benchmark.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark.py) | Punto de entrada unificado y despacho de benchmarks offline. |
| [`benchmark_cognitive_primitives.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_cognitive_primitives.py) | Evaluación de primitivas cognitivas (aritmética exacta, transitividad, precedencia RPN). |
| [`benchmark_episodic_reasoning.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_episodic_reasoning.py) | Evaluación de razonamiento episódico y memoria asociativa de grafos temporales. |
| [`benchmark_llm_comparison.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_llm_comparison.py) | Comparativa cara a cara de RELIC frente a modelos de lenguaje (exactitud, alucinación, latencia). |
| [`benchmark_cross_domain_transfer.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_cross_domain_transfer.py) | Pruebas de transferencia de conocimiento a dominios cruzados (inventario, ciberseguridad, spatial). |
| [`benchmark_semantic_interpreters.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_semantic_interpreters.py) | Evaluación exhaustiva de intérpretes semánticos y extracción de roles. |
| [`benchmark_semantic_error_matrix.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_semantic_error_matrix.py) | Matriz de confusión y categorización formal de causas de fallo o ambigüedad. |
| [`benchmark_semantic_bridge.py`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/benchmarks/benchmark_semantic_bridge.py) | Puente de verificación entre representaciones intermedias (RIR AST) y ejecución determinista. |

> **Nota:** La suite de benchmarks nativos en Rust (`cargo run -p relic_cli -- bench`) se encuentra orquestada en [`relic_cli::bench`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/crates/relic_cli/src/bench.rs) y documentada en [`docs/BENCHMARKS.md`](file:///c:/Users/Nicolas%20Pachon/Desktop/Seis%20Ojos/RELIC/docs/BENCHMARKS.md).

