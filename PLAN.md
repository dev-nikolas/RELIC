# Plan Maestro de Ingeniería: Creación de Nuestro Propio LLM (RELIC-10M)

> **Documento de Diseño Técnico y Hoja de Ruta de Producto**  
> **Objetivo:** Construir, entrenar desde cero y desplegar **nuestro propio LLM (10.0M parámetros)** con arquitectura moderna (RMSNorm, RoPE, QK-Norm, SwiGLU), tokenizer BPE propio y motor de inferencia nativo en **Rust puro**, completamente **multiplataforma (Windows, Linux y macOS sin ataduras)**.

---

## 1. Desglose Matemático Exacto de Parámetros y Memoria

### 1.1 Conteo de Parámetros de la Arquitectura (10.03M)
* **Vocabulario BPE:** $V = 8,192$ tokens.
* **Dimensión Oculta:** $d_{model} = 288$.
* **Capas Transformer:** $L = 6$ bloques decodificadores.
* **Longitud Máxima de Contexto:** $T_{\text{max}} = 512$ tokens (RoPE $\theta = 10,000$).
* **Longitud de Secuencia de Entrenamiento:** $T_{\text{train}} = 256$ tokens (fijado como límite operativo en el runtime Rust para evitar degradación de atención fuera de distribución).
* **Atención GQA (3:1):** $H_Q = 6$ cabezas ($d_k = 48$), $H_{KV} = 2$ cabezas ($d_{kv} = 96$). Sin ventana deslizante (*full causal attention*).
* **MLP SwiGLU:** $d_{ff} = 768$ (expansión $\approx 2.67\times$).
* **QK-RMSNorm:** Se aplica por cabeza sobre $d_k=48$ y se comparte entre cabezas ($48 \text{ para } Q + 48 \text{ para } K = 96$ parámetros por capa).

```text
Componente                               Fórmula de Parámetros                Parámetros
----------------------------------------------------------------------------------------
Embedding de Entrada                     V × d_model = 8192 × 288              2,359,296
Capa de Atención (x6):
  - W_Q                                  d_model × (H_Q × d_k) = 288 × 288        82,944
  - W_K (GQA)                            d_model × (H_KV × d_k) = 288 × 96        27,648
  - W_V (GQA)                            d_model × (H_KV × d_k) = 288 × 96        27,648
  - W_O                                  (H_Q × d_k) × d_model = 288 × 288        82,944
  - QK-RMSNorm (compartido por cabezas)  d_k (Q) + d_k (K) = 48 + 48                  96
Capa MLP SwiGLU (x6):
  - gate_proj (Swish)                    d_model × d_ff = 288 × 768              221,184
  - up_proj                              d_model × d_ff = 288 × 768              221,184
  - down_proj                            d_ff × d_model = 768 × 288              221,184
Normalizaciones RMSNorm (x6):            2 × d_model = 2 × 288                       576
----------------------------------------------------------------------------------------
Total por Capa Transformer                                                       885,408
Total 6 Capas Transformer                6 × 885,408                           5,312,448
RMSNorm Final                            d_model = 288                               288
LM Head (Des-embedding desatado)         d_model × V = 288 × 8192              2,359,296
========================================================================================
TOTAL MODELO (Pesos Desatados)                                                10,031,328 (10.03M)
TOTAL MODELO (Si se atan Embed y Head)                                         7,672,032 (7.67M)
```

### 1.2 Formato de Pesos y Presupuesto de Memoria
* **Procesador Host:** Intel Core i5-9400F (sin soporte AVX-512 BF16 nativo).
* **Formato de Pesos:** **`float16`** (~20.06 MB) o **`float32`** (~40.13 MB). Se descarta `bfloat16`.
* **Presupuesto Real de RAM en Inferencia:** **$< 80\text{ MB}$** de memoria física residente (RSS) activa con KV-cache.
* **Tolerancia tras Exportar a FP16:**
  $$\frac{|\text{PPL}_{\text{fp16}} - \text{PPL}_{\text{fp32}}|}{\text{PPL}_{\text{fp32}}} < 0.5\%$$

---

## 2. Datos y Estrategia de Entrenamiento Realista

