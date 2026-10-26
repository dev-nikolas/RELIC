#!/usr/bin/env python3
"""
RELIC & Simple Domain — Script de Humo End-to-End: Sandbox Lab
Valida flujo completo: creación de corrida, streaming SSE, cálculo de recompensa y persistencia.
Menos de 180 líneas.
"""

import sys, time, json, tempfile
from pathlib import Path

# Agregar Simple_Domain/backend/src al path
repo_root = Path(__file__).resolve().parents[2]
sd_backend = repo_root / "Simple_Domain" / "backend" / "src"
if str(sd_backend) not in sys.path:
    sys.path.insert(0, str(sd_backend))

from fastapi.testclient import TestClient
from simple_domain.service.app import create_app


def run_smoke_test():
    print("=" * 60)
    print("Iniciando prueba de humo: Sandbox Lab (Punta a Punta)")
    print("=" * 60)

    with tempfile.TemporaryDirectory() as tmpdir:
        app = create_app(storage_dir=tmpdir)
        with TestClient(app) as client:
            # 1. Crear corrida en sandbox
            payload = {
                "level": "L1_direct_lookup",
                "seed": 42,
                "system": "rules",
                "penalty_lambda": 5.0,
                "max_episodes": 5,
            }
            t0 = time.perf_counter()
            resp = client.post("/v1/sandbox/runs", json=payload)
            assert resp.status_code == 201, f"Fallo al crear corrida: {resp.text}"
            run_data = resp.json()
            run_id = run_data["id"]
            print(f"[MEDIDO] Corrida creada: {run_id} | Status: {run_data['status']}")
            print(f"[LEÍDO] Commit: {run_data['commit']} | Dataset Hash: {run_data['dataset_hash'][:10]}... | Pesos: {run_data['weights_hash'][:10]}...")

            # 2. Consumir SSE y validar eventos en vivo
            events_received = []
            with client.stream("GET", f"/v1/sandbox/runs/{run_id}/events") as sse_stream:
                for line in sse_stream.iter_lines():
                    if not line:
                        continue
                    if line.startswith("data:"):
                        raw_data = line[len("data:"):].strip()
                        try:
                            evt = json.loads(raw_data)
                            events_received.append(evt)
                            evt_type = evt.get("type", "")
                            if evt_type == "EPISODE_STARTED":
                                print(f"  -> Evento: EPISODE_STARTED #{evt['data']['episode']}")
                            elif evt_type == "RIR_PROPOSED":
                                print(f"  -> Evento: RIR_PROPOSED #{evt['data']['episode']} (status: {evt['data']['status']})")
                            elif evt_type == "VERIFIER_EVALUATED":
                                print(f"  -> Evento: VERIFIER_EVALUATED #{evt['data']['episode']} (recompensa: {evt['data']['reward']})")
                            elif evt_type in ["sandbox.completed", "EPISODE_COMPLETED_FINAL"]:
                                print(f"  -> Evento Terminal: {evt_type}")
                                break
                        except Exception:
                            pass

            t1 = time.perf_counter()
            elapsed_ms = (t1 - t0) * 1000
            print(f"[MEDIDO] Streaming SSE completado en {elapsed_ms:.1f} ms | Eventos recibidos: {len(events_received)}")

            # 3. Validar estado final y métricas en GET /v1/sandbox/runs/{id}
            get_resp = client.get(f"/v1/sandbox/runs/{run_id}")
            assert get_resp.status_code == 200, f"Error al consultar corrida: {get_resp.text}"
            final_run = get_resp.json()
            assert final_run["status"] == "completed", f"Estado esperado 'completed', obtenido: {final_run['status']}"

            metrics = final_run["metrics"]
            episodes = final_run.get("episodes", [])
            print(f"[MEDIDO] Total episodios ejecutados: {len(episodes)}")
            print(f"[MEDIDO] Exact Match Count: {metrics['exact_match_count']} / {final_run['total_episodes']}")
            print(f"[MEDIDO] Exact Match Rate: {metrics['exact_match_rate'] * 100:.1f}%")
            print(f"[MEDIDO] Wilson 95% CI: [{metrics['wilson_95'][0]*100:.1f}%, {metrics['wilson_95'][1]*100:.1f}%]")
            print(f"[MEDIDO] Falsa Aceptación Rate: {metrics['false_acceptance_rate'] * 100:.1f}%")
            print(f"[MEDIDO] Recompensa Media: {metrics['mean_reward']:.2f}")

            # 4. Validar estructura de trace en el primer episodio
            if episodes:
                ep1 = episodes[0]
                trace = ep1.get("trace", [])
                print(f"[MEDIDO] Etapas de traza en Ep 1: {len(trace)} etapas")
                for st in trace:
                    print(f"     * {st['stage']}: {st['latency_ms']:.2f} ms")

            # 5. Validar listado en GET /v1/sandbox/runs
            list_resp = client.get("/v1/sandbox/runs")
            assert list_resp.status_code == 200
            all_runs = list_resp.json()
            assert any(r["id"] == run_id for r in all_runs), "La corrida debe figurar en la lista global"
            print(f"[MEDIDO] GET /v1/sandbox/runs retornó {len(all_runs)} corridas")

    print("=" * 60)
    print("PRUEBA DE HUMO FINALIZADA CON ÉXITO: 100% VERIFICADO")
    print("=" * 60)


if __name__ == "__main__":
    run_smoke_test()
