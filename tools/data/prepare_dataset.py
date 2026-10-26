#!/usr/bin/env python3
"""Fase 0 - Paso 1: Descarga un subconjunto de TinyStories, lo limpia a ASCII
y lo parte en train/val (95/5) como texto plano.

Uso:
    python tools/prepare_dataset.py                      # streaming desde Hugging Face
    python tools/prepare_dataset.py --input-file X.txt   # o desde un .txt local de TinyStories
Salida: data/train.txt y data/val.txt (historias separadas por STORY_SEP).
"""

import argparse
import re
import sys
from pathlib import Path

# Permite ejecutar tanto directamente como modulo
sys.path.insert(0, str(Path(__file__).resolve().parent))
try:
    from common import DATA_DIR, STORY_SEP
except ImportError:
    from tools.common import DATA_DIR, STORY_SEP

import unicodedata

# Mapeo de comillas tipográficas raras a comillas legibles estándar
QUOTE_NORMALIZATION = {
    "\u2018": "'", "\u2019": "'", "\u201a": "'", "\u201b": "'",
    "\u201c": '"', "\u201d": '"', "\u201e": '"',
    "\u00a0": " ", "\u2002": " ", "\u2003": " ", "\u2009": " ",
}
_TRANSLATE = str.maketrans(QUOTE_NORMALIZATION)


def clean(text: str) -> str | None:
    """Normaliza una muestra preservando español y código. Descarta corrupción y caracteres invisibles."""
    if not isinstance(text, str):
        return None
    text = text.translate(_TRANSLATE)
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    if "\ufffd" in text or "<|" in text:
        return None

    # Normalizar espaciado preservando sangrías de código
    lines = [line.rstrip() for line in text.split("\n")]
    cleaned_lines = []
    empty_count = 0
    for line in lines:
        if not line:
            empty_count += 1
            if empty_count <= 2:
                cleaned_lines.append("")
        else:
            empty_count = 0
            cleaned_lines.append(line)
    text = "\n".join(cleaned_lines).strip()

    # Validar que no contenga caracteres de control corruptos
    for c in text:
        if c not in ("\n", "\t") and unicodedata.category(c).startswith("C"):
            return None
    return text


def write_splits(stories, out_dir: Path | str, target_bytes: int,
                 val_every: int = 20, min_chars: int = 100) -> tuple[int, int]:
    """Divide las historias procesadas en particiones de train y val."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    seen = kept = total = 0
    with open(out_dir / "train.txt", "w", encoding="utf-8", newline="\n") as ftr, \
         open(out_dir / "val.txt", "w", encoding="utf-8", newline="\n") as fva:
        for raw in stories:
            seen += 1
            text = clean(raw)
            if text is None or len(text) < min_chars:
                continue
            (fva if kept % val_every == 0 else ftr).write(text + STORY_SEP)
            kept += 1
            total += len(text) + len(STORY_SEP)
            if kept % 20000 == 0:
                print(f"  {kept:,} historias, {total / 1e6:.1f} MB", flush=True)
            if total >= target_bytes:
                break
    print(f"Vistas: {seen:,} | guardadas: {kept:,} | descartadas: {seen - kept:,} | {total / 1e6:.1f} MB")
    return kept, total


def stories_from_file(path: str):
    """Generador de historias desde archivo de texto plano."""
    with open(path, "r", encoding="utf-8", newline="\n") as f:
        for chunk in f.read().split("<|endoftext|>"):
            if chunk.strip():
                yield chunk


def stories_from_hf(seed: int, buffer_size: int):
    """Generador de historias con streaming directo desde Hugging Face."""
    try:
        from datasets import load_dataset
    except ImportError:
        sys.exit("Falta 'datasets': pip install datasets")
    ds = load_dataset("roneneldan/TinyStories", split="train", streaming=True)
    ds = ds.shuffle(seed=seed, buffer_size=buffer_size)
    for ex in ds:
        yield ex["text"]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=str(DATA_DIR))
    ap.add_argument("--target-mb", type=float, default=130.0)
    ap.add_argument("--val-frac", type=float, default=0.05)
    ap.add_argument("--seed", type=int, default=1337)
    ap.add_argument("--shuffle-buffer", type=int, default=20000)
    ap.add_argument("--input-file", default=None)
    args = ap.parse_args()

    stories = stories_from_file(args.input_file) if args.input_file \
        else stories_from_hf(args.seed, args.shuffle_buffer)
    write_splits(stories, args.out, int(args.target_mb * 1024 * 1024),
                 val_every=round(1 / args.val_frac))


if __name__ == "__main__":
    main()
