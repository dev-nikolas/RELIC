#!/usr/bin/env python3
"""Prueba de humo (smoke test) para validar relic_server y sus endpoints HTTP.
Menos de 180 líneas.
"""

import json
import sys
import urllib.error
import urllib.request

BASE_URL = "http://127.0.0.1:4000"


def request_json(method: str, path: str, payload=None) -> tuple[int, dict]:
    url = f"{BASE_URL}{path}"
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    headers = {"Content-Type": "application/json"} if payload is not None else {}
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, json.loads(body)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8")
        try:
            parsed = json.loads(body)
        except Exception:
            parsed = {"error": body}
        return e.code, parsed


def test_endpoint(name: str, passed: bool, detail: str = ""):
    status_str = "[OK]" if passed else "[FAIL]"
    print(f"{status_str} {name}: {detail}")
    if not passed:
        sys.exit(1)


def main():
    print("=" * 60)
    print("  RELIC HTTP Server — Prueba de Humo (Smoke Test)")
    print("=" * 60)

    # 1. Health
    status, health = request_json("GET", "/health")
    test_endpoint("GET /health", status == 200 and health.get("detector_ready") is True, f"status={health.get('status')}")

    # 2. Info
    status, info = request_json("GET", "/v1/info")
    test_endpoint("GET /v1/info", status == 200, f"capabilities={info.get('capabilities')}")

    # 3. Query Aritmética ("1 + 1")
    status, q1 = request_json("POST", "/api/query", {"query": "1 + 1"})
    ok1 = status == 200 and q1.get("status") == "VALID" and "2" in q1.get("text", "")
    test_endpoint("POST /api/query (1 + 1)", ok1, f"route={q1.get('route')}, text='{q1.get('text')}'")

    # 4. Query Grafo
    status, q2 = request_json("POST", "/api/query", {"query": "¿Depende el crate relic_cli del paquete memmap2?"})
    ok2 = status == 200 and q2.get("status") == "VALID" and q2.get("route") == "deterministic_graph"
    test_endpoint("POST /api/query (Grafo)", ok2, f"route={q2.get('route')}, text='{q2.get('text')}'")

    # 5. Query Lenguaje Libre
    status, q3 = request_json("POST", "/api/query", {"query": "El pingüino encontró una señal en Bogotá"})
    ok3 = status == 200 and q3.get("status") == "VALID"
    test_endpoint("POST /api/query (Lenguaje Libre)", ok3, f"route={q3.get('route')}")

    # 6. Conversación Simple Domain
    status, conv = request_json("POST", "/v1/conversations", {"title": "Sesión Humo"})
    cid = conv.get("id")
    test_endpoint("POST /v1/conversations", status == 201 and bool(cid), f"conv_id={cid}")

    status, msg = request_json("POST", f"/v1/conversations/{cid}/messages", {"role": "user", "content": "tengo 10 gatos y recibo 5"})
    ok_msg = status == 201 and msg.get("role") == "assistant" and "15" in msg.get("content", "")
    test_endpoint("POST /v1/conversations/.../messages", ok_msg, f"role={msg.get('role')}, reply='{msg.get('content')}'")

    # 7. SSE Streaming
    url = f"{BASE_URL}/api/generate"
    req = urllib.request.Request(url, data=json.dumps({"query": "1 + 1"}).encode("utf-8"), headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            raw = resp.read().decode("utf-8")
            ok_sse = resp.status == 200 and "data: " in raw and "finished" in raw
            test_endpoint("POST /api/generate (SSE)", ok_sse, f"chunks recibidos: {raw.count('data: ')}")
    except Exception as e:
        test_endpoint("POST /api/generate (SSE)", False, str(e))

    print("=" * 60)
    print("  Todas las pruebas de humo pasaron exitosamente.")
    print("=" * 60)


if __name__ == "__main__":
    main()
