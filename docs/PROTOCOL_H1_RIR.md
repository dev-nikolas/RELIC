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

## ETAPA 5 (autorizada 2026-10-22) — protocolo NUEVO, congelado ANTES de ejecutar

Referencias preservadas: veredictos etapa 3 (cc08220) y etapa 4 (a9bbf8f)
no se editan ni re-miden. El extractor queda integrado con el veredicto
formal "NO GANA por G2" tal cual registrado.

1. DEFINICION OPERACIONAL "desembragon consolidate/a" (ejecutor):
   - Entrenamiento DETERMINISTA por semilla {42,123,999}: corrida unica,
     epocas fijas (80), SIN early-stopping por validacion y SIN tuning
     posterior a la primera metrica observada.
   - Pesos circulados se firman sha256 en el registro; reentrenar significa
     NUEVA entrada append-only en prot_h1rir_intentos.jsonl (prohibido
     sobreescribir intentos: p-hacking).
   - El split y los hiperparametros se congelan en commits ANTES de correr.
   - "seal" = ningun cluster de la particion de prueba (los 95 congelados,
     sha d60eb8bb) toca train/val; clausura POR CONSTRUCCION verificada por
     hash y por inclusion-clave en el codigo del split.

2. SPLIT POR CONSTRUCCION, no por frase:
   - Claves de cluster = (family, structure): family del jsonl + estructura
     textual con digitos -> "#" y todo token alfanumerico no-estructural
     -> <w> (lista blanca estructural: de los las del que y pero luego
     despues mas menos tengo hay poseo tenia). Parafasis casi identicas
     caen en el MISMO cluster (los 25 F6 colapsan en 1).
   - Todo cluster que contenga un texto del set congelado 95 (exacto) o cuyo
     structure coincida con el de algo congelado -> cluster SEILLADO: excluido
     de train/val.
   - Train/val = reparto determinista de clustere libres (seed derivada del
     id del cluster, sin mirar resultados): ~90/10 por clusters.
   - Salida: reports/eval/prot_h1rir_etapa5_train.jsonl + _val.jsonl +
     _sha256.json (huellas). Prohibido re-generar el split tras ver metricas.

3. DATOS DE ENTRENAMIENTO especificados:
   - Filas: TODAS las 453 del dataset semantic_compiler (train+val+test_id+
     test_ood como fuente de CABEZA para el 75% no-sealed) - el conjunto
     final de la evaluaecion es SIEMPRE el split congelado 95 (sha d60eb8bb).
   - Etiquetas de cabeza: intent, domain, status presencia en rir (mismas
     clases que etapa 3, sin clase nueva).
   - Caracteristicas: mismas de etapa 3 (latente H1 [RECONSTRUIDO] de 64-d
     + char-bigram). VOCABULARIO char-bigram construido SOLO de textos de
     train/val sealed split (cambio documentado vs etapa 2, que usaba el
     vocab de dev completo).
   - Arquitectura: IDENTICA a A2 (cambiar algo mas introduciria cofundido:
     la etapa 5 atribuye sus resultados a CLEAN-DATA, no a arquitectura;
     documentado para el control comparable - requisito del protocolo).

4. CRITERIOS FRENTE A LA(EJECUTAR) (95 congelados, mediana 3 semillas,
   mismas metricas y casos para la cabeza ANTIGUA (preds congelados v1) y
   la n v2; techo estructural [MEDIDO] = 52/95 = 54.7% + 8 abstenciones):
   - CONTINUA: mediana e2e QuerySpec >= 45.0% (>= 47/95) Y F6 >= 23/25 Y
     0 aceptaciones falsas Y validez RIR 100% Y abstencion >= 7/8 Y
     no-regresion (todos los correctos de etapa 4 de la misma semilla) Y
     suites congeladas verdes Y lat p50 <= 250 us.
   - GRIS (investigar, no asignar victoria): 35.0% <= mediana < 45.0% con
     TODAS las salvaguardas Y F6 >= 18/25.
   - DETENER: mediana < 35.0% O aceptacion falsa >= 1 O test congelado rojo.
   - Latencia y cobertura se reportan SIEMPRE (no son gate).
   - Referencia control v1 (etapa 4, COMMITEADO): 40.0% e2e mediana.
