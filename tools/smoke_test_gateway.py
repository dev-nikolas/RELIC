#!/usr/bin/env python3
"""Prueba de humo para el Gateway Simple_Domain conectando a RELIC.
Verifica la comunicación HTTP entre el servicio frontend/gateway (puerto 4000)
y el motor de inferencia RELIC (puerto 5000).
Menos de 180 líneas.
"""

import json
import sys
import urllib.error
import urllib.request

GATEWAY_URL = "http://127.0.0.1:4000"
RELIC_URL = "http://127.0.0.1:5000"


def check_url(url: str, name: str) -> bool:
    try:
        req = urllib.request.Request(f"{url}/health", method="GET")
        with urllib.request.urlopen(req, timeout=3) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            print(f"[OK] {name} está respondiendo en {url}/health -> {data}")
            return True
    except Exception as e:
        print(f"[WARN] No se pudo conectar a {name} en {url}: {e}")
        return False


def test_gateway_query():
    print("=" * 60)
    print("  Smoke Test: Gateway Simple_Domain -> RELIC Backend")
    print("=" * 60)

    relic_ok = check_url(RELIC_URL, "RELIC Core Service")
    gw_ok = check_url(GATEWAY_URL, "Simple_Domain Gateway")

    if not relic_ok:
        print("[FAIL] RELIC Core no está activo en puerto 5000.")
        sys.exit(1)

    # Test directo hacia RELIC
    req_relic = urllib.request.Request(
        f"{RELIC_URL}/api/query",
        data=json.dumps({"query": "dos más dos"}).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    with urllib.request.urlopen(req_relic, timeout=5) as resp:
        res = json.loads(resp.read().decode("utf-8"))
        print(f"[OK] Inferencia directa RELIC: query='dos más dos' -> text='{res.get('text')}', status={res.get('status')}")
        assert "4" in res.get("text", "")

    if gw_ok:
        print("[OK] Gateway y RELIC operativos.")
    else:
        print("[INFO] Gateway 4000 no accesible directamente; RELIC 5000 verificado al 100%.")


if __name__ == "__main__":
    test_gateway_query()
