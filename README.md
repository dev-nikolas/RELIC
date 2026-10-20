# RELIC-10M — LLM Propio y Motor de Inferencia en Rust

[![Rust 1.80+](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg?style=flat-square)](LICENSE)
[![CI Matrix](https://img.shields.io/badge/CI-Windows%20%7C%20Linux-brightgreen.svg?style=flat-square&logo=githubactions)](.github/workflows/ci.yml)
[![Parameters](https://img.shields.io/badge/Parameters-10.03M-informational.svg?style=flat-square)](#especificaciones-tecnicas-de-la-arquitectura)
[![Inference Speed](https://img.shields.io/badge/Decode-180.3%20tok%2Fs%20(CPU)-success.svg?style=flat-square)](#rendimiento-y-benchmark-oficial)
[![Memory RSS](https://img.shields.io/badge/Memory%20RSS-%3C%2050%20MB-blueviolet.svg?style=flat-square)](#funcionalidad-destacable-motor-nativo-en-rust--80-mb-rss)

> **Ingenieria de Sistemas y Modelos de Lenguaje:** Construccion integral de un modelo Transformer decodificador moderno (10.03M parametros) entrenado desde cero, acompasado de un motor de inferencia nativo en Rust puro sin dependencias externas ni runtime de Python.

[Reporte de Benchmark](reports/benchmark_report.json) • [Metricas de Entrenamiento](reports/dry_run_metrics.json) • [Workflow CI Multiplataforma](.github/workflows/ci.yml)

---

## Motivacion y Proposito

La mayoria de proyectos de Inteligencia Artificial dependen de frameworks masivos en Python y binarios precompilados que consumen gigabytes de memoria y ocultan la mecanica real del modelo. RELIC demuestra lo contrario:

1. **Comprension Integral de Extremo a Extremo:** Cada capa (RMSNorm, RoPE, QK-RMSNorm, GQA, SwiGLU), el tokenizador BPE y el runtime de ejecucion estan construidos directamente a partir de primeros principios matematicos.
2. **Entrenamiento Viable en Hardware Domestico:** Diseno calibrado a 10.03 millones de parametros que cabe comodamente en la cache L2/L3 de una CPU estandar (Intel Core i5-9400F) y entrena sobre TinyStories en tiempos predecibles.
3. **Cero Dependencias en Produccion:** Inferencia nativa sin Python, CUDA ni LibTorch, mediante un binario autonomo compilado en Rust con footprint minimo.

---

## Funcionalidad Destacable: Motor Nativo en Rust (< 80 MB RSS)

El nucleo del proyecto es su motor de inferencia (`relic_core`), disenado con enfoque en rendimiento de sistemas y consumo ultrabajo de recursos:

* **Huella de Memoria Minima:** Requiere menos de 50 MB de memoria fisica residente (RSS) activa durante la decodificacion autoregresiva continua.
* **Cero Dependencias Externas:** Algebra lineal, normalizaciones, rotaciones RoPE y atencion causal implementadas en Rust puro.
* **KV-Cache Dinamico Preasignado:** Buffer circular de < 2 MB para secuencias de hasta 256 tokens, eliminando fragmentacion de memoria y logrando latencias $O(1)$ en decodificacion.
* **Carga Zero-Copy Instantanea:** Carga 69 tensores Safetensors en apenas 29 ms mediante mapeo de memoria directa (`memmap2`).

---

## Rendimiento y Benchmark Oficial

Medicion realizada en CPU Intel Core i5-9400F @ 2.90GHz (registro completo en [`reports/benchmark_report.json`](reports/benchmark_report.json)):

| Motor de Ejecucion | Modo de Inferencia | Throughput | Consumo RAM (RSS) | Aceleracion |
| :--- | :--- | :---: | :---: | :---: |
| **PyTorch CPU** | Decode Autoregresivo (6 cores OpenMP) | 125.9 tok/s | ~420 MB | Base (1.0x) |
| **RELIC Rust Core** | **Decode con KV-Cache + FMA/AVX2** | **180.3 tok/s** | **< 50 MB** | **1.43x mas rapido** |

---

## Especificaciones Tecnicas de la Arquitectura

| Parametro | Valor RELIC-10M | Funcion Matematica y Diseno |
| :--- | :--- | :--- |
| **Parametros Totales** | **10,031,328 (10.03M)** | Pesos desatados (~19.13 MB en formato `float16`) |
| **Capas Transformer** | **6 bloques decodificadores** | Pre-normalizacion dual con conexiones residuales |
| **Dimension Oculta ($d_{model}$)** | **288** | Dimension residual alineada a lineas de cache de CPU |
| **Mecanismo de Atencion** | **GQA 3:1 (6 Q / 2 KV)** | 6 cabezas Query ($d_k=48$) y 2 cabezas Key/Value |
| **Estabilizador de Atencion** | **QK-RMSNorm** | Normalizacion individual por cabeza antes de RoPE |
| **Embeddings Posicionales** | **RoPE ($\theta = 10,000$)** | Rotacion de posiciones isometrica ($T_{\text{max}} = 512$) |
| **Feed-Forward / MLP** | **SwiGLU ($d_{ff} = 768$)** | Compuertas no lineales `gate_proj`, `up_proj`, `down_proj` |
| **Normalizacion** | **RMSNorm ($\epsilon = 10^{-6}$)** | Normalizacion cuadratica media sin sesgo residual |
| **Vocabulario BPE** | **8,192 tokens** | Tokenizador BPE portable sin lookarounds |

---

## Inicio Rapido (Getting Started)

### 1. Compilar y Ejecutar en Rust
```bash
# Ejecutar la suite completa de 13 pruebas unitarias
cargo test --workspace

# Ejecutar inferencia optimizada con KV-Cache y pesos Safetensors
cargo run -p relic_cli --release
```

### 2. Entrenamiento y Herramientas (Python 3.12)
```bash
# 1. Descarga y particion limpia 95/5 de TinyStories
python tools/prepare_dataset.py

# 2. Entrenar tokenizador BPE de 8,192 tokens
python tools/train_bpe.py

# 3. Dry-run de entrenamiento y calibracion de throughput
python tools/train.py --dry-run --max-steps 100

# 4. Exportar checkpoint entrenado a formato Safetensors
python tools/export_safetensors.py
```

---

## Estructura del Repositorio

```text
RELIC/
├── Cargo.toml                    # Configuracion del workspace de Rust
├── crates/
│   ├── relic_core/               # Motor neuronal y operadores tensoriales nativos
│   │   ├── src/config.rs         # Hiperparametros y calculo analitico de parametros
│   │   ├── src/ops.rs            # Algebra lineal FMA/AVX2, SiLU, SwiGLU y Softmax
│   │   ├── src/norm.rs           # RMSNorm en precision float32
│   │   ├── src/rotary.rs         # Rotary Position Embeddings (RoPE)
│   │   ├── src/mlp.rs            # Bloque Feed-Forward SwiGLU
│   │   ├── src/attention.rs      # Atencion causal GQA con QK-Norm y KV-Cache
│   │   ├── src/block.rs          # Capa Transformer decodificadora
│   │   ├── src/model.rs          # Modelo causal RELIC-10M con prefill y decode
│   │   ├── src/kv_cache.rs       # KV-Cache dinamico preasignado (< 2 MB)
│   │   ├── src/sampler.rs        # Muestreo Greedy y temperatura
│   │   └── src/loader.rs         # Cargador zero-copy Safetensors (memmap2)
│   ├── relic_tokenizer/          # Tokenizador BPE nativo en Rust
│   │   ├── src/byte_level.rs     # Mapeo biyectivo de bytes
│   │   ├── src/vocab.rs          # Cargador de vocabulario y merges
│   │   └── src/bpe.rs            # Motor de codificacion y decodificacion
│   └── relic_cli/                # Binario de linea de comandos e inferencia
│       └── src/main.rs           # Generacion autoregresiva acelerada
├── tools/                        # Herramientas de soporte y calibracion
│   ├── common.py                 # Constantes de proyecto
│   ├── prepare_dataset.py        # Descarga y limpieza ASCII
│   ├── train_bpe.py              # Entrenamiento BPE y reporte
│   ├── train.py                  # Motor de entrenamiento AdamW + Cosine Decay
│   ├── dataset.py                # Dataloader autoregresivo
│   ├── export_safetensors.py     # Exportador PyTorch -> Safetensors FP16
│   └── benchmark.py              # Benchmark comparativo PyTorch vs Rust
└── .github/workflows/ci.yml      # CI Multiplataforma (Windows y Linux)
```
