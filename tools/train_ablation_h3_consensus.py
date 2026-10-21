#!/usr/bin/env python3
"""Ablación Controlada H3: Consenso Modular Descentralizado (102,432 parámetros).
Evalúa formalmente la Hipótesis 3:
- Desacopla el cómputo en K=4 columnas corticales independientes con paso de
  mensajes lateral y votación/consenso ensemble sin decodificador centralizado.
- Paridad matemática estricta: 102,432 parámetros (0.28% de diferencia con la línea base).
- Evalúa la hipótesis de falsación: ¿aporta la comunicación modular descentralizada
  una ventaja en búsqueda en grafos o inducción de reglas frente a las redes monolíticas?
- Salida guardada en reports/ablation_h3_consensus_report.json.
"""

import ctypes
from ctypes import wintypes
import json
import math
import random
import sys
import time
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F

# Configurar salida UTF-8
sys.stdout.reconfigure(encoding="utf-8")

ROOT_DIR = Path(__file__).resolve().parent.parent
BENCHMARK_DIR = ROOT_DIR / "data" / "benchmarks"
BASELINE_REPORT = ROOT_DIR / "reports" / "baseline_benchmark_report.json"
H1_REPORT = ROOT_DIR / "reports" / "ablation_h1_recurrent_report.json"
H2_REPORT = ROOT_DIR / "reports" / "ablation_h2_toroidal_report.json"
REPORT_PATH = ROOT_DIR / "reports" / "ablation_h3_consensus_report.json"

# ==============================================================================
# MONITOR DE MEMORIA RESIDENTE (RSS) NATIVO PARA WINDOWS
# ==============================================================================
class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]

GetProcessMemoryInfo = ctypes.windll.psapi.GetProcessMemoryInfo
GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESS_MEMORY_COUNTERS), wintypes.DWORD]
GetProcessMemoryInfo.restype = wintypes.BOOL

def get_current_rss_mb() -> float:
    handle = ctypes.windll.kernel32.GetCurrentProcess()
    counters = PROCESS_MEMORY_COUNTERS()
    counters.cb = ctypes.sizeof(PROCESS_MEMORY_COUNTERS)
    if GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
        return counters.WorkingSetSize / (1024.0 * 1024.0)
    return 0.0


# ==============================================================================
# VOCABULARIO COMPACTO (V = 128)
# ==============================================================================
VOCAB_SIZE = 128

class CharTokenizer:
    def __init__(self):
        self.pad_token = "<pad>"
        self.bos_token = "<bos>"
        self.eos_token = "<eos>"
        self.special = [self.pad_token, self.bos_token, self.eos_token]
        chars = (
            "()[]{}abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
            "0123456789-> ,.:;¿?!=_+\n"
        )
        unique_chars = sorted(list(set(chars)))
        self.char_to_id = {tok: i for i, tok in enumerate(self.special)}
        for c in unique_chars:
            if c not in self.char_to_id and len(self.char_to_id) < VOCAB_SIZE:
                self.char_to_id[c] = len(self.char_to_id)
        for i in range(len(self.char_to_id), VOCAB_SIZE):
            self.char_to_id[f"<res_{i}>"] = i
        self.id_to_char = {i: c for c, i in self.char_to_id.items()}
        self.pad_id = self.char_to_id[self.pad_token]
        self.bos_id = self.char_to_id[self.bos_token]
        self.eos_id = self.char_to_id[self.eos_token]

    def encode(self, text: str) -> list[int]:
        return [self.char_to_id.get(c, self.pad_id) for c in text]

    def decode(self, ids: list[int]) -> str:
        return "".join(self.id_to_char.get(i, "") for i in ids if i not in (self.pad_id, self.bos_id, self.eos_id))


