#!/usr/bin/env python3
"""
tools/download_weights.py
Descarga y verificación de integridad SHA-256 de pesos de modelo para RELIC.
Evita almacenar artefactos binarios pesados en el historial de Git.
Restricción: <= 180 líneas.
"""
import argparse, hashlib, os, sys, urllib.request

EXPECTED_SHA256 = "2f8485429c7e2ecf349336a0054ae49b64f940be8aa34d1d60d15daa987a5f5b"
DEFAULT_FILENAME = "relic_10m_trained.safetensors"
DEFAULT_TARGET_DIR = os.path.join(os.path.dirname(__file__), "..", "artifacts", "weights")

def calculate_sha256(filepath: str) -> str:
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()

def verify_file(filepath: str, expected_hash: str) -> bool:
    if not os.path.exists(filepath):
        return False
    actual = calculate_sha256(filepath)
    print(f"Archivo: {filepath}")
    print(f"  SHA-256 Actual:    {actual}")
    print(f"  SHA-256 Esperado:  {expected_hash}")
    return actual.lower() == expected_hash.lower()

def download_weights(url: str, dest_path: str, expected_hash: str) -> bool:
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    if os.path.exists(dest_path):
        print(f"[INFO]: El archivo ya existe localmente. Verificando integridad...")
        if verify_file(dest_path, expected_hash):
            print("[OK]: Archivo local verificado y conforme.")
            return True
        print("[AVISO]: Suma de verificación discrepante. Re-descargando...")
    if not url:
        print("[ERROR]: Se requiere una URL válida de descarga (GitHub Releases o Cloud Storage).")
        return False
    print(f"Descargando desde: {url}")
    urllib.request.urlretrieve(url, dest_path)
    if verify_file(dest_path, expected_hash):
        print("[OK]: Descarga e integridad validadas exitosamente.")
        return True
    else:
        print("[ERROR]: Falló la verificación de integridad SHA-256 tras la descarga.")
        return False

def main():
    p = argparse.ArgumentParser(description="Gestor de descarga y verificación de pesos RELIC")
    p.add_argument("--url", default="", help="URL de descarga de release")
    p.add_argument("--out", default=os.path.join(DEFAULT_TARGET_DIR, DEFAULT_FILENAME))
    p.add_argument("--expected-sha256", default=EXPECTED_SHA256)
    p.add_argument("--verify-only", action="store_true", help="Solo verifica el archivo existente")
    args = p.parse_args()

    if args.verify_only:
        ok = verify_file(args.out, args.expected_sha256)
        sys.exit(0 if ok else 1)
    ok = download_weights(args.url, args.out, args.expected_sha256)
    sys.exit(0 if ok else 1)

if __name__ == "__main__":
    main()
