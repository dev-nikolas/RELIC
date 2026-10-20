"""Configuracion de arquitectura para el modelo RELIC-10M.
Diseno modular, tipado estricto y menos de 180 lineas.
"""

from dataclasses import dataclass


@dataclass
class RelicConfig:
    """Hiperparametros arquitectonicos exactos para RELIC-10M."""

    vocab_size: int = 8192
    hidden_size: int = 288
    intermediate_size: int = 768
    num_hidden_layers: int = 6
    num_attention_heads: int = 6
    num_key_value_heads: int = 2
    head_dim: int = 48
    max_position_embeddings: int = 512
    max_train_seq_len: int = 256
    rope_theta: float = 10000.0
    rms_norm_eps: float = 1e-6
    attention_bias: bool = False
    tie_word_embeddings: bool = False
    initializer_range: float = 0.02

    def __post_init__(self) -> None:
        """Valida invariantes fundamentales de la arquitectura."""
        if self.hidden_size != self.num_attention_heads * self.head_dim:
            raise ValueError(
                f"hidden_size ({self.hidden_size}) != num_heads ({self.num_attention_heads}) * head_dim ({self.head_dim})"
            )
        if self.num_attention_heads % self.num_key_value_heads != 0:
            raise ValueError(
                f"num_attention_heads ({self.num_attention_heads}) must be divisible by num_key_value_heads ({self.num_key_value_heads})"
            )
