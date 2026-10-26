# ESPECIFICACIÓN TÉCNICA: CABEZA POINTER / CLASIFICADOR DE SLOTS (RELIC)
**Documento:** Especificación Arquitectural (Solo Diseño — Sin Implementar)  
**Estado:** Propuesta Formal para Sustituir Decodificación Autorregresiva Libre de JSON  
**Fecha:** 2026-10-25  

---

## 1. Justificación Arquitectural

La evaluación empírica de Fase A demostró que un modelo causal libre de 10M generando cadenas JSON crudas sufre de un 100% de rechazo por sintaxis rota en hold-out ($0/50$ EM en resolubles, $p < 10^{-7}$). 

La **Cabeza Pointer / Clasificador de Slots** desacopla la semántica de la sintaxis:
- La red neuronal solo predice una tupla compacta de intenciones, operadores y punteros a los tokens de entrada.
- Un **ensamblador determinista en Rust** construye el documento `RirDocumentV1` canónico.
- **Garantía:** Cero errores de sintaxis JSON por construcción y reducción de latencia de $> 50$ ms a $< 1$ ms en CPU.

---

## 2. Formato de Datos y Representación de Entrada/Salida

### 2.1. Secuencia de Entrada
Para una consulta de $T$ tokens de texto:
$$\mathbf{x} = [x_1, x_2, \dots, x_T]$$
Cada token $x_i$ genera un vector contextual $h_i \in \mathbb{R}^d$ en la última capa oculta del transformer.

### 2.2. Slots de Salida (Tupla Estructurada)
La predicción consta de 5 variables discretas:
1. $\mathbf{y}_{\text{intent}} \in \{\text{InventoryUpdate}, \text{ArithmeticQuery}, \text{EpistemicReject}, \text{Unsupported}\}$
2. $\mathbf{p}_{\text{init}} \in \{0, 1, \dots, T\}$ (Puntero al token de cantidad inicial; $0 = \text{NONE}$)
3. $\mathbf{y}_{\text{op}} \in \{\text{Add}, \text{Subtract}, \text{Multiply}, \text{Divide}, \text{None}\}$
4. $\mathbf{p}_{\text{operand}} \in \{0, 1, \dots, T\}$ (Puntero al token del operando; $0 = \text{NONE}$)
5. $\mathbf{p}_{\text{unit}} \in \{0, 1, \dots, T\}$ (Puntero al sustantivo de la unidad de inventario)

---

## 3. Vocabulario Discreto y Mecanismo de Punteros

### 3.1. Vocabulario de Clasificación de Slots
- **Intenciones (4 clases):** `[INTENT_INVENTORY, INTENT_ARITHMETIC, INTENT_EPISTEMIC_REJECT, INTENT_UNSUPPORTED]`
- **Operadores (5 clases):** `[OP_ADD, OP_SUBTRACT, OP_MULTIPLY, OP_DIVIDE, OP_NONE]`

### 3.2. Mecanismo de Atención Puntero (Pointer Mechanism)
Para predecir el índice del token de la cantidad inicial u operando:
$$u_i^{(s)} = \mathbf{v}_s^\top \tanh(W_h h_i + W_s q_s), \quad i \in \{1, \dots, T\}$$
$$P(\text{pointer}_s = i) = \text{softmax}(u^{(s)})_i$$
donde $q_s$ es un vector de consulta aprendible para el slot $s$, y el índice 0 representa un token especial `[NULL_POINTER]`.

---

## 4. Ensamblador Determinista a RIR en Rust

El módulo `relic_core::assembler` toma la tupla predicha y la traduce a `RirDocumentV1`:
1. **Tratamiento Epistémico:**
   - Si $\mathbf{y}_{\text{intent}} = \text{EpistemicReject}$, emite inmediatamente `RirDocumentV1` con `status: INCOMPLETE`.
   - Si $\mathbf{y}_{\text{intent}} = \text{Unsupported}$, emite `status: UNSUPPORTED`.
2. **Resolución de Punteros a Números:**
   - Extrae los substrings de texto apuntados por $\mathbf{p}_{\text{init}}$ y $\mathbf{p}_{\text{operand}}$.
   - Ejecuta `parse_numeral_or_digit(token_str)` con soporte para dígitos ("10") y palabras ("diez").
3. **Construcción y Validación:**
   - Si los números son válidos, crea la operación aritmética correspondiente (`add`, `subtract`, etc.).
   - Invoca `validate_semantic(&rir_doc)`. Si viola invariantes (división por cero, contradicciones), lo marca `INVALID`.

---

## 5. Métricas de Evaluación

1. **Exactitud de Intención (Intent Acc):** $\%$ de acierto en la clasificación de $\mathbf{y}_{\text{intent}}$.
2. **Exactitud de Operador (Op Acc):** $\%$ de acierto en $\mathbf{y}_{\text{op}}$.
3. **Exactitud de Puntero (Pointer Acc):** $\%$ de punteros que coinciden exactamente con los spans numéricos del texto.
4. **Exactitud de Tupla Completa (Slot Tuple EM):** $\%$ de ejemplos donde los 5 slots coinciden con el gold.
5. **Exact Match RIR Final (EM):** Exactitud sobre la respuesta computada por el motor determinista.
6. **Tasa de Falsa Aceptación (FA):** Proporción de consultas epistémicas donde la red predijo incorrectamente una intención válida ejecutable.

---

## 6. Criterio de Aceptación Inmutable

Para ser promovido a producción en RELIC, el checkpoint de la Cabeza Pointer debe cumplir:
1. **Compuerta de Seguridad:**
   $$\text{Falsa Aceptación}(\text{Pointer}) \le \text{Falsa Aceptación}(\text{Slot-Filler})$$
2. **Superioridad Estadística Pareada (Prueba de McNemar):**
   Evaluado sobre el **Banco de Evaluación Congelado Humano Externo** (`tests/external_human_gold_v1.json`, $N=100$):
   $$\chi^2_{\text{Edwards}} > 3.841 \quad (p < 0.05) \quad \text{con } c > b$$
   frente al baseline del slot-filler gramatical de `relic_cli`.
