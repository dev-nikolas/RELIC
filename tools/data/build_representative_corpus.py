#!/usr/bin/env python3
"""Generador de Corpus Representativo para RELIC (Español + Razonamiento + Código).
Construye particiones balanceadas en data/train.txt y data/val.txt asegurando
cobertura léxica completa de diacríticos, signos de apertura, lógica y sintaxis de código,
con riqueza morfológica suficiente para saturar el vocabulario BPE de 8,192 tokens.
"""

import itertools
import random
import sys
from pathlib import Path

# Resolver imports
sys.path.insert(0, str(Path(__file__).resolve().parent))
try:
    from common import DATA_DIR, STORY_SEP
    from prepare_dataset import clean
except ImportError:
    from tools.common import DATA_DIR, STORY_SEP
    from tools.prepare_dataset import clean

# Semilla determinista
random.seed(42)

# ==============================================================================
# 1. MORFOLOGÍA Y DICCIONARIO GENERATIVO EN ESPAÑOL
# ==============================================================================
PREFIJOS = [
    "", "des", "in", "con", "re", "trans", "sub", "pre", "anti", "inter",
    "sobre", "auto", "multi", "poli", "micro", "macro", "super", "hiper", "co"
]

RAICES = [
    "comput", "form", "estructur", "funcion", "oper", "cre", "escrib", "pens",
    "razon", "decid", "corr", "habl", "busc", "encontr", "mir", "sent", "produc",
    "redu", "cambi", "resolv", "estudi", "analiz", "observ", "verific", "demostr",
    "constru", "optimiz", "secuenci", "relacion", "organiz", "coordin", "articul",
    "sintetiz", "clasific", "cuantific", "cualific", "interpret", "formul", "evalu",
    "registr", "almacen", "transmit", "modific", "rectific", "consolid", "expand",
    "comprim", "codific", "decodific", "sincroniz", "generaliz", "particulariz",
    "dimension", "geometriz", "focaliz", "centraliz", "descentraliz", "estabiliz"
]

SUFIJOS = [
    "able", "ible", "ación", "ición", "ador", "edor", "amiento", "imiento",
    "idad", "itud", "oso", "osa", "mente", "izar", "ificado", "ante", "ente",
    "aria", "ario", "orial", "utivo", "tivo", "tiva", "alidad", "ismo", "ista",
    "ógico", "ógica", "ometría", "ología", "ático", "ática", "iforme"
]

TERMINACIONES_VERBALES = [
    "ar", "er", "ir", "o", "as", "a", "amos", "an", "é", "aste", "ó", "aron",
    "ía", "ías", "íamos", "ían", "ará", "arán", "aría", "arían", "ando", "iendo"
]

SUJETOS_BASE = [
    "El pingüino emperador", "La investigadora de datos", "El arquitecto de sistemas",
    "Una cigüeña blanca", "El ingeniero de compiladores", "La comunidad académica",
    "El filósofo contemporáneo", "La matemática aplicada", "El médico cirujano",
    "Un astronauta veterano", "La programadora de Rust", "El físico cuántico",
    "El botánico experto", "La geóloga exploradora", "El relojero artesanal",
    "Un lingüista computacional", "La jueza constitucional", "El analista financiero",
    "El músico compositor", "La arqueóloga subacuática", "El maestro de escuela",
    "Un piloto comercial", "La química orgánica", "El historiador medieval",
    "El estudiante universitario", "La bióloga molecular", "El administrador de redes"
]

LUGARES_Y_CONTEXTOS = [
    "en Bogotá", "en Medellín", "en Cali", "en Cartagena", "en Bucaramanga",
    "en Manizales", "en Madrid", "en Barcelona", "en Buenos Aires", "en Santiago",
    "en la cordillera de los Andes", "bajo la llovizna otoñal", "en el laboratorio central",
    "durante la conferencia internacional", "en la estación espacial orbital",
    "frente a la pantalla del ordenador", "en el centro de cómputo de alto rendimiento"
]

CONECTORES = [
    "Por consiguiente,", "En consecuencia,", "Sin embargo,", "Asimismo,",
    "A pesar de las dificultades técnicas,", "Desde una perspectiva epistemológica,",
    "Bajo condiciones de carga extrema,", "Con un consumo mínimo de energía,",
    "De forma completamente determinista,", "Siguiendo los principios de Hawkins,"
]

