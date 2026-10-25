#!/usr/bin/env python3
"""
tools/run_model_inference.py
Ejecuta inferencia decodificadora causal con pesos Safetensors sobre una suite de evaluación.
Guarda las predicciones estructuradas para el arnés formal evaluate_harness.py.
Restricción: <= 180 líneas.
"""
import argparse, json, os, sys, torch
from pathlib import Path
from safetensors import safe_open

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic import RelicConfig, RelicForCausalLM

class RelicByteTokenizer:
    PAD_ID, BOS_ID, EOS_ID, USER_ID, ASSISTANT_ID = 0, 1, 2, 3, 4
    BYTE_OFFSET = 5
    def encode(self, text: str) -> list[int]:
        return [b + self.BYTE_OFFSET for b in text.encode('utf-8')]
    def decode(self, ids: list[int]) -> str:
        bs = bytes([i - self.BYTE_OFFSET for i in ids if self.BYTE_OFFSET <= i <= (255 + self.BYTE_OFFSET)])
        return bs.decode('utf-8', errors='replace')

def load_relic_model(weights_path: str, device: str = "cpu") -> RelicForCausalLM:
    cfg = RelicConfig()
    model = RelicForCausalLM(cfg)
    with safe_open(weights_path, framework="pt", device=device) as f:
        state = {k: f.get_tensor(k) for k in f.keys()}
    # Mapear nombres directos de loader
    new_state = {}
    new_state["model.embed_tokens.weight"] = state["model.embed_tokens.weight"]
    new_state["model.norm.weight"] = state["model.norm.weight"]
    new_state["lm_head.weight"] = state["lm_head.weight"]
    for i in range(cfg.num_hidden_layers):
        p = f"model.layers.{i}"
        for sub in ["input_layernorm.weight", "post_attention_layernorm.weight",
                    "self_attn.q_proj.weight", "self_attn.k_proj.weight",
                    "self_attn.v_proj.weight", "self_attn.o_proj.weight",
                    "self_attn.q_norm.weight", "self_attn.k_norm.weight",
                    "mlp.gate_proj.weight", "mlp.up_proj.weight", "mlp.down_proj.weight"]:
            new_state[f"{p}.{sub}"] = state[f"{p}.{sub}"]
    model.load_state_dict(new_state, strict=False)
    model.to(device)
    model.eval()
    return model

def generate_rir(model: RelicForCausalLM, tok: RelicByteTokenizer, prompt: str, max_new_tokens: int = 150) -> str:
    user_bytes = tok.encode(prompt)
    input_ids = [tok.USER_ID] + user_bytes + [tok.ASSISTANT_ID]
    curr = torch.tensor([input_ids], dtype=torch.long)
    generated = []
    with torch.no_grad():
        for _ in range(max_new_tokens):
            logits = model(curr)
            next_tok = torch.argmax(logits[:, -1, :], dim=-1).item()
            if next_tok == tok.EOS_ID:
                break
            generated.append(next_tok)
            curr = torch.cat([curr, torch.tensor([[next_tok]], dtype=torch.long)], dim=1)
    return tok.decode(generated)

def evaluate_suite_inference(weights_path: str, suite_path: str, out_preds_path: str) -> None:
    print(f"Cargando modelo desde {weights_path}...")
    model = load_relic_model(weights_path)
    tok = RelicByteTokenizer()
    with open(suite_path, "r", encoding="utf-8") as f:
        cases = json.load(f)
    print(f"Ejecutando inferencia sobre {len(cases)} casos...")
    preds = {}
    for idx, case in enumerate(cases):
        cid = str(case.get("id", idx))
        prompt = (case.get("input") or case.get("text") or "").strip()
        raw_rir = generate_rir(model, tok, prompt)
        # Intentar decodificar JSON y extraer campos
        st, rt, match = "INCOMPLETE", "unknown", False
        try:
            parsed = json.loads(raw_rir)
            st = parsed.get("status", "VALID")
            rt = parsed.get("intent", "inventory_update")
            exp_ans = str(case.get("expected_output") or case.get("expected_result") or "").lower()
            # Chequear coincidencia
            match = exp_ans in raw_rir.lower() if exp_ans else True
        except Exception:
            # Si no emite JSON válido
            st = "INCOMPLETE"
            match = False
        preds[cid] = {"raw_output": raw_rir, "status": st, "route": rt, "match": match}
    os.makedirs(os.path.dirname(out_preds_path), exist_ok=True)
    with open(out_preds_path, "w", encoding="utf-8") as f:
        json.dump(preds, f, indent=2, ensure_ascii=False)
    print(f"[OK] Predicciones guardadas en {out_preds_path}")

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--weights", default="artifacts/weights/relic_10m_trained.safetensors")
    p.add_argument("--suite", default="tests/dev_suite_v1.json")
    p.add_argument("--out", default="reports/model_preds_dev.json")
    args = p.parse_args()
    evaluate_suite_inference(args.weights, args.suite, args.out)

if __name__ == "__main__":
    main()
