# Plantilla: conjunto de evaluacion de Terceros (TERCEROS_v1)

Estado: [DESIGN] — plantilla de protocolo. [PENDIENTE DE EJECUTAR] la generacion física.
La regla dura: NADIE del iterativo del repo (agente incluido) puede generar los casos.
Se aceptan unicamente frases escritas por personas externas (u obtenidas de corpus
publicos citables) y se "quema" el archivo en git con su suma sha256 antes de la
primera medicion.

## Formato de archivo final: `tests/terceros_v1.json`

[{"input": "...", "expected_route": null, "expected_output": null,
  "kept_only_for_free_text_graders": false, "source": "<cite|persona>", "note": "..."}]

- NO se permite `expected_*` declarado por el equipo de RELIC: la correccion basica es
  por aguja (needle) acordada APARTIR de la respuesta escrita a mano de un humano externo.
- El archivo se firma: `sha256sum tests/terceros_v1.json` y esa firma queda documentada
  en reports/eval/ antes de correr cualquier harness, y NO se modifica nunca mas.
- Numeral en cifras y en palabras ambos permitidos (el gap de numerales [MEDIDO] en
  seccion 6.2 de docs/ESTADO_Y_ORIENTACION.md es consumo de esta suite).

## Generacion sugerida (50 casos)

- 10 frases aritmeticas (3 formas distintas de decir "sumar" y "restar").
- 10 entregables espaciales ("como ir de X a Y", incluya un viaje con "pasando por" y uno
  con "sin pasar por"). Neryl: se esperan las plantillas del parser de reglas resultados.
- 10 dominio desconocido (loca-georgia, personas, empresas ficticias).
- 10 conversacionales ("Hola", "que haces?", "cuentame").
- 10 objetivos de frase larga (2+ en 1, protegida por composicion).

## Comparacion automatizable (cross-check LLM)

Notebook Colab PENDIENTE (Qwen3-0.6B-Instruct u otro, temp=0, 3 repeticiones, grader por
aguja via exact-match/contains acordado humano). No hay hardware externo del autor; el
notebook es ejecutable por KAGGLE/COLAB y su salida es un CSV appendeable.
