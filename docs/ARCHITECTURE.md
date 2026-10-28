# Arquitectura Técnica de RELIC (Estado de Producción)

Este documento describe la arquitectura implementada y formalmente verificada en el código fuente de **RELIC**.

---

## 1. Separación de Capas: Núcleo Inmutable vs. Engrama Transaccional

RELIC divide estrictamente su ejecución en tres anillos concéntricos:

1. **Anillo 0 — Núcleo Inmutable (`relic_core` + `relic_cli` base):**
   - Contiene los algoritmos matemáticos y lógicos fundamentales:
     - Evaluador aritmético **Shunting-Yard** con registro de reducciones en pila RPN (`ArithmeticReduction`).
     - Operadores proporcionales nativos (`"la mitad de"`, `"un tercio de"`, `"el doble de"`), variaciones porcentuales (`"X% de Y"`, `"Y más/menos el X%"`) y reparto con cálculo de cociente y resto entero.
     - Motor de alcanzabilidad en grafos por búsqueda en anchura con restricciones (`solve_reachability_bfs_constrained`).
     - Motor de orden parcial y transitividad mediante el algoritmo de **Kahn** con detección de ciclos (`TemporalGraph`).
     - Motor de **Activación Propagada (`propagate_activation`)** con entropía de Shannon $H = -\sum p_i \log_2 p_i$ y compuerta de abstención por incertidumbre.
     - Motor de **Sincronización Espontánea de Kuramoto (`kuramoto.rs`)**: Integración Runge-Kutta de 4to orden (RK4) para osciladores acoplados, cálculo del parámetro de orden complejo $r e^{i\psi}$ y abstención física cuando $r < r^*$.
     - Motor de **Transformada Discreta de Fourier y Teorema de Incertidumbre de Donoho-Stark (`spectral.rs`)**.
     - Validador semántico de la Representación Intermedia de Razonamiento (**RIR v1**).
     - Implementación nativa de **SHA-256 (FIPS 180-4)** sin dependencias externas.

2. **Anillo 1 — Frontera Transaccional y Asimilación (`relic_blackwall`):**
   - `BlackwallGate` encapsula de forma privada el `EngramStore`. Ningún componente HTTP ni MCP posee acceso directo de escritura a la memoria episódica.
   - Toda propuesta de mutación (`LexiconPayload`) atraviesa tres compuertas deterministas:
     1. **Oráculo (`oracle.rs`):** Bloquea la alteración de palabras reservadas del núcleo (`RESERVED_CORE_TOKENS`), identificadores vacíos, pesos no positivos/no finitos y auto-ciclos reflexivos (`u -> u`).
     2. **Sandbox Efímero y Regresión (`regression.rs`):** Clona el estado en RAM, aplica la mutación candidata y ejecuta la suite de regresión canónica más los casos del usuario. Si un solo caso falla, rechaza atómicamente (`TransactionOutcome::RejectedByRegression`).
     3. **Commit / Rollback / Consolidación (`transaction.rs`):** Si supera el 100% de los tests, sella la entrada con su hash SHA-256 encadenado y persiste en la bitácora JSONL.
   - **Ciclo de Asimilación Autónoma (`assimilation.rs`):** Permite el ciclo de "Shadow Distillation" donde consultas incompletas son asimiladas como nuevos engramas solo si no generan colisiones, calibrando fases con `auto_calibrate_phases`.

3. **Anillo 2 — Engrama Versionado y Cápsulas Portátiles (`relic_lexicon`):**
   - Bitácora *append-only* (`LogEntry`) encadenada criptográficamente desde `GENESIS_PREV_HASH` (`64` ceros).
   - Verificación de integridad en $O(N)$ al iniciar (`EngramStore::from_jsonl_reader`). Cualquier alteración de un carácter emite `Err(IntegrityError::HashMismatch)`.
   - **Auto-Calibrador de Fase (`calibrator.rs`):** Asigna dinámicamente fases constructivas ($\theta = 0$) para sinónimos y destructivas ($\theta = \pi$) para antónimos/exclusiones basándose en evidencia observada, eliminando ángulos hardcodeados.
   - **Cápsulas de Engrama Portátiles (`capsule.rs`):** Paquetes `.engram` serializados con firma criptográfica de cabecera SHA-256 (`merkle_seal_sha256`) para importación/exportación segura entre instancias.

---

## 2. Diagrama del Ciclo de Vida del Engrama

```mermaid
sequenceDiagram
    participant U as Cliente (CLI / HTTP / MCP)
    participant BW as BlackwallGate
    participant OR as Oráculo (oracle.rs)
    participant SB as Sandbox + Regresión
    participant LOG as EngramStore (JSONL SHA-256)

    U->>BW: propose_mutation(payload, ts)
    BW->>OR: validate_proposal_oracle(payload)
    alt Violación de Invariante Nuclear
        OR-->>BW: Err(OracleViolation)
        BW-->>U: RejectedByOracle
    else Oráculo OK
        BW->>SB: Clonar EngramStore en RAM + append_entry
        SB->>SB: run_regression_suite(canonical + custom)
        alt Falla >= 1 caso de regresión
            SB-->>BW: Err(RegressionFailure)
            BW-->>U: RejectedByRegression { failed_case, reason }
        else 100% Regresión en Verde
            BW->>LOG: append_entry (Sello SHA-256 + Flush JSONL)
            LOG-->>BW: LogEntry { version, entry_hash }
            BW-->>U: Committed { version, entry_hash }
        end
    end
```

---

## 3. Infraestructura de Evaluación Científica (`relic_benchmarks`)

El espacio de trabajo incorpora el crate independiente [`relic_benchmarks`](benchmarks/), ejecutable vía `cargo run -p relic_benchmarks`. Evalúa:
- **Ablación Cuántica vs. Clásica:** Anulación exacta por interferencia destructiva de fase.
- **Abstención Epistémica:** Cero invención ante trampas de razonamiento.
- **Auditoría Espectral de Telemetría:** Detección de balizas C2 y aislamiento topológico.
- **ContinualEditStress:** 200 mutaciones continuas con 0% olvido catastrófico y CLUTRR multi-hop.
- **Transición de Fase de Kuramoto:** Bifurcación de segundo orden $r$ vs. $K$ y abstención subcrítica.
- **Jitter Estocástico:** Invariancia espectral y teorema de Donoho-Stark frente a perturbaciones temporales.
