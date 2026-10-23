# Protocolo del experimento Puente H1 → RIR (v1, 2026-10-22)

Estado: [DESIGN] — protocolo aprobado por el propietario ANTES de escribir la cabeza.
La hipotesis falsable: una cabeza pequena entrenada sobre H1 congelado aprende
regularidades que mejoran la interpretacion de FRASES NUEVAS y supera a
(a) el parser de reglas actual y (b) una baseline lexica simple. Si no supera a
(b), el puente muere y el proyecto se reposiciona como verificador deterministico.

Condiciones obligatorias (sin excepciones):

1. Participacion de evaluacion congelada ANTES de ver datos: del dataset
   data/semantic_compiler/*.jsonl (453 muestras, familias F1-F9) se extrae un
   split estratificado por familia, firmado con sha256 y commiteado
   (reports/eval/prot_h1rir_test_split.json) antes de cualquier entrenamiento.
   El split NO se usa para elegir hiperparametros: esos viviran tambien congelados.
   Anti-duplicados: las 453 frases se normalizan (minusculas, sin tildes,
   espacios) y se busca por shingles de 3 palabras colisiones; cualquier par con
   similitud >= 0.6 va TODO al split de test o TODO al de train, nunca repartido.

2. Baselines obligatorias en la MISMA particion de test:
   (a) parser de reglas Rust actual via dumps (pipeline completo);
   (b) clasificador lexico trivial (SVM/logistica sobre bigramas de caracteres),
       entrenado solo con el split de train.

3. Metricas a reportar: exactitud de QuerySpec (intent + slots por campo),
   cobertura (frases que producen RIR), abstenciones correctas vs erroneous,
   validez RIR (solo salidas que pasan validacion), y latencia p50/p95.

4. Ejecutor determinista INTACTO: la cabeza solo produce campos del QuerySpec /
   plan aritmetico; toda salida pasa por el mismo validador RIR actual. Ninguna
   inferencia "hasta el final" salta validacion. El comportamiento del motor
   existente debe mantener par/pasada en unseen_frozen_v2 (no debe retroceder).

5. Presupuesto declarado: <= 3 semillas, <= 5 modelos candidatos; si el mejor
   no supera la baseline (b) por >= 5 puntos porcentuales de exactitud QuerySpec
   en el split congelado, se declara RECHAZO del puente y se reposiciona.

Registro de resultados: reports/eval/prot_h1rir_resultados.json con etiqueta
[MEDIDO]/[PENDIENTE DE EJECUTAR] y hardware especifico. Sin hardware, nada de
cifras de GPU externes sin correr.

## Resultados etapa 1 (2026-10-22, CPU local [MEDIDO])

- Particion congelada: test_id + test_ood = 95 casos, sha256
  d60eb8bb39236698f00f5108fa37df2c4b9ca653f8e6cc2c246350024521a96d.
  HALLAZGO: 30/95 (31.6%) eran casi-duplicados de train/val (Jaccard>=0.6):
  el dataset original estaba parcialmente contaminado; el congelado los Lista y
  el entrenamiento de candidatos usa dev SIN leaks (358 muestras).
- Baseline lexica (char-bigram + logistica, HP fijos [DESIGN]):
  intent 70.5%, domain 77.9%, status 62.1%, tupla exacta 54.7%.
- Baseline despacho Rust (mismo congelado): cobertura RIR 48.4% (de los cuales
  solo 20.0% equivalen sematicamente a inventory_update con unidad),
  concordancia de intencion 20.0%.
- Criterio transición a etapa 2: el candidato H1->RIR debe lograr >= 59.7%
  de tupla exacta (baseline + 5 pp) para NO ser rechazado.

## CONGELACION DE CRITERIOS (2026-10-22, ANTES de ejecutar la etapa 2)

Huella del repo en el momento de congelar: commit 1133540 (etapa 1), sha del
split congelado d60eb8bb39236698f00f5108fa37df2c4b9ca653f8e6cc2c246350024521a96d,
baseline char-bigram tupla 54.7% [MEDIDO]. Este texto no se edita mas sin commit
separado documentado.

Zonas de decision, DEFINIDAS EXACTAMENTE (tuple exacta sobre los 95 congelados):
  - ZONA VIVA: >= 59.7% -> el puente continua a etapa 3 (integracion).
  - ZONA GRIS (54.7%..59.7%): ES RECHAZO. Supera la baseline numerica pero el
    margen (<=5 pp) no es distinguible de ruido con n=95 (1 caso = 1.05 pp;
    IC95 binomial ~+/-10 pp). No se declara victoria intermedia. Registro
    conservador: "no supera el criterio".
  - ZONA DE RECHAZO: <= 54.7% -> rechazo inmediato del puente.
Regla operativa: la decision se toma con la MEDIANA de exactitud de tupla entre
3 semillas (42, 123, 999). TODOS los intentos (por semilla, por arquitectura
candidata) quedan en reports/eval/prot_h1rir_intentos.jsonl; prohibido eliminar
registros o correr semillas extra hasta una "que toque mejor" (p-hacking).

Par de candidatos permitidos (sin expancion posterior):
  A1: solo latente H1 congelado (64-d) -> MLP -> intent/domain/status.
  A2: latente H1 congelado (64-d) + char-bigram -> MLP -> intent/domain/status.
Tangibles adicionales (informativos, no deciden): exactitud por campo, por
familia, desglose congelados-limpios vs duplicados-eliminados.


