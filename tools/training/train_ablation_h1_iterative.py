#!/usr/bin/env python3
"""Experimento: H1 + Cómputo Iterativo en Espacio Latente (Weight-Tied Thinking Loops).

Evalúa si desacoplar la profundidad algorítmica de la longitud de secuencia
permite resolver transitividad en GRAPH y composición en RULES sin añadir
parámetros entrenables (exactamente 102,720 parámetros).

Compara formalmente:
- S = 1 paso (Línea base H1 estándar)
- S = 2 pasos
- S = 4 pasos
- S = 8 pasos
"""

import ctypes
from ctypes import wintypes
import json
import math
import sys
import time
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F

sys.stdout.reconfigure(encoding="utf-8")

ROOT_DIR = Path(__file__).resolve().parent.parent
BENCHMARK_DIR = ROOT_DIR / "data" / "benchmarks"
REPORT_PATH = ROOT_DIR / "reports" / "ablation_h1_iterative_report.json"

# ==============================================================================
# MONITOR DE MEMORIA NATIVO PARA WINDOWS
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
# TOKENIZADOR
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
# ARQUITECTURA H1-ITER: NÚCLEO RECURRENTE CON REFINAMIENTO ITERATIVO
# ==============================================================================
class RMSNorm(nn.Module):
    def __init__(self, dim: int, eps: float = 1e-6):
        super().__init__()
        self.eps = eps
        self.weight = nn.Parameter(torch.ones(dim))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        variance = x.pow(2).mean(-1, keepdim=True)
        return x * torch.rsqrt(variance + self.eps) * self.weight


