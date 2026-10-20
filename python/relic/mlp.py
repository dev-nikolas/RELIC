"""Bloque SwiGLU MLP para RELIC-10M.
Implementacion nativa optimizada. Menos de 180 lineas.
"""

import torch
import torch.nn as nn
import torch.nn.functional as F

from relic.config import RelicConfig


class RelicMLP(nn.Module):
    """Feed-Forward Network con compuerta SwiGLU (Swish-Gated Linear Unit)."""

    def __init__(self, config: RelicConfig) -> None:
        super().__init__()
        self.hidden_size = config.hidden_size
        self.intermediate_size = config.intermediate_size

        self.gate_proj = nn.Linear(self.hidden_size, self.intermediate_size, bias=False)
        self.up_proj = nn.Linear(self.hidden_size, self.intermediate_size, bias=False)
        self.down_proj = nn.Linear(self.intermediate_size, self.hidden_size, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # SwiGLU: down_proj(silu(gate_proj(x)) * up_proj(x))
        swish = F.silu(self.gate_proj(x))
        gated = swish * self.up_proj(x)
        return self.down_proj(gated)