### 2.1 Corpus de Datos (TinyStories)
* **Fuente:** Subconjunto limpio de **TinyStories** desde Hugging Face (`roneneldan/TinyStories`).
* **Volumen:** **120 MB a 150 MB** de texto plano en inglés sintético estructurado (~30M a 40M tokens).
* **Naturaleza del Modelo:** Completador probabilístico de texto/historias (no chatbot conversacional). Solo tokens `<|bos|>`, `<|eos|>` y `<|pad|>`.

### 2.2 Hiperparámetros de Entrenamiento
* **Optimizador:** AdamW ($\beta_1 = 0.9, \beta_2 = 0.95, \text{weight\_decay} = 0.1$).
* **Learning Rate:** Pico de **$5 \times 10^{-4}$** con warm-up de 500 pasos y decaimiento coseno (*Cosine Decay*) hasta $5 \times 10^{-5}$.
* **Batch Size:** 32 (longitud de secuencia = 256 tokens $\implies 8,192$ tokens por paso).
* **Gradient Clipping:** 1.0.

### 2.3 Throughput y Estrategia de Cómputo (CPU Local vs Cloud Bursting)
* **Throughput Estimado en CPU:** 1,500 a 3,500 tokens/s en los 6 núcleos del i5-9400F.
* **La Realidad de 1 Época:** 1 época sobre 35M tokens (ratio $\sim 3.5:1$) toma $\sim 3.5-4$ horas en CPU, pero suele producir un modelo apenas semi-coherente.
* **Criterio de Escalado de Entrenamiento:**
  - **Plan A (100% Local en CPU):** Entrenar 3 a 4 épocas completas (~100M a 140M tokens vistos). Tiempo de cómputo estimado: **12 a 16 horas** (puede dejarse corriendo una noche).
  - **Plan B (Cloud Bursting gratuito):** Si se desea iterar en minutos y no horas, ejecutar el mismo script `train.py` en un notebook gratuito de Google Colab / Kaggle con GPU T4 (~15-20k tokens/s, completando 4 épocas en $<2$ horas) y descargar los pesos resultantes.

---

## 3. Hoja de Ruta de Fases de Ingeniería

### Fase 0: Datos, Pre-tokenizador Portable y BPE (8k)
* **Objetivo:** Adquirir TinyStories y entrenar un tokenizador BPE con pre-tokenizador sin lookarounds incompatibles con Rust.
* **Entregables:**
  1. `tools/prepare_dataset.py`: Descarga y partición 95% train / 5% val.
  2. `tools/train_bpe.py`: Tokenizer BPE de 8,192 tokens con reglas de división portables (whitespace + punctuation splitting estándar).
  3. Archivos `tokenizer.json` y `vocab.json`.
* **DoD:** Tokenizer entrenado con compresión $<4.0$ bytes/token sobre el conjunto de validación.

---

### Fase 1: Arquitectura y Verificacion Tensorial Rigurosa
* **Objetivo:** Validar la arquitectura matematicamente contra especificacion formal con paridad absoluta.
* **Configuracion Exacta del Modelo:**
  - Instanciar `RelicForCausalLM` con:
    - `vocab_size = 8192`
    - `hidden_size = 288`
    - `intermediate_size = 768`
    - `num_hidden_layers = 6`
    - `num_attention_heads = 6`
    - `num_key_value_heads = 2`
    - `head_dim = 48`
    - `max_position_embeddings = 512`
    - `rope_theta = 10000.0`
    - `sliding_window = None` (sin ventana deslizante)
    - `attention_bias = False`
    - `tie_word_embeddings = False`
    - `rms_norm_eps = 1e-6`
* **Verificacion:** Validar invariantes matematicas de dimensiones (10,031,328 parametros), comportamiento autoregresivo y paridad tensorial estricta entre implementacion de referencia y runtime Rust.
* **DoD:** Conteo exacto de 10,031,328 parametros y divergencia de tensores $\max |z_{\text{rust}} - z_{\text{ref}}| < 10^{-5}$.

---

### Fase 2: Medición de 100 Pasos y Entrenamiento con Métricas Claras
* **Objetivo:** Calibrar throughput real y entrenar con objetivos cuantitativos verificables.
* **Métrica Base:** La pérdida por puro azar es:
  $$\mathcal{L}_{\text{azar}} = \ln(8192) \approx 9.01$$
