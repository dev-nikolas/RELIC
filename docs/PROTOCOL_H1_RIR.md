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

## ETAPA 2 EJECUTADA (2026-10-22) — veredicto con criterios de la congelación

Pesos del H1 congelado NO existen en el repo [MEDIDO]: se RECONSTRUYO la misma
arquitectura (102,720 params) con la receta certificada en dyck: 100%/100% a 30
epocas, sha256 de pesos 7448feabbb50d19d909798bdcd58abaa6deb6bc25bc8acf458bc7c37e84b0217
[RECONSTRUIDO, no citable como congelado]. Resultados (tupla exacta intent+
domain+status sobre 95 congelados, mediana de 3 semillas [MEDIDO]):
- A1 solo-latente-H1: 26.3/30.5/30.5 -> mediana 30.5% => RECHAZO.
  El estado interno del core ( entrenado en dyck) NO posee semantica de
  lenguaje natural utilizable por si solo (acuerdo con la advertencia previa:
  Dyck no garantiza semantica NL).
- A2 latent-H1 + char-bigrams: 66.3/74.7/73.7 -> mediana 73.7% => ZONA VIVA
  (>= 59.7% por congelacion). Comparacion justa: baseline char-bigram puro
  3 semillas 54.7/18.9/44.2, mediana 44.2% y TODAS las semillas de A2 >= 66.3
  => el aporte del latente H1 es real y estable (+29.5 pp de mediana).
VEREDICTO: el puente H1->RIR SOBREVIVE solo en su version HIBRIDA (A2).
La etapa 3 (integrar cabeza en el despacho Rust, ejecutar y comprobar que no
regresa unseen_frozen_v2 y known_failures_v2) queda definida con este cerco.

## CAMINO B (auditoria de robustez, paralelo e independiente)

B1 - los 30 casos 'leaks' [MEDIDO, caracterizacion]: son DUPLICADOS IDENTICOS
(Jaccard = 1.0, texto exacto), 25/30 de la familia F6_inverted_clause_val.
Implicacion dura: el dataset semantic_compiler original tenia copias literales
del train dentro de test_id/test_ood; CUALQUIER metrica [HISTORICA] del
compilador semantico producida con ese dataset esta inflada por memorizacion
literal, no por generalizacion. Quedan en reports/eval/
prot_h1rir_leaks_analisis.json y en el sha congelado; se reportan como
desglose, nunca se silencian.

B2 - re-auditoria de suites congeladas con el binario post-numerales
[medicion append-only en analisis_consolidado.json, 9 corridas]:
- unseen_frozen_v2: 14 correcto + 2 abstenciones = 100% extendida, SIN retardos;
  los dos cambios (oro v2 corregido y numerales) NO degradan el congelado.
- probe de abstencion: 27/30 correctas + 2 VALID correctos, idem pre-cambio.
Determinismo 100% en todas las corridas de B2.

## ETAPA 3 (2026-10-22) — aislada en tools, despacho y ejecutor intactos

Notas de definicion, fijadas ANTES de mirar resultados:
- El "100% congelado" invocado en el repo tiempo atras es UNICAMENTE: suite no
  congelada unseen_frozen_v2 de 16 casos = 14 correctos + 2 abstenciones de oro.
  NO es una afirmacion global de precision del sistema y queda definido asi.
- El puente NO modifica produccion: corre como ejemplo aislado
  (crates/relic_cli/examples/bridge_dump.rs) que SOLO elige entre caminos
  existentes; textos de abstencion tomados del engine (0 frases nuevas);
  sin inventar estructuras (0 aceptaciones falsas exigible).
- concurrentemente se documenta: la ventaja del puente en la MEDICION DE ANALIZA
  no se evalua si no llega al flujo completo: se mide aqui exactamente eso.

Resultados por semilla [MEDIDO], reports/eval/prot_h1rir_etapa3_resultados.json:
- cobertura ejecutable: 20.0% (mediana; 17.9-20.0)
- exactitud QuerySpec de lo ejecutado: 20.0% mediana; validez RIR 100%
- abstencion correcta: 8/8 x3 semillas; aceptacion_falsa: 0/95 x3
- falso_rechazo: 68-70 (oro VALID sin plan extraible por el parseador existente)
- referencia despacho [MEDIDO]: cobertura_rir 48.4%, concordancia 20.0%
VEREDICTO: la mejora de A2 (73.7% vs baseline 44.2% de mediana en la metrica
de cabeza) NO llega al flujo completo: queda en 20.0% mediana, igual que el
despacho, y por debajo de su cobertura ejecutando el plan - por defecto se
abstiene con-validation. El bloqueo [MEDIDO] esta en el decodificador
sintactico: parse_inventory_problem NO extrae la clausula invertida F6
('De los N libros que habia comprado, presto M') de la que Test 30/95
son duplicados del train; la cabeza clasifica VALID correctamente y el plan no
se puede generar. ABIERTO al final de la etapa 3 con nueva congelacion (abajo, ETAPA 4).

## ETAPA 4 (autorizada 2026-10-22) — CONGELACION previa a la implementacion

Alcance: UNICAMENTE extractor de clausula invertida (nuevo modulo) + su
integracion como fallback de parse_inventory_problem. Prohibido tocar modelo,
despacho de intents, ejecutor, suites congeladas previas ni el oro de etapa 3.
Estado actual: commit cc08220 es la referencia cerrada; veredicto etapa 3 NO se
reabre ni se re-mide con criterios retrospechos.

ORO congelado ANTES del codigo (tests/f6_inverted_oro_v1.json): 25 casos F6
copiados del split sha d60eb8bb SIN retoques + 12 held-out NUEVAS (7 VALID
estructuralmente distintas de F6: operacion-primero, multi-op, numerales
escritos, 'del paquete de N', comienzo posesivo + 5 ABSTAIN). Prueba Rust
(inverted_extractor_frozen_suite): 3/3 ROJO antes de implementar (evidencia).

Definicion operacional (fijada ANTES de medir):
- El "3.3x" mencionado antes de esta seccion NO es meta de etapa 4: el techo
  [MEDIDO] del solo-extractor es 52/95 = 54.7% e2e (19 ejecutables actuales
  + F6 25 + F7_inverted_clauses 8), porque F9-lexicon (15 palabras fuera del
  lexico actual) y F7_passive requieren lexicos adicionales FUERA DE ALCANCE.
  Alcanzar 66% (3.3x) exigiria tocar el lexico: protocolo aparte si procede.
- GANAR la etapa 4 (todos obligatorios):
  G1 factor_e2e = exactitud QuerySpec e2e mediana con extractor / 20.0%
     (mediana de etapa 3, MISMOS preds congelados) >= 2.0 (>= 40.0%).
  G2 bloque F6: >= 23/25 exactos (QuerySpec completa).
  G3 held-out: 7/7 VALID exactos y 5/5 abstenciones (extractor general,
     no receta-only-F6).
  G4 salvaguardas: 0 aceptaciones falsas e2e; validez RIR 100% de lo
     ejecutado; abstencion correcta 8/8 del split; suites congeladas verdes.
  G5 no-regresion: los 19 correctos de etapa 3 siguen correctos.
- STOP (cualquiera): >= 1 aceptacion falsa; F6 mejora pero held-out no;
  cualquier test congelado rojo; factor < 1.5.
- Comparacion contra despacho: mismos preds, misma particion; reporte con
  exactitud e2e, cobertura y desglose F6/no-F6 SEPARADOS. El exito en F6
  NUNCA se presenta como generalizacion (por eso existe G3).

Estado: PENDIENTE DE EJECUTAR.

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



