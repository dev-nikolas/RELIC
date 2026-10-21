#!/usr/bin/env python3
"""Ablación Controlada H2: Representación de Fase Toroidal (102,752 parámetros).
Evalúa formalmente la Hipótesis 2:
- Sustituye los embeddings euclidianos no acotados por coordenadas de fase
  periódicas en toroides unitarios (S^1)^(d/2) con frecuencias aprendibles omega.
- Paridad matemática estricta: 102,752 parámetros (0.03% de diferencia con la línea base).
- Evalúa la hipótesis de falsación: ¿aporta la representación de fase una ventaja
  inductiva medible en generalización ordinaria (ID) o extrapolación (OOD)?
- Salida guardada en reports/ablation_h2_toroidal_report.json.
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
REPORT_PATH = ROOT_DIR / "reports" / "ablation_h2_toroidal_report.json"

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
# ARQUITECTURA H2: REPRESENTACIÓN DE FASE EN TOROIDES T^(d/2)
# ==============================================================================
class PhaseEmbedding(nn.Module):
    """Mapeo a coordenadas periódicas circulares (cos, sin) en S^1 con frecuencias aprendibles."""
    def __init__(self, vocab_size: int = 128, d_model: int = 64):
        super().__init__()
        self.d_phase = d_model // 2
        # Ángulos de fase libres por cada token en [ -pi, pi ]
        self.phase_angles = nn.Parameter(torch.randn(vocab_size, self.d_phase))
        # Frecuencias intrínsecas omega aprendibles
        self.omega = nn.Parameter(torch.ones(self.d_phase))
        # Proyección isométrica al espacio residual d_model
        self.proj = nn.Linear(d_model, d_model, bias=False)

    def forward(self, tokens: torch.Tensor) -> torch.Tensor:
        # angles: [B, T, d_phase]
        angles = self.phase_angles[tokens] * self.omega
        # Coordenadas en el toroide unitario (cos, sin)
        coords = torch.cat([angles.cos(), angles.sin()], dim=-1)
        return self.proj(coords)


class RMSNorm(nn.Module):
    def __init__(self, dim: int, eps: float = 1e-6):
        super().__init__()
        self.eps = eps
        self.weight = nn.Parameter(torch.ones(dim))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        variance = x.pow(2).mean(-1, keepdim=True)
        return x * torch.rsqrt(variance + self.eps) * self.weight


def apply_rotary_emb(xq: torch.Tensor, xk: torch.Tensor, dim: int, base: float = 10000.0) -> tuple[torch.Tensor, torch.Tensor]:
    seq_len = xq.shape[1]
    half_dim = dim // 2
    inv_freq = 1.0 / (base ** (torch.arange(0, half_dim, dtype=torch.float32, device=xq.device) / half_dim))
    t = torch.arange(seq_len, dtype=torch.float32, device=xq.device)
    freqs = torch.outer(t, inv_freq)
    emb = torch.cat((freqs, freqs), dim=-1)
    cos = emb.cos().unsqueeze(0).unsqueeze(2)
    sin = emb.sin().unsqueeze(0).unsqueeze(2)

    def rotate_half(x):
        x1 = x[..., :half_dim]
        x2 = x[..., half_dim:]
        return torch.cat((-x2, x1), dim=-1)

    xq_out = (xq * cos) + (rotate_half(xq) * sin)
    xk_out = (xk * cos) + (rotate_half(xk) * sin)
    return xq_out, xk_out


class CausalSelfAttention(nn.Module):
    def __init__(self, d_model: int, n_heads: int):
        super().__init__()
        self.d_model = d_model
        self.n_heads = n_heads
        self.head_dim = d_model // n_heads

        self.q_proj = nn.Linear(d_model, d_model, bias=False)
        self.k_proj = nn.Linear(d_model, d_model, bias=False)
        self.v_proj = nn.Linear(d_model, d_model, bias=False)
        self.out_proj = nn.Linear(d_model, d_model, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        B, T, C = x.shape
        q = self.q_proj(x).view(B, T, self.n_heads, self.head_dim)
        k = self.k_proj(x).view(B, T, self.n_heads, self.head_dim)
        v = self.v_proj(x).view(B, T, self.n_heads, self.head_dim)

        q, k = apply_rotary_emb(q, k, self.head_dim)

        q = q.transpose(1, 2)
        k = k.transpose(1, 2)
        v = v.transpose(1, 2)

        scores = torch.matmul(q, k.transpose(-2, -1)) / math.sqrt(self.head_dim)
        mask = torch.triu(torch.full((T, T), float("-inf"), device=x.device), diagonal=1)
        scores = scores + mask
        attn = F.softmax(scores, dim=-1)
        out = torch.matmul(attn, v)

        out = out.transpose(1, 2).contiguous().view(B, T, C)
        return self.out_proj(out)


class SwiGLU(nn.Module):
    def __init__(self, d_model: int, d_ff: int):
        super().__init__()
        self.gate_proj = nn.Linear(d_model, d_ff, bias=False)
        self.up_proj = nn.Linear(d_model, d_ff, bias=False)
        self.down_proj = nn.Linear(d_ff, d_model, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.down_proj(F.silu(self.gate_proj(x)) * self.up_proj(x))


class TransformerBlock(nn.Module):
    def __init__(self, d_model: int, n_heads: int, d_ff: int):
        super().__init__()
        self.input_norm = RMSNorm(d_model)
        self.attn = CausalSelfAttention(d_model, n_heads)
        self.post_norm = RMSNorm(d_model)
        self.mlp = SwiGLU(d_model, d_ff)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = x + self.attn(self.input_norm(x))
        x = x + self.mlp(self.post_norm(x))
        return x


class CalibratedPhaseModel(nn.Module):
    """Modelo de Fase Toroidal calibrado a exactamente 102,752 parámetros."""
    def __init__(self, vocab_size: int = 128, d_model: int = 64, n_heads: int = 4,
                 n_layers: int = 2, d_ff: int = 160):
        super().__init__()
        self.vocab_size = vocab_size
        self.d_model = d_model
        self.n_heads = n_heads
        self.n_layers = n_layers
        self.d_ff = d_ff

        self.phase_emb = PhaseEmbedding(vocab_size, d_model)
        self.layers = nn.ModuleList([
            TransformerBlock(d_model, n_heads, d_ff) for _ in range(n_layers)
        ])
        self.final_norm = RMSNorm(d_model)

    def count_parameters(self) -> int:
        return sum(p.numel() for p in self.parameters())

    def forward(self, tokens: torch.Tensor) -> torch.Tensor:
        x = self.phase_emb(tokens)
        for layer in self.layers:
            x = layer(x)
        x = self.final_norm(x)
        
        # Proyección resonante de fase sobre el vocabulario
        all_vocab_tokens = torch.arange(self.vocab_size, device=tokens.device)
        vocab_repr = self.phase_emb(all_vocab_tokens.unsqueeze(0)).squeeze(0)  # [V, d_model]
        logits = F.linear(x, vocab_repr)
        return logits


# ==============================================================================
# FLOPs ANALÍTICOS
# ==============================================================================
def calculate_phase_flops(seq_len: int, d_model: int = 64, n_layers: int = 2,
                          d_ff: int = 160, vocab_size: int = 128) -> int:
    d_phase = d_model // 2
    # 1. Phase embedding: rotaciones trigonometricas + proyeccion lineal
    phase_ops = seq_len * (4 * d_phase) + (2 * seq_len * d_model * d_model)
    # 2. Atencion
    attn_flops = (8 * seq_len * (d_model ** 2)) + (4 * (seq_len ** 2) * d_model)
    # 3. SwiGLU
    mlp_flops = 6 * seq_len * d_model * d_ff
    # 4. Normas
    norm_flops = (2 * n_layers + 1) * 3 * seq_len * d_model
    # 5. Proyeccion resonante
    lm_head_flops = 2 * seq_len * d_model * vocab_size

    total_flops = phase_ops + n_layers * (attn_flops + mlp_flops) + norm_flops + lm_head_flops
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

    model = CalibratedPhaseModel(vocab_size=VOCAB_SIZE)
    total_params = model.count_parameters()
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=0.01)

    print(f"\n--- ENTRENANDO ABLACIÓN H2 EN: {task_name.upper()} ({total_params} params) ---")
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
    flops_per_pass = calculate_phase_flops(avg_seq_len, d_model=64, n_layers=2, d_ff=160, vocab_size=VOCAB_SIZE)

    print(f"\n  [RESULTADOS H2 PHASE {task_name.upper()}]:")
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


def run_ablation_h2_benchmark():
    print("=" * 65)
    print("  EXPERIMENTO 3 — ABLACIÓN H2: REPRESENTACIÓN DE FASE (102,752 params)")
    print("=" * 65)

    tok = CharTokenizer()
    results = {}

    tasks = ["dyck", "graph", "rules"]
    for task in tasks:
        results[task] = train_and_eval_task(task, tok, epochs=30)

    # Cargar reportes previos para contraste
    baseline_data = {}
    if BASELINE_REPORT.exists():
        with open(BASELINE_REPORT, "r", encoding="utf-8") as f:
            baseline_data = json.load(f)

    h1_data = {}
    if H1_REPORT.exists():
        with open(H1_REPORT, "r", encoding="utf-8") as f:
            h1_data = json.load(f)

    full_report = {
        "hypothesis": "H2: Representación de Fase Toroidal en T^(d/2) con Frecuencias Aprendibles",
        "model_architecture": "Phase Embedding Causal Transformer",
        "parameters": 102752,
        "parameter_parity_with_baseline": True,
        "hyperparameters": {
            "vocab_size": VOCAB_SIZE,
            "d_model": 64,
            "d_phase": 32,
            "n_heads": 4,
            "n_layers": 2,
            "d_ff": 160,
            "phase_projection": "Resonant Tied Vocabulary",
            "norm": "RMSNorm",
            "pos_emb": "RoPE",
            "mlp": "SwiGLU"
        },
        "tasks": results
    }

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        json.dump(full_report, f, indent=2)

    print("\n" + "=" * 85)
    print("  COMPARATIVA TRIANGULAR: TRANSFORMER vs H1 (RECURRENTE) vs H2 (FASE TOROIDAL)")
    print("=" * 85)
    print(f"{'Tarea':<8} | {'Modelo':<16} | {'Acc ID':<9} | {'Acc OOD':<9} | {'FLOPs/pass':<11} | {'RAM (RSS)':<10}")
    print("-" * 85)

    base_tasks = baseline_data.get("tasks", {})
    h1_tasks = h1_data.get("tasks", {})

    for t in tasks:
        b = base_tasks.get(t, {})
        h1 = h1_tasks.get(t, {})
        h2 = results[t]
        if b:
            print(f"{t.upper():<8} | {'Transformer Base':<16} | {b['acc_id']:>6.2f}%   | {b['acc_ood']:>6.2f}%   | {b['flops_per_forward']:>9,}   | {b['peak_rss_mb']:>6.1f} MB")
        if h1:
            print(f"{'':<8} | {'H1 Recurrent':<16} | {h1['acc_id']:>6.2f}%   | {h1['acc_ood']:>6.2f}%   | {h1['flops_per_forward']:>9,}   | {h1['peak_rss_mb']:>6.1f} MB")
        print(f"{'':<8} | {'H2 Phase Torus':<16} | {h2['acc_id']:>6.2f}%   | {h2['acc_ood']:>6.2f}%   | {h2['flops_per_forward']:>9,}   | {h2['peak_rss_mb']:>6.1f} MB")
        print("-" * 85)

    print(f"Reporte oficial guardado en: {REPORT_PATH}")


if __name__ == "__main__":
    run_ablation_h2_benchmark()
