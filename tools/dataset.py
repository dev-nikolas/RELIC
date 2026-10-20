"""Dataset y dataloader eficiente para entrenamiento autoregresivo de RELIC-10M.
Diseno limpio, sin fugas de memoria y menos de 180 lineas.
"""

from pathlib import Path
import torch
from torch.utils.data import Dataset
from tokenizers import Tokenizer


class StoryDataset(Dataset):
    """Dataset que tokeniza y segmenta el texto plano en secuencias de longitud fija."""

    def __init__(self, file_path: Path | str, tokenizer: Tokenizer, seq_len: int = 256) -> None:
        self.seq_len = seq_len
        with open(file_path, "r", encoding="utf-8") as f:
            text = f.read()

        # Codificar texto completo a una secuencia continua de IDs
        enc = tokenizer.encode(text)
        token_ids = enc.ids

        # Descartar remanente para division exacta en bloques de seq_len + 1 (input y target)
        block_size = seq_len + 1
        num_blocks = len(token_ids) // block_size
        if num_blocks == 0:
            raise ValueError(f"El texto en {file_path} es demasiado corto para bloques de {block_size}")

        trimmed_len = num_blocks * block_size
        self.data = torch.tensor(token_ids[:trimmed_len], dtype=torch.long)
        self.num_blocks = num_blocks

    def __len__(self) -> int:
        return self.num_blocks

    def __getitem__(self, idx: int) -> tuple[torch.Tensor, torch.Tensor]:
        start = idx * (self.seq_len + 1)
        chunk = self.data[start : start + self.seq_len + 1]
        x = chunk[:-1]
        y = chunk[1:]
        return x, y
