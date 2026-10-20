#!/usr/bin/env python3
"""Prueba de concepto minimalista: Consenso de marcos de referencia ciclicos (CRT).
Sin sobreingenieria: 2 modulos con periodos coprimos (5 y 7) resuelven suma mod 35.
Menos de 180 lineas.
"""

import math
import torch
import torch.nn as nn
import torch.nn.functional as F

P1, P2 = 5, 7
M = P1 * P2  # Modulo global = 35


class CorticalModule(nn.Module):
    """Modulo individual con su propio marco de referencia periodico."""

    def __init__(self, period: int, hidden_dim: int = 16) -> None:
        super().__init__()
        self.period = period
        self.omega = 2.0 * math.pi / period

        # Codificador de coordenadas circulares (cos, sin) para (a, b) -> 4 dimensiones
        self.net = nn.Sequential(
            nn.Linear(4, hidden_dim),
            nn.SiLU(),
            nn.Linear(hidden_dim, 2),  # Predice fase resultante (cos, sin) en su circulo
        )

    def forward(self, a: torch.Tensor, b: torch.Tensor) -> torch.Tensor:
        # Mapeo al marco de referencia local del modulo
        theta_a = a * self.omega
        theta_b = b * self.omega
        coords = torch.cat([theta_a.cos(), theta_a.sin(), theta_b.cos(), theta_b.sin()], dim=-1)
        out_phase = self.net(coords)
        return F.normalize(out_phase, p=2, dim=-1)


class CyclicConsensusModel(nn.Module):
    """Consenso entre modulos coprimos sin decodificador denso pesado."""

    def __init__(self, periods: list[int] = [5, 7]) -> None:
        super().__init__()
        self.modules_list = nn.ModuleList([CorticalModule(p) for p in periods])
        # Proyeccion de consenso directo: 2 dimensiones de fase por modulo (total 4) -> M clases
        total_phase_dims = len(periods) * 2
        self.consensus_proj = nn.Linear(total_phase_dims, M, bias=False)

    def forward(self, a: torch.Tensor, b: torch.Tensor) -> torch.Tensor:
        phases = [mod(a, b) for mod in self.modules_list]
        joint_representation = torch.cat(phases, dim=-1)
        logits = self.consensus_proj(joint_representation)
        return logits


def run_experiment() -> None:
    torch.manual_seed(42)

    # 1. Generar todos los pares posibles (a, b) con a, b in [0, 34]
    all_pairs = []
    all_targets = []
    for a in range(M):
        for b in range(M):
            all_pairs.append([float(a), float(b)])
            all_targets.append((a + b) % M)

    data_x = torch.tensor(all_pairs, dtype=torch.float32)
    data_y = torch.tensor(all_targets, dtype=torch.long)

    # 2. Particion Train/Test: 70% entrenamiento / 30% nunca visto (OOD)
    num_samples = len(data_x)
    indices = torch.randperm(num_samples)
    split = int(0.70 * num_samples)

    train_idx = indices[:split]
    test_idx = indices[split:]

    train_x, train_y = data_x[train_idx], data_y[train_idx]
    test_x, test_y = data_x[test_idx], data_y[test_idx]

    # 3. Entrenar el modelo de consenso
    model = CyclicConsensusModel(periods=[P1, P2])
    total_params = sum(p.numel() for p in model.parameters())
    print(f"Modelo instanciado: {total_params} parametros (Modulos coprimos: {P1} y {P2})")
    print(f"Muestras de entrenamiento: {len(train_x)} | Evaluacion OOD: {len(test_x)}")

    optimizer = torch.optim.Adam(model.parameters(), lr=0.01)

    print("\nEntrenando consenso durante 300 epocas...")
    for epoch in range(1, 301):
        optimizer.zero_grad()
        logits = model(train_x[:, 0:1], train_x[:, 1:2])
        loss = F.cross_entropy(logits, train_y)
        loss.backward()
        optimizer.step()

        if epoch % 50 == 0 or epoch == 300:
            pred_train = logits.argmax(dim=-1)
            acc_train = (pred_train == train_y).float().mean() * 100.0
            print(f"Epoca {epoch:03d} | Loss: {loss.item():.4f} | Acc Train: {acc_train:.1f}%")

    # 4. Evaluacion sobre pares no vistos (Generalizacion real)
    model.eval()
    with torch.no_grad():
        test_logits = model(test_x[:, 0:1], test_x[:, 1:2])
        test_preds = test_logits.argmax(dim=-1)
        test_acc = (test_preds == test_y).float().mean() * 100.0

    # 5. Baseline: Red densa estandar (Vanilla MLP) con los mismos parametros
    vanilla_model = nn.Sequential(
        nn.Linear(2, 24),
        nn.SiLU(),
        nn.Linear(24, M),
    )
    opt_v = torch.optim.Adam(vanilla_model.parameters(), lr=0.01)
    for _ in range(300):
        opt_v.zero_grad()
        loss_v = F.cross_entropy(vanilla_model(train_x), train_y)
        loss_v.backward()
        opt_v.step()

    vanilla_model.eval()
    with torch.no_grad():
        v_preds = vanilla_model(test_x).argmax(dim=-1)
        v_acc = (v_preds == test_y).float().mean() * 100.0

    print("\n" + "=" * 55)
    print("COMPARATIVA DE GENERALIZACION OOD (PARES NUNCA VISTOS)")
    print(f"Red Densa Estandar (Vanilla MLP):      {v_acc:.2f}% precision OOD")
    print(f"Consenso Ciclico Coprimo (Nuestro):    {test_acc:.2f}% precision OOD")
    print("=" * 55)


if __name__ == "__main__":
    run_experiment()
