#!/usr/bin/env python3
"""Script de entrenamiento optimizado para Google Colab y Kaggle con acelerador GPU (T4/P100/V100).
Menos de 180 líneas.

Instrucciones de uso en Colab/Kaggle:
1. Clonar el repositorio:
   !git clone <URL_REPOSITORIO> && cd RELIC
2. Instalar dependencias:
   !pip install -q torch safetensors tokenizers numpy
3. Ejecutar este script:
   !python tools/train_colab.py --steps 5000
4. Descargar el archivo exportado: artifacts/weights/relic_10m_fp16.safetensors
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "tools"))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--config", default=str(ROOT_DIR / "configs" / "train_tinystories_10m.json"))
    ap.add_argument("--steps", type=int, default=5000, help="Pasos de entrenamiento a ejecutar")
    ap.add_argument("--batch-size", type=int, default=32, help="Tamaño de lote para GPU")
    ap.add_argument("--resume", default=None, help="Checkpoint .pt previo")
    args = ap.parse_args()

    # Cargar y ajustar configuración en memoria
    with open(args.config, "r", encoding="utf-8") as f:
        cfg = json.load(f)

    cfg["training"]["max_steps"] = args.steps
    cfg["training"]["batch_size"] = args.batch_size
    temp_cfg = ROOT_DIR / "artifacts" / "colab_run_config.json"
    temp_cfg.parent.mkdir(parents=True, exist_ok=True)
    with open(temp_cfg, "w", encoding="utf-8") as f:
        json.dump(cfg, f, indent=2)

    print("=" * 60)
    print("  RELIC — Entrenamiento en Acelerador Cloud (Colab / Kaggle)")
    print("=" * 60)
    print(f"  Configuración: {temp_cfg}")
    print(f"  Pasos:         {args.steps}")
    print(f"  Lote (Batch):  {args.batch_size}")
    print("=" * 60)

    # 1. Ejecutar motor de entrenamiento
    train_cmd = [
        sys.executable,
        str(ROOT_DIR / "tools" / "train.py"),
        "--config", str(temp_cfg),
        "--device", "cuda",
    ]
    if args.resume:
        train_cmd.extend(["--resume", str(args.resume)])

    ret = subprocess.run(train_cmd, cwd=str(ROOT_DIR))
    if ret.returncode != 0:
        sys.exit(f"[ERROR] El entrenamiento falló con código {ret.returncode}")

    # 2. Exportar automáticamente a Safetensors FP16
    print("\nExportando pesos a Safetensors FP16...")
    export_cmd = [sys.executable, str(ROOT_DIR / "tools" / "export_safetensors.py")]
    subprocess.run(export_cmd, cwd=str(ROOT_DIR), check=True)

    weights_out = ROOT_DIR / "artifacts" / "weights" / "relic_10m_fp16.safetensors"
    print("\n" + "=" * 60)
    print("  ENTRENAMIENTO Y EXPORTACIÓN COMPLETADOS EXITOSAMENTE")
    print(f"  Archivo generado: {weights_out}")
    print("  Para descargar en Colab:")
    print("    from google.colab import files")
    print("    files.download('artifacts/weights/relic_10m_fp16.safetensors')")
    print("=" * 60)


if __name__ == "__main__":
    main()
