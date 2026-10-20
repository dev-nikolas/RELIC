"""RELIC-10M: Modelo Transformer Minimalista de 10 Millones de Parametros.
Menos de 180 lineas.
"""

from relic.attention import RelicAttention
from relic.block import RelicBlock
from relic.config import RelicConfig
from relic.mlp import RelicMLP
from relic.model import RelicForCausalLM, RelicModel
from relic.norm import RMSNorm
from relic.rotary import RotaryEmbedding

__all__ = [
    "RelicConfig",
    "RMSNorm",
    "RotaryEmbedding",
    "RelicAttention",
    "RelicMLP",
    "RelicBlock",
    "RelicModel",
    "RelicForCausalLM",
]
