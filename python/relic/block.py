"""Bloque Decodificador Transformer para RELIC-10M.
Implementacion modular nativa. Menos de 180 lineas.
"""

import torch
import torch.nn as nn

from relic.attention import RelicAttention
from relic.config import RelicConfig
from relic.mlp import RelicMLP
from relic.norm import RMSNorm


class RelicBlock(nn.Module):
    """Capa decodificadora con pre-RMSNorm, atencion GQA y MLP SwiGLU."""

    def __init__(self, config: RelicConfig) -> None:
        super().__init__()
        self.input_layernorm = RMSNorm(config.hidden_size, eps=config.rms_norm_eps)
        self.self_attn = RelicAttention(config)
        self.post_attention_layernorm = RMSNorm(config.hidden_size, eps=config.rms_norm_eps)
        self.mlp = RelicMLP(config)

    def forward(
        self,
        hidden_states: torch.Tensor,
        position_embeddings: tuple[torch.Tensor, torch.Tensor],
        attention_mask: torch.Tensor | None = None,
    ) -> torch.Tensor:
        # Sub-bloque 1: Atencion con conexion residual
        residual = hidden_states
        normed_states = self.input_layernorm(hidden_states)
        attn_out = self.self_attn(
            normed_states,
            position_embeddings=position_embeddings,
            attention_mask=attention_mask,
        )
        hidden_states = residual + attn_out

        # Sub-bloque 2: SwiGLU MLP con conexion residual
        residual = hidden_states
        normed_states = self.post_attention_layernorm(hidden_states)
        mlp_out = self.mlp(normed_states)
        hidden_states = residual + mlp_out

        return hidden_states