# ==============================================================================
# 2. CORPUS DE RAZONAMIENTO Y RESOLUCIÓN DE PROBLEMAS
# ==============================================================================
REASONING_TEMPLATES = [
    """Problema de Deducción Lógica:
Premisa 1: {premisa_1}
Premisa 2: {premisa_2}
Pregunta: ¿Se deduce válidamente que {conclusion}?
Respuesta y justificación:
Analizamos las proposiciones mediante lógica formal de primer orden.
Dado que la premisa mayor abarca universalmente a todos los elementos del conjunto, y la premisa menor sitúa al sujeto dentro de dicha categoría, la conclusión se sigue de forma necesaria y deductiva. Por lo tanto, la respuesta es afirmativa.""",

    """Resolución Aritmética Paso a Paso:
Enunciado: Un servidor procesa {n1} peticiones por segundo. Durante un pico de tráfico, la carga aumenta un {pct}% durante {mins} minutos. ¿Cuántas peticiones totales atiende en ese intervalo?
Paso 1: Calcular la tasa aumentada:
Tasa pico = {n1} * (1 + {pct}/100) = {tasa_pico:.1f} peticiones/segundo.
Paso 2: Convertir el tiempo a segundos:
Tiempo = {mins} minutos * 60 segundos/minuto = {segs} segundos.
Paso 3: Multiplicar tasa por tiempo:
Total = {tasa_pico:.1f} * {segs} = {total_req:.0f} peticiones.
Conclusión: El servidor atiende aproximadamente {total_req:.0f} peticiones durante el periodo de alta demanda.""",

    """Análisis de Complejidad Algorítmica:
Algoritmo evaluado: {algoritmo} sobre una colección de tamaño N.
1. Complejidad temporal en el mejor caso: {o_best}.
2. Complejidad temporal en el caso promedio: {o_avg}.
3. Complejidad temporal en el peor caso: {o_worst}.
4. Complejidad espacial auxiliar: {o_space}.
Explicación técnica: Debido a que la partición divide el espacio de búsqueda recursivamente a la mitad en cada nivel de profundidad, la altura del árbol de llamadas es logarítmica O(log N). Si cada nivel realiza un trabajo lineal O(N), el coste acumulado es O(N log N)."""
]

# ==============================================================================
# 3. CORPUS DE CÓDIGO (RUST Y PYTHON)
# ==============================================================================
CODE_SNIPPETS = [
    """def busqueda_binaria(arreglo: list[int], objetivo: int) -> int | None:
    \"\"\"Encuentra la posición del elemento objetivo en un arreglo ordenado.\"\"\"
    izquierda = 0
    derecha = len(arreglo) - 1

    while izquierda <= derecha:
        medio = (izquierda + derecha) // 2
        valor = arreglo[medio]

        if valor == objetivo:
            return medio
        elif valor < objetivo:
            izquierda = medio + 1
        else:
            derecha = medio - 1

    return None

# Ejemplo de ejecución
numeros = [3, 7, 14, 25, 38, 49, 62, 75, 88, 91]
resultado = busqueda_binaria(numeros, 38)
assert resultado == 4, f"Error en búsqueda: esperado 4, obtenido {resultado}\"""",

    """use std::collections::BTreeMap;

/// Estructura de almacenamiento ordenado para métricas de latencia.
pub struct MetricRegistry {
    nombre: String,
    conteos: BTreeMap<u64, usize>,
}

impl MetricRegistry {
    pub fn new(nombre: &str) -> Self {
        Self {
            nombre: nombre.to_string(),
            conteos: BTreeMap::new(),
        }
    }

    pub fn registrar(&mut self, latencia_ms: u64) {
        *self.conteos.entry(latencia_ms).or_insert(0) += 1;
    }

    pub fn total_muestras(&self) -> usize {
        self.conteos.values().sum()
    }
}""",

    """async fn procesar_lote_asincrono(mensajes: Vec<String>) -> Result<usize, String> {
    let mut procesados = 0;
    for msg in mensajes.iter() {
        if msg.is_empty() {
            continue;
        }
        // Simulación de pipeline en tiempo real sin asignaciones dinámicas
        procesados += 1;
    }
    Ok(procesados)
}"""
]


