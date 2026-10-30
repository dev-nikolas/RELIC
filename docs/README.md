# Índice de Documentación de RELIC

Bienvenido a la documentación de **RELIC**. Esta guía organiza los documentos técnicos y reportes de auditoría según lo que necesites consultar.

---

## 🗺️ Mapa de Navegación Rápida

| Si buscas... | Documento Recomendado | Descripción |
| :--- | :--- | :--- |
| **Visión General y Uso Rápido** | [`README.md`](../README.md) | Qué es RELIC, para qué sirve, ejemplos de terminal y licencias. |
| **Arquitectura del Sistema** | [`ARCHITECTURE.md`](ARCHITECTURE.md) | Estructura de anillos (`relic_core`, `relic_blackwall`, `relic_lexicon`) y flujo de transacciones. |
| **Telemetría y Detección (Seis Ojos)** | [`../INFORME_INTEGRACION_ESTRICTA_SEIS_OJOS_RELIC.md`](../INFORME_INTEGRACION_ESTRICTA_SEIS_OJOS_RELIC.md) | Benchmark empírico sobre NAB, ablaciones, falsas alarmas y métricas de latencia. |
| **Guía Simple de Seis Ojos** | [`../GUIA_SIMPLE_SEIS_OJOS_RELIC.md`](../GUIA_SIMPLE_SEIS_OJOS_RELIC.md) | Explicación intuitiva sin tecnicismos del motor de anomalías. |
| **Comparativa de Benchmarks** | [`BENCHMARKS.md`](BENCHMARKS.md) | Tiempos de ejecución, exactitud frente a baselines y evaluación de memoria. |
| **Integración con Agentes (MCP)** | [`DISENO_SANDBOX_MCP.md`](DISENO_SANDBOX_MCP.md) | Especificación del servidor Model Context Protocol sobre `stdio`. |
| **Hoja de Ruta y Planes** | [`../PLAN.md`](../PLAN.md) | Plan de desarrollo activo e hitos técnicos. |

---

## 📂 Archivos de Auditoría e Investigación (Historial Técnico)

Los siguientes documentos recogen el proceso de investigación rigurosa, auditorías forenses y falsación de hipótesis del proyecto:

- [`AUDITORIA_FUNCIONAL.md`](AUDITORIA_FUNCIONAL.md): Evaluación funcional profunda del prototipo inicial y limitaciones encontradas.
- [`AUDITORIA_GENERAL.md`](AUDITORIA_GENERAL.md): Auditoría completa del código fuente, dependencias y reglas de diseño.
- [`CORRECCIONES_AUDITORIA.md`](CORRECCIONES_AUDITORIA.md): Bitácora de correcciones técnicas aplicadas tras las auditorías.
- [`PROTOCOL_H1_RIR.md`](PROTOCOL_H1_RIR.md): Protocolo científico y veredicto de cierre sobre la cabeza aprendida de lenguaje natural (H1).
- [`NEURAL_COMPILER_REPORT.md`](NEURAL_COMPILER_REPORT.md): Reporte de evaluación del compilador neuronal y lecciones aprendidas.
- [`SANDBOX_LAB.md`](SANDBOX_LAB.md): Especificaciones del entorno de experimentación y aislamiento transaccional.
