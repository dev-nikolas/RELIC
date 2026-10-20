"""Modelo RELIC-10M completo (Decoder-only Transformer).
Conteo de parametros: 10,031,328 (Pesos desatados).
Menos de 180 lineas.
"""

import torch
import torch.nn as nn

from relic.block import RelicBlock
from relic.config import RelicConfig
from relic.norm import RMSNorm
from relic.rotary import RotaryEmbedding


class RelicModel(nn.Module):
    """Tronco de representacion Transformer decodificador."""

    def __init__(self, config: RelicConfig) -> None:
        super().__init__()
        self.config = config
        self.embed_tokens = nn.Embedding(config.vocab_size, config.hidden_size)
        self.layers = nn.ModuleList([RelicBlock(config) for _ in range(config.num_hidden_layers)])
        self.norm = RMSNorm(config.hidden_size, eps=config.rms_norm_eps)
        self.rotary_emb = RotaryEmbedding(
            dim=config.head_dim,
            max_position_embeddings=config.max_position_embeddings,
            base=config.rope_theta,
        )

    def forward(
        self,
        input_ids: torch.Tensor,
        position_ids: torch.Tensor | None = None,
        attention_mask: torch.Tensor | None = None,
    ) -> torch.Tensor:
        batch_size, seq_len = input_ids.shape
        if position_ids is None:
            position_ids = torch.arange(seq_len, dtype=torch.long, device=input_ids.device)
            position_ids = position_ids.unsqueeze(0).expand(batch_size, -1)

        hidden_states = self.embed_tokens(input_ids)
        position_embeddings = self.rotary_emb(hidden_states, position_ids)

        for layer in self.layers:
            hidden_states = layer(
                hidden_states,
                position_embeddings=position_embeddings,
                attention_mask=attention_mask,
            )

        hidden_states = self.norm(hidden_states)
        return hidden_states


class RelicForCausalLM(nn.Module):
    """Modelo de lenguaje causal RELIC con cabeza LM desatada."""

    def __init__(self, config: RelicConfig) -> None:
        super().__init__()
        self.config = config
        self.model = RelicModel(config)
        self.lm_head = nn.Linear(config.hidden_size, config.vocab_size, bias=False)

    def forward(
        self,
        input_ids: torch.Tensor,
        position_ids: torch.Tensor | None = None,
        attention_mask: torch.Tensor | None = None,
    ) -> torch.Tensor:
        hidden_states = self.model(
            input_ids=input_ids,
            position_ids=position_ids,
            attention_mask=attention_mask,
        )
        logits = self.lm_head(hidden_states)
        return logits

    def count_parameters(self) -> int:
        """Calcula el numero total de parametros entrenables."""
        return sum(p.numel() for p in self.parameters())
