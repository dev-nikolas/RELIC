#!/usr/bin/env python3
"""Fase 0 - Paso 2: Entrena un tokenizador BPE de 8,192 tokens sobre TinyStories
con pre-tokenizador portable (sin lookarounds incompatibles con Rust).

Salidas:
    artifacts/tokenizer/tokenizer.json
    artifacts/tokenizer/vocab.json
    reports/tokenizer_report.json
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any
from tokenizers import Tokenizer, decoders, models, pre_tokenizers, trainers

# Permite resolver imports tanto desde raiz como desde el directorio tools
CURRENT_DIR = Path(__file__).resolve().parent
if str(CURRENT_DIR) not in sys.path:
    sys.path.insert(0, str(CURRENT_DIR))

try:
    from common import (
        DATA_DIR,
        REPORTS_DIR,
        SPECIAL_TOKENS,
        TOKENIZER_DIR,
        VOCAB_SIZE,
    )
except ImportError:
    from tools.common import (
        DATA_DIR,
        REPORTS_DIR,
        SPECIAL_TOKENS,
        TOKENIZER_DIR,
        VOCAB_SIZE,
    )


def create_tokenizer() -> Tokenizer:
    """Construye un tokenizador BPE con pre-tokenizador portable."""
    tokenizer = Tokenizer(models.BPE(byte_fallback=True))
    tokenizer.pre_tokenizer = pre_tokenizers.ByteLevel(
        add_prefix_space=False,
        trim_offsets=False,
        use_regex=True,
    )
    tokenizer.decoder = decoders.ByteLevel()
    return tokenizer


def get_bytelevel_alphabet() -> list[str]:
    """Obtiene el alfabeto inicial de ByteLevel compatible con tipos estaticos."""
    alphabet_fn: Any = getattr(pre_tokenizers.ByteLevel, "alphabet", None)
    if callable(alphabet_fn):
        return list(alphabet_fn())
    return [chr(i) for i in range(256)]


def train_tokenizer(train_path: Path, vocab_size: int = VOCAB_SIZE) -> Tokenizer:
    """Entrena el tokenizador BPE sobre el archivo de texto provisto."""
    tokenizer = create_tokenizer()
    trainer = trainers.BpeTrainer(
        vocab_size=vocab_size,
        special_tokens=SPECIAL_TOKENS,
        initial_alphabet=get_bytelevel_alphabet(),
        show_progress=True,
    )
    print(f"Entrenando BPE (vocab={vocab_size}) desde {train_path}...")
    tokenizer.train(files=[str(train_path)], trainer=trainer)
    return tokenizer


def evaluate_compression(tokenizer: Tokenizer, val_path: Path, max_bytes: int = 2_000_000) -> float:
    """Calcula ratio de compresion en bytes/token sobre el conjunto de validacion."""
    if not val_path.exists():
        return 0.0
    with open(val_path, "r", encoding="utf-8") as f:
        sample_text = f.read(max_bytes)
    total_bytes = len(sample_text.encode("utf-8"))
    enc = tokenizer.encode(sample_text)
    total_tokens = len(enc.ids)
    bytes_per_token = total_bytes / max(1, total_tokens)
    return float(bytes_per_token)


def save_vocab(tokenizer: Tokenizer, out_dir: Path) -> None:
    """Exporta el vocabulario a vocab.json ademas de tokenizer.json."""
    vocab = tokenizer.get_vocab()
    sorted_vocab = dict(sorted(vocab.items(), key=lambda item: item[1]))
    with open(out_dir / "vocab.json", "w", encoding="utf-8") as f:
        json.dump(sorted_vocab, f, indent=2, ensure_ascii=False)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--train-file", default=str(DATA_DIR / "train.txt"))
    ap.add_argument("--val-file", default=str(DATA_DIR / "val.txt"))
    ap.add_argument("--out-dir", default=str(TOKENIZER_DIR))
    ap.add_argument("--vocab-size", type=int, default=VOCAB_SIZE)
    args = ap.parse_args()

    train_path = Path(args.train_file)
    val_path = Path(args.val_file)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    if not train_path.exists():
        sys.exit(f"Error: No existe el archivo de entrenamiento: {train_path}")

    tokenizer = train_tokenizer(train_path, args.vocab_size)
    tok_json_path = out_dir / "tokenizer.json"
    tokenizer.save(str(tok_json_path))
    save_vocab(tokenizer, out_dir)

    bpt = evaluate_compression(tokenizer, val_path)
    dod_met = bool(0.0 < bpt < 4.0)
    report = {
        "vocab_size": tokenizer.get_vocab_size(),
        "target_vocab_size": args.vocab_size,
        "special_tokens": SPECIAL_TOKENS,
        "bytes_per_token_val": round(bpt, 4),
        "dod_met": dod_met,
    }

    report_path = REPORTS_DIR / "tokenizer_report.json"
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)

    print(f"Tokenizador guardado en: {out_dir}")
    print(f"Compresion de validacion: {bpt:.3f} bytes/token (DoD < 4.0: {dod_met})")


if __name__ == "__main__":
    main()