class CompactGatedRecurrence(nn.Module):
    def __init__(self, d_model: int):
        super().__init__()
        self.d_model = d_model
        self.in_proj = nn.Linear(d_model, 3 * d_model, bias=False)
        self.out_proj = nn.Linear(d_model, d_model, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        B, T, C = x.shape
        proj = self.in_proj(x)
        u, g, z = proj.chunk(3, dim=-1)

        gate = torch.sigmoid(g)
        cand = torch.tanh(u)
        out_gate = torch.sigmoid(z)

        h = torch.zeros(B, C, device=x.device)
        outputs = []
        for t in range(T):
            h = gate[:, t] * h + (1.0 - gate[:, t]) * cand[:, t]
            y_t = h * out_gate[:, t]
            outputs.append(y_t)

        y = torch.stack(outputs, dim=1)
        return self.out_proj(y)


class SwiGLU(nn.Module):
    def __init__(self, d_model: int, d_ff: int):
        super().__init__()
        self.gate_proj = nn.Linear(d_model, d_ff, bias=False)
        self.up_proj = nn.Linear(d_model, d_ff, bias=False)
        self.down_proj = nn.Linear(d_ff, d_model, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.down_proj(F.silu(self.gate_proj(x)) * self.up_proj(x))


class RecurrentBlock(nn.Module):
    def __init__(self, d_model: int, d_ff: int):
        super().__init__()
        self.input_norm = RMSNorm(d_model)
        self.rec = CompactGatedRecurrence(d_model)
        self.post_norm = RMSNorm(d_model)
        self.mlp = SwiGLU(d_model, d_ff)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = x + self.rec(self.input_norm(x))
        x = x + self.mlp(self.post_norm(x))
        return x


class IterativeRecurrentModel(nn.Module):
    """Modelo H1 con bucle de refinamiento iterativo en pesos compartidos.
    Conserva exactamente 102,720 parámetros entrenables.
    """
    def __init__(self, vocab_size: int = 128, d_model: int = 64, n_layers: int = 2,
                 d_ff: int = 160, thinking_steps: int = 1):
        super().__init__()
        self.vocab_size = vocab_size
        self.d_model = d_model
        self.n_layers = n_layers
        self.d_ff = d_ff
        self.thinking_steps = thinking_steps

        self.tok_embeddings = nn.Embedding(vocab_size, d_model)
        self.layers = nn.ModuleList([
            RecurrentBlock(d_model, d_ff) for _ in range(n_layers)
        ])
        self.norm = RMSNorm(d_model)
        self.output = nn.Linear(d_model, vocab_size, bias=False)
        self.output.weight = self.tok_embeddings.weight

    def forward(self, tokens: torch.Tensor, steps: int | None = None) -> torch.Tensor:
        S = steps if steps is not None else self.thinking_steps
        x = self.tok_embeddings(tokens)

        # Pasada secuencial base a través de las capas
        for layer in self.layers:
            x = layer(x)

        # Refinamiento iterativo sobre el estado latente final (Weight-tied thinking loop)
        # Reutiliza la capa superior recurrentemente para propagar inferencia (S-1 pasos adicionales)
        if S > 1:
            refine_layer = self.layers[-1]
            for _ in range(S - 1):
                x = refine_layer(x)

        x = self.norm(x)
        return self.output(x)

    def count_parameters(self) -> int:
        return sum(p.numel() for p in self.parameters() if p.requires_grad)


# ==============================================================================
# FLOPs ANALÍTICOS
# ==============================================================================
def calculate_h1_iter_flops(seq_len: int, steps: int, d_model: int = 64,
                            n_layers: int = 2, d_ff: int = 160, vocab_size: int = 128) -> int:
    rec_flops_per_layer = 8 * seq_len * (d_model ** 2) + 6 * seq_len * d_model
    mlp_flops_per_layer = 6 * seq_len * d_model * d_ff
    layer_flops = rec_flops_per_layer + mlp_flops_per_layer + (2 * 3 * seq_len * d_model)
    
    # Pasada base de L capas + (S-1) pasadas de la capa de refinamiento
    total_layers_executed = n_layers + (steps - 1)
    core_flops = total_layers_executed * layer_flops
    head_flops = 2 * seq_len * d_model * vocab_size + (3 * seq_len * d_model)
    return int(core_flops + head_flops)


# ==============================================================================
# ENTRENAMIENTO Y EVALUACIÓN POR TAREA Y PASOS
# ==============================================================================
def train_and_eval_configuration(task_name: str, steps: int, tokenizer: CharTokenizer,
                                 epochs: int = 30, lr: float = 3e-3, batch_size: int = 32,
                                 seed: int = 42) -> dict:
    torch.manual_seed(seed)
    task_dir = BENCHMARK_DIR / task_name

    with open(task_dir / "train.json", "r", encoding="utf-8") as f:
        train_data = json.load(f)
    with open(task_dir / "val_id.json", "r", encoding="utf-8") as f:
        val_id_data = json.load(f)
    with open(task_dir / "test_ood.json", "r", encoding="utf-8") as f:
        test_ood_data = json.load(f)

    model = IterativeRecurrentModel(vocab_size=VOCAB_SIZE, thinking_steps=steps)
    total_params = model.count_parameters()
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=0.01)

    print(f"\n>>> Entrenando H1-Iter (S={steps}) en: {task_name.upper()} | Params: {total_params} | Seed: {seed}")
    start_time = time.time()
    initial_rss = get_current_rss_mb()
    max_rss = initial_rss
    loss_history = []

    for epoch in range(1, epochs + 1):
        model.train()
        total_loss = 0.0
        n_batches = 0

        # Barajar
        indices = list(range(len(train_data)))
        torch.randperm(len(train_data))

        for i in range(0, len(train_data), batch_size):
            batch = train_data[i : i + batch_size]
            encoded_samples = []
            for item in batch:
                full_text = f"{item['prompt']} {item['target']}{tokenizer.eos_token}"
                tokens = tokenizer.encode(full_text)
                prompt_len = len(tokenizer.encode(f"{item['prompt']} "))
                encoded_samples.append((tokens, prompt_len))

            max_len = max(len(s[0]) for s in encoded_samples)
            inp_batch = []
            tgt_batch = []

            for tokens, prompt_len in encoded_samples:
                padded = tokens + [tokenizer.pad_id] * (max_len - len(tokens))
                targets = [-100] * (prompt_len - 1) + tokens[prompt_len:] + [-100] * (max_len - len(tokens))
                inp_batch.append(padded[:-1])
                tgt_batch.append(targets[: max_len - 1])

            x_t = torch.tensor(inp_batch, dtype=torch.long)
            y_t = torch.tensor(tgt_batch, dtype=torch.long)

            optimizer.zero_grad()
            logits = model(x_t, steps=steps)
            loss = F.cross_entropy(logits.view(-1, VOCAB_SIZE), y_t.view(-1), ignore_index=-100)
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()

            total_loss += loss.item()
            n_batches += 1

        avg_loss = total_loss / max(1, n_batches)
        loss_history.append(round(avg_loss, 4))
        cur_rss = get_current_rss_mb()
        if cur_rss > max_rss:
            max_rss = cur_rss

        if epoch % 10 == 0 or epoch == epochs:
            print(f"  Época {epoch:02d}/{epochs} | Loss: {avg_loss:.4f} | RAM: {cur_rss:.1f} MB")

    train_duration = time.time() - start_time

    # Evaluación Estricta
    model.eval()
    def evaluate_split(split_data: list[dict]) -> float:
        correct = 0
        with torch.no_grad():
            for item in split_data:
                prompt_text = f"{item['prompt']} "
                prompt_ids = tokenizer.encode(prompt_text)
                expected_target = item["target"]
                curr_ids = list(prompt_ids)
                pred_tokens = []

                for _ in range(len(expected_target)):
                    inp = torch.tensor([curr_ids], dtype=torch.long)
                    logits = model(inp, steps=steps)
                    next_token = logits[0, -1, :].argmax().item()
                    pred_tokens.append(next_token)
                    curr_ids.append(next_token)

                pred_text = tokenizer.decode(pred_tokens).strip()
                if pred_text == expected_target:
                    correct += 1
        return (correct / len(split_data)) * 100.0

    acc_id = evaluate_split(val_id_data)
    acc_ood = evaluate_split(test_ood_data)

    # Medir velocidad de inferencia (tokens/segundo)
    inference_tok_s = 0.0
    with torch.no_grad():
        test_prompt = tokenizer.encode(f"{test_ood_data[0]['prompt']} ")
        inp = torch.tensor([test_prompt], dtype=torch.long)
        t_start = time.time()
        for _ in range(50):
            _ = model(inp, steps=steps)
        t_end = time.time()
        inference_tok_s = round((50 * len(test_prompt)) / max(1e-5, (t_end - t_start)), 1)

    eval_len = len(tokenizer.encode(f"{test_ood_data[0]['prompt']} {test_ood_data[0]['target']}"))
    flops_per_forward = calculate_h1_iter_flops(seq_len=eval_len, steps=steps)

    print(f"  [RESULTADOS S={steps}]: ID={acc_id:.2f}% | OOD={acc_ood:.2f}% | Throughput={inference_tok_s} tok/s | FLOPs={flops_per_forward:,}")

    return {
        "task": task_name,
        "steps": steps,
        "acc_id": acc_id,
        "acc_ood": acc_ood,
        "flops_per_forward": flops_per_forward,
        "inference_tokens_per_sec": inference_tok_s,
        "peak_rss_mb": round(max_rss, 2),
        "train_time_sec": round(train_duration, 2),
        "final_loss": loss_history[-1]
    }


def main():
    tokenizer = CharTokenizer()
    steps_list = [1, 2, 4, 8]
    tasks = ["graph", "rules"]

    print("=" * 80)
    print(" EXPERIMENTO: H1 + CÓMPUTO ITERATIVO (WEIGHT-TIED REFINEMENT LOOPS)")
    print(" Parámetros constantes: 102,720 | Barrido: S = 1, 2, 4, 8 pasos")
    print("=" * 80)

    results = {}
    for task in tasks:
        results[task] = {}
        for s in steps_list:
            res = train_and_eval_configuration(task_name=task, steps=s, tokenizer=tokenizer, epochs=30)
            results[task][f"steps_{s}"] = res

    # Serializar reporte
    full_report = {
        "experiment": "H1 + Cómputo Iterativo en Espacio Latente",
        "description": "Evaluación del impacto del desacople de profundidad algorítmica sobre GRAPH y RULES",
        "parameters": 102720,
        "tasks": results
    }

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        json.dump(full_report, f, indent=2)

    print("\n" + "=" * 90)
    print("  MATRIZ COMPARATIVA DE CÓMPUTO ITERATIVO (H1-ITER)")
    print("=" * 90)
    print(f"{'Tarea':<8} | {'Pasos (S)':<10} | {'Acc ID':<9} | {'Acc OOD':<9} | {'FLOPs/pass':<12} | {'Throughput':<14} | {'RAM (RSS)':<10}")
    print("-" * 90)

    for task in tasks:
        for s in steps_list:
            r = results[task][f"steps_{s}"]
            print(f"{task.upper():<8} | {s:<10} | {r['acc_id']:>6.2f}%   | {r['acc_ood']:>6.2f}%   | {r['flops_per_forward']:>10,}   | {r['inference_tokens_per_sec']:>9.1f} tok/s | {r['peak_rss_mb']:>6.1f} MB")
        print("-" * 90)

    print(f"\nReporte guardado en: {REPORT_PATH}")


if __name__ == "__main__":
    main()
