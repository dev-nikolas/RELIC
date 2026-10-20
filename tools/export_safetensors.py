#!/usr/bin/env python3
"""Fase 3: Exportador de checkpoints PyTorch a formato Safetensors en FP16 y FP32.
Menos de 180 lineas.
"""

import argparse
from pathlib import Path
import torch
from safetensors.torch import save_file

ROOT_DIR = Path(__file__).resolve().parent.parent
WEIGHTS_DIR = ROOT_DIR / "artifacts" / "weights"


def export_checkpoint_to_safetensors(
    ckpt_path: Path,
    out_path: Path,
    fp16: bool = True,
) -> None:
    """Convierte un checkpoint .pt a formato .safetensors."""
    print(f"Cargando checkpoint desde {ckpt_path}...")
    state_dict = torch.load(ckpt_path, map_location="cpu", weights_only=True)

    target_dict = {}
    total_bytes = 0
    for key, tensor in state_dict.items():
        if fp16 and tensor.is_floating_point():
            t_out = tensor.to(torch.float16)
        else:
            t_out = tensor
        target_dict[key] = t_out
        total_bytes += t_out.numel() * t_out.element_size()

    out_path.parent.mkdir(parents=True, exist_ok=True)
    save_file(target_dict, str(out_path))
    size_mb = total_bytes / (1024 * 1024)
    dtype_str = "FP16" if fp16 else "FP32"
    print(f"Exportado exitosamente a {out_path} ({dtype_str}, {size_mb:.2f} MB, {len(target_dict)} tensores)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--ckpt", default=str(ROOT_DIR / "artifacts" / "checkpoints" / "relic_10m_latest.pt"))
    ap.add_argument("--out-dir", default=str(WEIGHTS_DIR))
    ap.add_argument("--fp32", action="store_true")
    args = ap.parse_args()

    ckpt_path = Path(args.ckpt)
    out_dir = Path(args.out_dir)
    suffix = "fp32" if args.fp32 else "fp16"
    out_path = out_dir / f"relic_10m_{suffix}.safetensors"

    export_checkpoint_to_safetensors(ckpt_path, out_path, fp16=not args.fp32)


if __name__ == "__main__":
    main()
