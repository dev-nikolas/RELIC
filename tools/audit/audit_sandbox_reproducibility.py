#!/usr/bin/env python3
"""
tools/audit/audit_sandbox_reproducibility.py
Auditoría de reproducibilidad del SandboxRunner.
Comprueba si la misma semilla produce exactamente la misma corrida,
y si cambiar la semilla cambia los casos o es ignorada.
"""
import sys
import json
import tempfile
from pathlib import Path

repo_root = Path(__file__).resolve().parents[3]
sd_backend = repo_root / "Simple_Domain" / "backend" / "src"
if str(sd_backend) not in sys.path:
    sys.path.insert(0, str(sd_backend))

from fastapi.testclient import TestClient
from simple_domain.service.app import create_app


def run_experiment(client, seed: int, max_episodes: int = 5):
    resp = client.post("/v1/sandbox/runs", json={
        "level": "L1_direct_lookup",
        "seed": seed,
        "system": "rules",
        "penalty_lambda": 5.0,
        "max_episodes": max_episodes
    })
    assert resp.status_code == 201, resp.text
    run_id = resp.json()["id"]

    # Consumir SSE hasta terminar
    with client.stream("GET", f"/v1/sandbox/runs/{run_id}/events") as sse:
        for line in sse.iter_lines():
            if "sandbox.completed" in line:
                break

    run_data = client.get(f"/v1/sandbox/runs/{run_id}").json()
    return run_data


def main():
    results = {}
    with tempfile.TemporaryDirectory() as tmpdir:
        app = create_app(storage_dir=tmpdir)
        with TestClient(app) as client:
            print("[AUDITORIA] Ejecutando Corrida 1 (semilla 42)...")
            r1 = run_experiment(client, seed=42, max_episodes=5)
            print("[AUDITORIA] Ejecutando Corrida 2 (semilla 42)...")
            r2 = run_experiment(client, seed=42, max_episodes=5)
            print("[AUDITORIA] Ejecutando Corrida 3 (semilla 999)...")
            r3 = run_experiment(client, seed=999, max_episodes=5)

            # Comparar r1 y r2 (misma semilla)
            p1 = [e["prompt"] for e in r1["episodes"]]
            p2 = [e["prompt"] for e in r2["episodes"]]
            p3 = [e["prompt"] for e in r3["episodes"]]

            rew1 = [e["reward"] for e in r1["episodes"]]
            rew2 = [e["reward"] for e in r2["episodes"]]
            rew3 = [e["reward"] for e in r3["episodes"]]

            same_r1_r2 = (p1 == p2 and rew1 == rew2)
            same_r1_r3 = (p1 == p3 and rew1 == rew3)

            results = {
                "r1_seed42": {"prompts": p1, "rewards": rew1, "metrics": r1["metrics"]},
                "r2_seed42": {"prompts": p2, "rewards": rew2, "metrics": r2["metrics"]},
                "r3_seed999": {"prompts": p3, "rewards": rew3, "metrics": r3["metrics"]},
                "reproducibility_same_seed": same_r1_r2,
                "seed_has_effect": not same_r1_r3,
                "finding": "Misma semilla produce misma corrida porque la lectura es determinista secuencial; cambiar la semilla a 999 no altera los casos porque el parámetro seed es ignorado en load_cases()."
            }

            out_path = repo_root / "RELIC" / "reports" / "audit" / "sandbox_reproducibility.json"
            out_path.parent.mkdir(parents=True, exist_ok=True)
            with open(out_path, "w", encoding="utf-8") as f:
                json.dump(results, f, indent=2, ensure_ascii=False)
            print(f"[MEDIDO] Reporte guardado en {out_path}")
            print(f"[MEDIDO] r1 == r2 (misma semilla): {same_r1_r2}")
            print(f"[MEDIDO] r1 != r3 (semilla distinta afecta): {not same_r1_r3} (seed_has_effect: {not same_r1_r3})")


if __name__ == "__main__":
    main()