* **Protocolo y Criterio Numérico de Éxito:**
  1. **Dry-run de 100 pasos:** Medir tokens/s reales y estabilidad térmica en CPU.
  2. **Objetivo Numérico de Pérdida:**
     - Pérdida de validación final: **$\mathcal{L}_{\text{val}} \le 2.20$** (equivalente a perplejidad $\text{PPL} \le 9.0$).
     - Si tras 1 época la pérdida queda en $\sim 2.8 - 3.2$ (texto fragmentado), se activan 3 épocas adicionales en local (12-16 h) o en Colab T4.
* **DoD:** Checkpoint guardado con $\mathcal{L}_{\text{val}} \le 2.20$ y registro de la curva de aprendizaje en `reports/training_metrics.json`.

---

### Fase 3: Exportación y Validación de Tokenizer en Rust
* **Objetivo:** Congelar pesos en Safetensors y portar el tokenizador a Rust con paridad absoluta.
* **Entregables:**
  1. `export_safetensors.py`: Exporta a `relic_10m.safetensors` en FP16 (~20.06 MB).
  2. Módulo de tokenizer en Rust dentro de `crates/relic_tokenizer`.
* **DoD:**
  1. Pérdida de perplejidad relativa tras exportación a FP16 $< 0.5\%$.
  2. **Paridad Total de IDs:** El tokenizador en Rust produce **exactamente los mismos token IDs** que el de Python sobre el **100% del dataset de validación**.

---

### Fase 4: Motor de Inferencia en Rust Puro (Multiplataforma)
* **Objetivo:** Runtime autónomo en Rust puro (`memmap2`, KV-Cache dinámico) sin dependencias externas, con límite estricto de contexto a 256 tokens para evitar degradación.
* **Entregables:**
  1. Crate `relic_core`:
     - Cargador `memmap2` para Safetensors.
     - Operadores: RMSNorm, RoPE ($\theta=10000$), QK-Norm, GQA, SwiGLU.
     - KV-Cache dinámico preasignado para $T \le 256$.
     - Generador determinista Greedy y muestreador por temperatura.
* **Criterios de Verificación Rigurosos (DoD):**
  1. **Paridad de Logits en Prefill:** Para un prompt de 64 tokens, los logits de **todas las 64 posiciones** coinciden con PyTorch con $\max |z_{\text{rust}} - z_{\text{torch}}| < 10^{-4}$.
  2. **Paridad de Logits en Decode:** Tras el prefill, durante los siguientes **32 pasos de generación autoregresiva con KV-cache**, los logits producidos en cada paso coinciden con PyTorch con $|z_{\text{rust}} - z_{\text{torch}}| < 10^{-4}$.
  3. **Generación Determinista Idéntica:** Greedy search produce exactamente la misma secuencia de tokens que PyTorch en **50 prompts de prueba**.
  4. **Meta de Velocidad en Decode:** La tasa de generación en Rust debe **igualar o superar a PyTorch CPU** en modo decode autoregresivo ($\ge 1.0\times$ PyTorch tok/s en CPU).
  5. **Memoria:** Consumo físico real RSS $< 80\text{ MB}$.

---

### Fase 5: Documentación, Benchmarks y CI Multiplataforma
* **Objetivo:** Repositorio público impecable como evidencia de portafolio profesional.
* **Entregables:**
  1. Gráfica real de pérdida y muestras de texto generadas.
  2. Benchmark comparativo: tokens/s en PyTorch CPU vs Rust CPU.
  3. CI en GitHub Actions con matriz multiplataforma (`ubuntu-latest` y `windows-latest`).
* **DoD:** Repositorio en GitHub listo, reproducible con `cargo test` y `cargo run` en cualquier máquina sin configurar nada especial.

---

## 4. Cronograma Estimado (6 a 7 Semanas)

| Periodo | Objetivo Principal |
| :--- | :--- |
| **Semanas 1 y 2** | Fase 0 (Dataset TinyStories + BPE) y Fase 1 (Arquitectura RELIC-10M verified). |
| **Semana 3** | Fase 2 (Dry-run de 100 pasos + Entrenamiento de 1 época o multi-época). |
| **Semanas 4 y 5** | Fase 3 (Exportación FP16 + Tokenizer en Rust verificado al 100% en val). |
| **Semana 6** | Fase 4 (Motor de inferencia en Rust multiplataforma + paridad completa en prefill/decode/50 prompts). |
| **Semana 7** | Fase 5 (Métricas de latencia y RAM RSS, documentación y CI dual Windows/Linux). |
