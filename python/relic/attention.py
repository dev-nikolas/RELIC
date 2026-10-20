"""Mecanismo de Atencion GQA con QK-RMSNorm y RoPE para RELIC-10M.
Implementacion nativa optimizada. Menos de 180 lineas.
"""

import math
import torch
import torch.nn as nn
import torch.nn.functional as F

from relic.config import RelicConfig
from relic.norm import RMSNorm
from relic.rotary import apply_rotary_pos_emb


def repeat_kv(x: torch.Tensor, n_rep: int) -> torch.Tensor:
    """Expande cabezas KV para equiparar al conteo de cabezas Query en GQA."""
    if n_rep == 1:
        return x
    batch, num_kv_heads, seq_len, head_dim = x.shape
    expanded = x[:, :, None, :, :].expand(batch, num_kv_heads, n_rep, seq_len, head_dim)
    return expanded.reshape(batch, num_kv_heads * n_rep, seq_len, head_dim)


class RelicAttention(nn.Module):
    """Atencion causal multi-cabeza con GQA (Grouped Query Attention) y QK-Norm."""

    def __init__(self, config: RelicConfig) -> None:
        super().__init__()
        self.config = config
        self.hidden_size = config.hidden_size
        self.num_heads = config.num_attention_heads
        self.num_kv_heads = config.num_key_value_heads
        self.head_dim = config.head_dim
        self.num_kv_groups = self.num_heads // self.num_kv_heads
        self.scaling = 1.0 / math.sqrt(self.head_dim)

        self.q_proj = nn.Linear(self.hidden_size, self.num_heads * self.head_dim, bias=config.attention_bias)
        self.k_proj = nn.Linear(self.hidden_size, self.num_kv_heads * self.head_dim, bias=config.attention_bias)
        self.v_proj = nn.Linear(self.hidden_size, self.num_kv_heads * self.head_dim, bias=config.attention_bias)
        self.o_proj = nn.Linear(self.num_heads * self.head_dim, self.hidden_size, bias=config.attention_bias)

        # QK-RMSNorm aplicado sobre head_dim y compartido entre cabezas
        self.q_norm = RMSNorm(self.head_dim, eps=config.rms_norm_eps)
        self.k_norm = RMSNorm(self.head_dim, eps=config.rms_norm_eps)

    def forward(
        self,
        hidden_states: torch.Tensor,
        position_embeddings: tuple[torch.Tensor, torch.Tensor],
        attention_mask: torch.Tensor | None = None,
    ) -> torch.Tensor:
        batch_size, seq_len, _ = hidden_states.shape

        # Proyecciones lineales iniciales
        q = self.q_proj(hidden_states).view(batch_size, seq_len, self.num_heads, self.head_dim)
        k = self.k_proj(hidden_states).view(batch_size, seq_len, self.num_kv_heads, self.head_dim)
        v = self.v_proj(hidden_states).view(batch_size, seq_len, self.num_kv_heads, self.head_dim)

        # Normalizacion QK por cabeza previa a RoPE
        q = self.q_norm(q).transpose(1, 2)
        k = self.k_norm(k).transpose(1, 2)
        v = v.transpose(1, 2)

        # Aplicacion de RoPE
        cos, sin = position_embeddings
        q, k = apply_rotary_pos_emb(q, k, cos, sin)

        # GQA: Repeticion de cabezas K y V
        k = repeat_kv(k, self.num_kv_groups)
        v = repeat_kv(v, self.num_kv_groups)

        # Puntuaciones de atencion escaladas
        scores = torch.matmul(q, k.transpose(-2, -1)) * self.scaling

        # Mascara de atencion causal
        if attention_mask is not None:
            scores = scores + attention_mask
        else:
            causal_mask = torch.triu(
                torch.full((seq_len, seq_len), float("-inf"), device=hidden_states.device, dtype=scores.dtype),
                diagonal=1,
            )
            scores = scores + causal_mask

        # Softmax en precision float32 para estabilidad numerica
        attn_probs = F.softmax(scores, dim=-1, dtype=torch.float32).to(q.dtype)

        # Agregacion de valores y proyeccion de salida
        context = torch.matmul(attn_probs, v)
        context = context.transpose(1, 2).contiguous().view(batch_size, seq_len, -1)
        return self.o_proj(context)