# ==============================================================================
# ARQUITECTURA H3: CONSENSO MODULAR DESCENTRALIZADO (K=4 COLUMNAS CORTICALES)
# ==============================================================================
class RMSNorm(nn.Module):
    def __init__(self, dim: int, eps: float = 1e-6):
        super().__init__()
        self.eps = eps
        self.weight = nn.Parameter(torch.ones(dim))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        variance = x.pow(2).mean(-1, keepdim=True)
        return x * torch.rsqrt(variance + self.eps) * self.weight


class SwiGLU(nn.Module):
    def __init__(self, d_in: int, d_ff: int):
        super().__init__()
        self.gate_proj = nn.Linear(d_in, d_ff, bias=False)
        self.up_proj = nn.Linear(d_in, d_ff, bias=False)
        self.down_proj = nn.Linear(d_ff, d_in, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.down_proj(F.silu(self.gate_proj(x)) * self.up_proj(x))


class CorticalModule(nn.Module):
    """Módulo o columna cortical autónoma con cómputo local y emisión de mensaje."""
    def __init__(self, d_m: int = 16, d_ff_m: int = 218):
        super().__init__()
        self.d_m = d_m
        self.norm1 = RMSNorm(d_m)
        self.q = nn.Linear(d_m, d_m, bias=False)
        self.k = nn.Linear(d_m, d_m, bias=False)
        self.v = nn.Linear(d_m, d_m, bias=False)
        self.o = nn.Linear(d_m, d_m, bias=False)
        # Emisor de mensaje compacto
        self.msg_proj = nn.Linear(d_m, d_m, bias=False)
        self.norm2 = RMSNorm(d_m)
        self.mlp = SwiGLU(d_m, d_ff_m)

    def forward(self, x: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        B, T, C = x.shape
        x_norm = self.norm1(x)
        q = self.q(x_norm)
        k = self.k(x_norm)
        v = self.v(x_norm)

        scores = torch.matmul(q, k.transpose(-2, -1)) / math.sqrt(self.d_m)
        mask = torch.triu(torch.full((T, T), float("-inf"), device=x.device), diagonal=1)
        attn = F.softmax(scores + mask, dim=-1)
        h = self.o(torch.matmul(attn, v)) + x

        msg = self.msg_proj(h)
        return h, msg


class ModularConsensusModel(nn.Module):
    """Modelo de Consenso Descentralizado con K=4 Columnas y Voto Ensemble."""
    def __init__(self, vocab_size: int = 128, d_model: int = 64, K: int = 4,
                 n_layers: int = 2, d_ff_m: int = 218):
        super().__init__()
        self.vocab_size = vocab_size
        self.d_model = d_model
        self.K = K
        self.d_m = d_model // K  # 16
        self.emb = nn.Embedding(vocab_size, d_model)

        # Matrices de acople lateral A_ij entre columnas por capa
        self.lateral_A = nn.ParameterList([
            nn.Parameter(torch.eye(K) + 0.1 * torch.randn(K, K)) for _ in range(n_layers)
        ])

        self.layers = nn.ModuleList([
            nn.ModuleList([CorticalModule(self.d_m, d_ff_m) for _ in range(K)])
            for _ in range(n_layers)
        ])

    def count_parameters(self) -> int:
        return sum(p.numel() for p in self.parameters())

    def forward(self, tokens: torch.Tensor) -> torch.Tensor:
        B, T = tokens.shape
        x_all = self.emb(tokens)
        cols = [x_all[..., k * self.d_m : (k + 1) * self.d_m] for k in range(self.K)]

        for l_idx, layer in enumerate(self.layers):
            A = self.lateral_A[l_idx]
            h_list = []
            msg_list = []
            for k in range(self.K):
                h_k, msg_k = layer[k](cols[k])
                h_list.append(h_k)
                msg_list.append(msg_k)

            # Paso de mensajes lateral entre módulos
            msgs = torch.stack(msg_list, dim=-2)  # [B, T, K, d_m]
            integrated = torch.einsum("ij, btjd -> btid", A, msgs)

            # Integración local y SwiGLU por columna
            new_cols = []
            for k in range(self.K):
                h_fused = h_list[k] + integrated[:, :, k, :]
                h_out = h_fused + layer[k].mlp(layer[k].norm2(h_fused))
                new_cols.append(h_out)
            cols = new_cols

        # Consenso de salida descentralizado por votación ensemble
        logits_total = torch.zeros(B, T, self.vocab_size, device=tokens.device)
        for k in range(self.K):
            sub_emb = self.emb.weight[:, k * self.d_m : (k + 1) * self.d_m]
            logits_total += F.linear(cols[k], sub_emb)

        return logits_total


# ==============================================================================
# FLOPs ANALÍTICOS
# ==============================================================================
def calculate_consensus_flops(seq_len: int, d_model: int = 64, K: int = 4,
                              n_layers: int = 2, d_ff_m: int = 218, vocab_size: int = 128) -> int:
    d_m = d_model // K
    # 1. Atencion local por modulo: 4 * (2 * T * d_m^2) + 4 * T^2 * d_m
    local_attn_flops = K * ((8 * seq_len * (d_m ** 2)) + (4 * (seq_len ** 2) * d_m))
    # 2. Mensajes: emision (2 * T * d_m^2 * K) + integracion lateral (2 * T * K^2 * d_m)
    msg_flops = (2 * seq_len * (d_m ** 2) * K) + (2 * seq_len * (K ** 2) * d_m)
    # 3. SwiGLU local por modulo: K * (6 * T * d_m * d_ff_m)
    mlp_flops = K * (6 * seq_len * d_m * d_ff_m)
    # 4. Normas locales
    norm_flops = n_layers * K * 2 * 3 * seq_len * d_m
    # 5. Votacion de salida descentralizada: K * (2 * T * d_m * V)
    voting_flops = K * (2 * seq_len * d_m * vocab_size)

    total_flops = n_layers * (local_attn_flops + msg_flops + mlp_flops) + norm_flops + voting_flops
    return int(total_flops)


# ==============================================================================
# ENTRENAMIENTO Y EVALUACIÓN
# ==============================================================================
def train_and_eval_task(task_name: str, tokenizer: CharTokenizer, epochs: int = 30,
                        lr: float = 3e-3, batch_size: int = 32) -> dict:
    torch.manual_seed(42)
    task_dir = BENCHMARK_DIR / task_name

    with open(task_dir / "train.json", "r", encoding="utf-8") as f:
        train_data = json.load(f)
    with open(task_dir / "val_id.json", "r", encoding="utf-8") as f:
        val_id_data = json.load(f)
    with open(task_dir / "test_ood.json", "r", encoding="utf-8") as f:
        test_ood_data = json.load(f)

    model = ModularConsensusModel(vocab_size=VOCAB_SIZE)
    total_params = model.count_parameters()
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=0.01)

    print(f"\n--- ENTRENANDO ABLACIÓN H3 EN: {task_name.upper()} ({total_params} params) ---")
    start_time = time.time()
    initial_rss = get_current_rss_mb()
    max_rss = initial_rss

    loss_history = []

    for epoch in range(1, epochs + 1):
        model.train()
        total_loss = 0.0
        n_batches = 0

        indices = list(range(len(train_data)))
        random.shuffle(indices)

        for b_start in range(0, len(indices), batch_size):
            batch_idx = indices[b_start:b_start + batch_size]
            batch_samples = [train_data[i] for i in batch_idx]

            full_texts = [f"{s['prompt']} {s['target']}" for s in batch_samples]
            encoded_batch = [tokenizer.encode(t) for t in full_texts]
            max_len = max(len(ids) for ids in encoded_batch)

            input_ids = []
            target_ids = []

            for s, ids in zip(batch_samples, encoded_batch):
                prompt_ids = tokenizer.encode(f"{s['prompt']} ")
                pad_len = max_len - len(ids)
                padded = ids + [tokenizer.pad_id] * pad_len
                mask_target = [-100] * (len(prompt_ids) - 1) + ids[len(prompt_ids) - 1:] + [-100] * pad_len
                input_ids.append(padded[:-1])
                target_ids.append(mask_target[1:])

            inputs = torch.tensor(input_ids, dtype=torch.long)
            targets = torch.tensor(target_ids, dtype=torch.long)

            optimizer.zero_grad()
            logits = model(inputs)
            loss = F.cross_entropy(logits.view(-1, VOCAB_SIZE), targets.view(-1), ignore_index=-100)
            loss.backward()
            optimizer.step()

            total_loss += loss.item()
            n_batches += 1

        avg_loss = total_loss / max(1, n_batches)
        loss_history.append(round(avg_loss, 4))

        current_rss = get_current_rss_mb()
        if current_rss > max_rss:
            max_rss = current_rss

        if epoch % 5 == 0 or epoch == epochs:
            print(f"  Época {epoch:02d}/{epochs} | Loss: {avg_loss:.4f} | RAM: {current_rss:.1f} MB")

    train_duration = time.time() - start_time

    # ==========================================================================
    # EVALUACIÓN ESTRICTA (Acc_ID vs Acc_OOD)
    # ==========================================================================
    model.eval()

    def evaluate_split(split_data: list[dict]) -> tuple[float, list[dict]]:
        correct = 0
        details = []
        with torch.no_grad():
            for item in split_data:
                prompt_text = f"{item['prompt']} "
                prompt_ids = tokenizer.encode(prompt_text)
                expected_target = item["target"]

                curr_ids = list(prompt_ids)
                pred_tokens = []

                for _ in range(len(expected_target)):
                    inp = torch.tensor([curr_ids], dtype=torch.long)
                    logits = model(inp)
                    next_token_id = logits[0, -1, :].argmax().item()
                    pred_tokens.append(next_token_id)
                    curr_ids.append(next_token_id)

                predicted_text = tokenizer.decode(pred_tokens).strip()
                is_match = (predicted_text == expected_target)
                if is_match:
                    correct += 1

                details.append({
                    "id": item["id"],
                    "expected": expected_target,
                    "predicted": predicted_text,
                    "correct": is_match
                })

        accuracy = (correct / len(split_data)) * 100.0
        return accuracy, details

    acc_id, details_id = evaluate_split(val_id_data)
    acc_ood, details_ood = evaluate_split(test_ood_data)

    # Medir velocidad de inferencia (tokens/segundo)
    inference_tok_s = 0.0
    with torch.no_grad():
        test_prompt = tokenizer.encode(f"{test_ood_data[0]['prompt']} ")
        inp = torch.tensor([test_prompt], dtype=torch.long)
        t_start = time.time()
        for _ in range(50):
            _ = model(inp)
        inference_tok_s = (50 * len(test_prompt)) / (time.time() - t_start)

    avg_seq_len = int(sum(len(s['prompt']) for s in val_id_data) / len(val_id_data))
    flops_per_pass = calculate_consensus_flops(avg_seq_len, d_model=64, K=4, n_layers=2, d_ff_m=218, vocab_size=VOCAB_SIZE)

    print(f"\n  [RESULTADOS H3 CONSENSUS {task_name.upper()}]:")
    print(f"    Precisión In-Distribution (Val ID):  {acc_id:.2f}%")
    print(f"    Precisión Extrapolación (Test OOD):  {acc_ood:.2f}%")
    print(f"    FLOPs teóricos / pasada (T={avg_seq_len}): {flops_per_pass:,}")
    print(f"    Velocidad de inferencia CPU:         {inference_tok_s:.1f} tok/s")
    print(f"    Tiempo total de entrenamiento:       {train_duration:.1f} s")
    print(f"    Pico de memoria RAM (RSS):           {max_rss:.1f} MB (Delta: +{max_rss - initial_rss:.1f} MB)")

    return {
        "task": task_name,
        "acc_id": round(acc_id, 2),
        "acc_ood": round(acc_ood, 2),
        "train_loss_history": loss_history,
        "train_time_sec": round(train_duration, 2),
        "peak_rss_mb": round(max_rss, 2),
        "delta_rss_mb": round(max_rss - initial_rss, 2),
        "inference_tokens_per_sec": round(inference_tok_s, 1),
        "flops_per_forward": flops_per_pass
    }


def run_ablation_h3_benchmark():
    print("=" * 65)
    print("  EXPERIMENTO 3 — ABLACIÓN H3: CONSENSO MODULAR (102,432 params)")
    print("=" * 65)

    tok = CharTokenizer()
    results = {}

    tasks = ["dyck", "graph", "rules"]
    for task in tasks:
        results[task] = train_and_eval_task(task, tok, epochs=30)

    # Cargar reportes previos
    baseline_data = {}
    if BASELINE_REPORT.exists():
        with open(BASELINE_REPORT, "r", encoding="utf-8") as f:
            baseline_data = json.load(f)

    h1_data = {}
    if H1_REPORT.exists():
        with open(H1_REPORT, "r", encoding="utf-8") as f:
            h1_data = json.load(f)

    h2_data = {}
    if H2_REPORT.exists():
        with open(H2_REPORT, "r", encoding="utf-8") as f:
            h2_data = json.load(f)

    full_report = {
        "hypothesis": "H3: Consenso Modular Descentralizado con K=4 Columnas y Voto Ensemble",
        "model_architecture": "Modular Cortical Consensus Model",
        "parameters": 102432,
        "parameter_parity_with_baseline": True,
        "hyperparameters": {
            "vocab_size": VOCAB_SIZE,
            "d_model": 64,
            "K_modules": 4,
            "d_m": 16,
            "n_layers": 2,
            "d_ff_m": 218,
            "lateral_routing": "Learnable A_ij Matrix",
            "output_mechanism": "Decentralized Ensemble Voting",
            "norm": "RMSNorm",
            "mlp": "SwiGLU"
        },
        "tasks": results
    }

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        json.dump(full_report, f, indent=2)

    print("\n" + "=" * 90)
    print("  MATRIZ FINAL COMPLETA: BASELINE vs H1 (RECURRENTE) vs H2 (FASE) vs H3 (CONSENSO)")
    print("=" * 90)
    print(f"{'Tarea':<8} | {'Modelo':<18} | {'Acc ID':<9} | {'Acc OOD':<9} | {'FLOPs/pass':<11} | {'RAM (RSS)':<10}")
    print("-" * 90)

    base_tasks = baseline_data.get("tasks", {})
    h1_tasks = h1_data.get("tasks", {})
    h2_tasks = h2_data.get("tasks", {})

    for t in tasks:
        b = base_tasks.get(t, {})
        h1 = h1_tasks.get(t, {})
        h2 = h2_tasks.get(t, {})
        h3 = results[t]
        if b:
            print(f"{t.upper():<8} | {'Transformer Base':<18} | {b['acc_id']:>6.2f}%   | {b['acc_ood']:>6.2f}%   | {b['flops_per_forward']:>9,}   | {b['peak_rss_mb']:>6.1f} MB")
        if h1:
            print(f"{'':<8} | {'H1 Recurrent':<18} | {h1['acc_id']:>6.2f}%   | {h1['acc_ood']:>6.2f}%   | {h1['flops_per_forward']:>9,}   | {h1['peak_rss_mb']:>6.1f} MB")
        if h2:
            print(f"{'':<8} | {'H2 Phase Torus':<18} | {h2['acc_id']:>6.2f}%   | {h2['acc_ood']:>6.2f}%   | {h2['flops_per_forward']:>9,}   | {h2['peak_rss_mb']:>6.1f} MB")
        print(f"{'':<8} | {'H3 Modular Consensus':<18} | {h3['acc_id']:>6.2f}%   | {h3['acc_ood']:>6.2f}%   | {h3['flops_per_forward']:>9,}   | {h3['peak_rss_mb']:>6.1f} MB")
        print("-" * 90)

    print(f"Reporte final guardado en: {REPORT_PATH}")


if __name__ == "__main__":
    run_ablation_h3_benchmark()
