"""Rotary Position Embeddings (RoPE) para RELIC-10M.
Implementacion nativa optimizada. Menos de 180 lineas.
"""

import torch
import torch.nn as nn


def rotate_half(x: torch.Tensor) -> torch.Tensor:
    """Rota la primera y segunda mitad de las dimensiones de cabeza."""
    half = x.shape[-1] // 2
    x1 = x[..., :half]
    x2 = x[..., half:]
    return torch.cat((-x2, x1), dim=-1)


def apply_rotary_pos_emb(
    q: torch.Tensor,
    k: torch.Tensor,
    cos: torch.Tensor,
    sin: torch.Tensor,
    unsqueeze_dim: int = 1,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Aplica RoPE a los tensores de query y key broadcasting sobre cabezas."""
    cos = cos.unsqueeze(unsqueeze_dim)
    sin = sin.unsqueeze(unsqueeze_dim)
    q_out = (q * cos) + (rotate_half(q) * sin)
    k_out = (k * cos) + (rotate_half(k) * sin)
    return q_out, k_out


class RotaryEmbedding(nn.Module):
    """Calcula frecuencias de embedding rotacional de posiciones (RoPE)."""

    def __init__(self, dim: int, max_position_embeddings: int = 512, base: float = 10000.0) -> None:
        super().__init__()
        self.dim = dim
        self.max_position_embeddings = max_position_embeddings
        self.base = base

        # Inversas de frecuencias: 1 / (base ^ (2i / dim))
        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2, dtype=torch.float32) / dim))
        self.register_buffer("inv_freq", inv_freq, persistent=False)

    @torch.no_grad()
    def forward(
        self,
        x: torch.Tensor,
        position_ids: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor]:
        """Calcula cos y sin para las posiciones dadas."""
        device = x.device
        dtype = x.dtype
        # shape de freqs: [batch_size, seq_len, dim // 2]
        inv_freq = self.inv_freq.to(device=device, dtype=torch.float32)
        freqs = position_ids.unsqueeze(-1).to(torch.float32) * inv_freq
        # shape emb: [batch_size, seq_len, dim]
        emb = torch.cat((freqs, freqs), dim=-1)
        cos = emb.cos().to(dtype=dtype)
        sin = emb.sin().to(dtype=dtype)
        return cos, sin
