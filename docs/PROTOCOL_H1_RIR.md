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
