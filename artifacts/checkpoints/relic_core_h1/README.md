# Núcleo RELIC H1: Congelado como Estándar de Referencia
**Estado:** CONGELADO / INMUTABLE
**Fecha:** 21 de Octubre de 2026

Este directorio contiene la especificación formal y las métricas certificadas del núcleo recurrente compacto de RELIC (Hipótesis 1). Cualquier evolución o módulo adicional (cómputo iterativo, memoria episódica, routing) debe compararse contra esta línea base.

---

## 1. Especificación Arquitectónica
- **Archivo fuente:** `python/relic/core_h1.py`
- **Vocabulario:** 128 tokens
- **Dimensión oculta ($d$):** 64
- **Capas ($L$):** 2
- **Dimensión SwiGLU ($d_{\text{ff}}$):** 160
- **Mecanismo de secuencia:** `CompactGatedRecurrence` ($O(1)$ espacio, sin caché $O(T^2)$)
- **Normalización:** `RMSNorm`
- **Total Parámetros:** 102,720 (weight-tied embeddings/head)

---

## 2. Métricas de Referencia Certificadas
| Dimensión Evaluada | Métrica Medida | Estado de Aceptación |
| :--- | :---: | :---: |
| **Dyck-$k$ Familiar (Val ID)** | **100.00%** | Aprobado |
| **Dyck-$k$ Extrapolación (Test OOD prof. 5–8)** | **100.00%** | Aprobado (Supera al Transformer Base: 90.67%) |
| **Throughput CPU** | **23,174 tok/s** | +56% más rápido que Transformer (14,845 tok/s) |
| **Pico Memoria RAM (RSS)** | **321.8 MB** | Menor consumo (-6.9 MB vs Transformer, -62.8 MB vs H3) |
| **FLOPs analíticos / pasada (T=26)** | **5,363,072** | -6% FLOPs vs Transformer Base |

---

## 3. Limitaciones Documentadas a Superar
- **Graph Reachability OOD:** 55.00% (nivel de azar binario en caminos $\ge 4$).
- **Rule Induction OOD:** 0.00% (incapacidad de inferir reglas compuestas en una sola pasada feedforward).
- **Causa raíz:** Profundidad algorítmica fija ($L=2$ operaciones lineales consecutivas). Requiere desacoplar el tiempo de cómputo en inferencia mediante bucles de refinamiento (*cómputo iterativo*).