5. NO ACTIVACION: en NINGUN caso la etapa 5 integra el puente completo en
   produccion; хотя siendo GANA, el despliegue requiere una autorizacion
   separada del usuario.

Estado: EJECUTADA (2026-10-22). Resultados [MEDIDO],
reports/eval/prot_h1rir_etapa5_resultados.json:
- Cabeza v2 (split sellado 127 train / 1 val; arquitectura A2 IDENTICA;
  pesos con sha 529e6f7f / 3c71ba45 / 7372da70): tupla exacta mediana
  2.1% (1.1/3.2/2.1). SIN senal de inventario: el seal elimina las familias
  de inventario del train (46/57 clusteres sellados; 0 filas de inventario
  VALID entrenables) por diseño del criterio de no-parafraisis.
- e2e con extractor: exactitud 0.0/1.1/1.1 -> mediana 1.1% -> mediana e2e
  1.1% < 35.0% => DETENER (criterios congelados). 0 aceptaciones falsas,
  abstencion correcta 8/8, validez RIR 100% de lo ejecutado.
VEREDICTO FORMAL: DETENER en etapa 5. La condicion de limpieza rompe la
senal: los 40% e2e de etapa 4 y la tupla 73.7% de etapa 2 dependen de
entrenamiento con parafasis proximas a la evaluaacion defectiva (memorizaa
sib no generaliza). El defecto del artifact (datos) es estructural: el
dataset semantic_compiler noprovee supervision de inventario apartada del
set congelado. El puente H1->RIR NO se activa en produccion (criterio 5
congelado). Proxima hiptesis (fuera de proyecto): requiere dataset con
construcciones de inventario DIVIDIDAS en train por diseni (no derivar del
1152 congelado): datos nuevos ANTES de reconsiderar la cabeza.

## ETAPA 4 (autorizada 2026-10-22) - CONGELACION previa a la implementacion
 (autorizada 2026-10-22) — CONGELACION previa a la implementacion

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

Estado: EJECUTADA (2026-10-22). Resultados con desglose separado F6/no-F6
(reports/eval/prot_h1rir_etapa4_resultados.json, mediana de 3 semillas,
mismos preds congelados de etapa 3 [MEDIDO]):
- exactitud QuerySpec e2e: 40.0% mediana (30.5/44.2/40.0); cobertura = 40.0%.
- factor_e2e: 2.0 mediana (1.52/2.21/2.0) vs etapa 3 (20.0%).
- F6: ok 9/25 (42), 22/25 (123), 20/25 (999) -> mediana 20/25.
- DESCOMPOSICION [MEDIDO] de los F6 sin plan (16/3/5 por semilla): TODOS
  son ajenos al extractor: 16 (seed42) por ABSTENCION de la CABEZA (pred
  status != VALID) y 3 (123)/5 (999) por pred graph_query|VALID que enruta
  a unsupported/parse_dispatch_intent. Nivel PARSEADOR: extractor 25/25 F6
  y 7/7 held-out VALID + 5/5 abstain (prueba Rust 3/3 verde).
- salvaguardas: 0 aceptaciones falsas x3; validez RIR 100%; abstencion
  correcta 8/8 x3; falso rechazo total 58/45/49; suites congeladas verdes.
- no-regresion: 19/19 (42,123) y 17/17 (999) del set de la misma semilla.
VEREDICTO FORMAL SEGUN CRITERIOS CONGELADOS: NO GANA (G2 = 20/25 mediana
< 23/25 exigido). STOP no se activa (0 aceptaciones falsas, factor 2.0 >= 1.5,
tests verdes). Resultado honesto: el EXTRACTOR mismo SÍ soporta el criterio
de generalizacion (G3 al nivel parseador) y NO introduce regresion (G5),
pero el criterio G2 mide el PUENTE COMPLETO y la cabeza congelada lo acota
(abstiene F6 correctos). La parte del bloqueo diagnosticada en etapa 3 queda
RESUELTA y CUBIERTA; el bloque restante es la CAMARA de decision de la cabeza
congelada de etapa 2, que NO se re-entrena ni re-congela aqui (eso requeriria
nueva congelacion segun este protocolo).
Decision de produccion del extractor: se MANTIENE integrado (dominante
debil: soborno 0 falsas, 0 regresion, factor 2.0 e2e) PERO su activacion
completa depende de re-trenar/re-frenar la cabeza: PENDIENTE DE AUTH.

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




