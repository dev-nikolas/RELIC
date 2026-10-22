#!/usr/bin/env python3
"""Verificación OBLIGATORIA de paridad numérica: Compara logits de PyTorch vs Rust.
Si la diferencia máxima excede la tolerancia numérica FP16, el pipeline falla.
Menos de 180 líneas.
"""

import json
import subprocess
import sys
from pathlib import Path
import numpy as np
import torch
from safetensors.torch import load_file

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic import RelicConfig, RelicForCausalLM


def verify_parity() -> None:
    weights_path = ROOT_DIR / "artifacts" / "weights" / "relic_10m_fp16.safetensors"
    if not weights_path.exists():
        sys.exit(f"[ERROR] No se encontró {weights_path}. Ejecuta primero tools/export_safetensors.py")

    config = RelicConfig()
    model = RelicForCausalLM(config)
    state_dict = load_file(str(weights_path))

    # Convertir a FP32 para inferencia de referencia en PyTorch
    state_dict_fp32 = {k: v.to(torch.float32) for k, v in state_dict.items()}
    model.load_state_dict(state_dict_fp32)
    model.eval()

    cli_exe = ROOT_DIR / "target" / "debug" / "relic_cli.exe"
    if not cli_exe.exists():
        cli_exe = ROOT_DIR / "target" / "release" / "relic_cli.exe"
    if not cli_exe.exists():
        subprocess.run(["cargo", "build", "-p", "relic_cli"], cwd=str(ROOT_DIR), check=True)
        cli_exe = ROOT_DIR / "target" / "debug" / "relic_cli.exe"

    test_sequences = [
        [12, 45, 99],
        [100, 200, 300, 400],
        [1, 5, 12, 24, 48],
    ]

    max_tolerance = 0.08  # Tolerancia numérica por cuantización FP16 en CPU
    overall_max_diff = 0.0

    print("=" * 60)
    print("  RELIC - Verificacion de Paridad Numerica PyTorch <-> Rust")
    print("=" * 60)

    for idx, seq in enumerate(test_sequences, 1):
        # 1. Inferencia PyTorch
        x = torch.tensor([seq], dtype=torch.long)
        with torch.no_grad():
            py_logits = model(x)[0, -1, :].numpy()

        # 2. Inferencia Motor Rust
        tokens_arg = ",".join(map(str, seq))
        res = subprocess.run([str(cli_exe), "parity", tokens_arg], cwd=str(ROOT_DIR), capture_output=True, text=True, check=True)
        rust_logits = np.array(json.loads(res.stdout.strip()), dtype=np.float32)

        # 3. Comparación numérica
        diff = np.abs(py_logits - rust_logits)
        max_diff = float(np.max(diff))
        mean_diff = float(np.mean(diff))
        overall_max_diff = max(overall_max_diff, max_diff)

        print(f"Test {idx} (len={len(seq)}): Max Diff = {max_diff:.6f} | Mean Diff = {mean_diff:.6f}")
        if max_diff > max_tolerance:
            print(f"[FAIL] Diferencia máxima {max_diff:.6f} supera la tolerancia de {max_tolerance}!")
            sys.exit(1)

    print("-" * 60)
    print(f"Resultado: PARIDAD EXITOSA (Diferencia máxima global: {overall_max_diff:.6f})")
    print("=" * 60)


if __name__ == "__main__":
    verify_parity()