def generate_lexical_diversity() -> list[str]:
    """Genera miles de oraciones y párrafos ricos en vocabulario en español."""
    items = []

    # 1. Palabras derivadas y términos técnicos en oraciones con sentido
    for p in PREFIJOS:
        for r in RAICES:
            suf = random.choice(SUFIJOS)
            palabra = f"{p}{r}{suf}" if p else f"{r}{suf}"
            
            sujeto = random.choice(SUJETOS_BASE)
            lugar = random.choice(LUGARES_Y_CONTEXTOS)
            conector = random.choice(CONECTORES)
            
            oracion = (
                f"{sujeto} analizó el fenómeno de {palabra} {lugar}. "
                f"{conector} se logró verificar que la implementación cumple los requisitos de exactitud "
                f"y mantiene un consumo de memoria inferior a cincuenta megabytes sin fugas."
            )
            items.append(oracion)

    # 2. Oraciones de prueba directa y patrones interrogativos/exclamativos
    frases_tipo = [
        "El pingüino encontró una señal en Bogotá mientras examinaba la convergencia de las series de Fourier.",
        "¿Podrías explicarme con rigor cómo funciona la sincronización de fases en sistemas acoplados?",
        "¡Qué magnífica coincidencia! Jamás imaginé encontrar un algoritmo tan óptimo en esta región.",
        "La cigüeña sobrevoló pacíficamente el viñedo de don Ramón, buscando ramitas secas para su nido.",
        "«La ciencia no busca convencer con retórica, sino demostrar mediante falsación experimental y datos reproducibles».",
        "La optimización de registros vectoriales con AVX2 y FMA acelera el producto punto en procesadores modernos.",
        "El búho real observaba desde la copa del castaño con una mirada serena y penetrante.",
        "La decodificación causal con KV-Cache preasignado reduce la latencia de generación a complejidad temporal constante O(1).",
        "Las tildes diacríticas diferencian categorías gramaticales: «él» es pronombre y «el» es artículo determinante.",
        "¿Por qué la memoria física residente (RSS) debe mantenerse plana durante la decodificación autoregresiva?"
    ]
    items.extend(frases_tipo * 20)

    # 3. Razonamiento algorítmico y matemático
    algos = [
        ("MergeSort", "O(N log N)", "O(N log N)", "O(N log N)", "O(N)"),
        ("QuickSort", "O(N log N)", "O(N log N)", "O(N²)", "O(log N)"),
        ("Búsqueda Binaria", "O(1)", "O(log N)", "O(log N)", "O(1)"),
        ("HeapSort", "O(N log N)", "O(N log N)", "O(N log N)", "O(1)"),
        ("Dijkstra", "O((V + E) log V)", "O((V + E) log V)", "O((V + E) log V)", "O(V)"),
        ("Kruskal", "O(E log E)", "O(E log E)", "O(E log E)", "O(V)")
    ]
    for nombre_algo, ob, oa, ow, os in algos:
        items.append(REASONING_TEMPLATES[2].format(
            algoritmo=nombre_algo, o_best=ob, o_avg=oa, o_worst=ow, o_space=os
        ))

    for i in range(1, 30):
        n1 = 250 * i
        pct = 15 + (i * 3)
        mins = 3 + i
        tasa_pico = n1 * (1 + pct / 100.0)
        segs = mins * 60
        total_req = tasa_pico * segs
        items.append(REASONING_TEMPLATES[1].format(
            n1=n1, pct=pct, mins=mins, tasa_pico=tasa_pico, segs=segs, total_req=total_req
        ))

    # 4. Código fuente
    items.extend(CODE_SNIPPETS * 15)

    return items


def build_rich_corpus(out_dir: Path, target_mb: float = 3.5) -> tuple[int, int]:
    """Genera datos de alta diversidad léxica hasta alcanzar aproximadamente target_mb."""
    out_dir.mkdir(parents=True, exist_ok=True)
    samples = generate_lexical_diversity()
    random.shuffle(samples)

    train_file = out_dir / "train.txt"
    val_file = out_dir / "val.txt"

    target_bytes = int(target_mb * 1024 * 1024)
    total_bytes = 0
    kept_train = 0
    kept_val = 0
    val_every = 10

    print(f"Generando corpus de alta diversidad léxica (~{target_mb:.1f} MB)...")
    with open(train_file, "w", encoding="utf-8", newline="\n") as f_train, \
         open(val_file, "w", encoding="utf-8", newline="\n") as f_val:

        sample_cycle = itertools.cycle(samples)
        for idx, text in enumerate(sample_cycle):
            cleaned = clean(text)
            if cleaned is None or len(cleaned) < 15:
                continue

            entry = cleaned + STORY_SEP
            entry_bytes = len(entry.encode("utf-8"))

            if idx % val_every == 0:
                f_val.write(entry)
                kept_val += 1
            else:
                f_train.write(entry)
                kept_train += 1

            total_bytes += entry_bytes
            if total_bytes >= target_bytes:
                break

    train_size = train_file.stat().st_size
    val_size = val_file.stat().st_size
    print(f"[OK] Corpus generado exitosamente:")
    print(f"  Train: {kept_train:,} historias ({train_size / (1024 * 1024):.2f} MB)")
    print(f"  Val:   {kept_val:,} historias ({val_size / (1024 * 1024):.2f} MB)")
    return kept_train, kept_val


if __name__ == "__main__":
    build_rich_corpus(DATA_DIR, target_mb=3.5)
