"""Dataset y dataloader eficiente para entrenamiento autoregresivo de RELIC-10M.
Tokeniza una sola vez, cachea en disco como uint16 y soporta streaming en bloques.
Menos de 180 líneas.
"""

from pathlib import Path
import numpy as np
import torch
from torch.utils.data import Dataset
from tokenizers import Tokenizer


class StoryDataset(Dataset):
    """Dataset que carga secuencias de tokens cacheadas en disco en bloques de longitud fija."""

    def __init__(
        self,
        file_path: Path | str,
        tokenizer: Tokenizer,
        seq_len: int = 256,
        force_retokenize: bool = False,
    ) -> None:
        self.seq_len = seq_len
        path = Path(file_path)
        bin_path = path.with_suffix(".bin")

        if bin_path.exists() and not force_retokenize:
            # Carga instantánea de tokens precalculados desde disco (uint16)
            arr = np.fromfile(bin_path, dtype=np.uint16)
            token_ids = torch.from_numpy(arr.astype(np.int64))
        else:
            if not path.exists():
                raise FileNotFoundError(f"No se encontró el archivo de texto en {path}")
            with open(path, "r", encoding="utf-8") as f:
                text = f.read()

            enc = tokenizer.encode(text)
            ids_arr = np.array(enc.ids, dtype=np.uint16)
            ids_arr.tofile(bin_path)
            token_ids = torch.from_numpy(ids_arr.astype(np.int64))

        block_size = seq_len + 1
        num_blocks = len(token_ids) // block_size
        if num_blocks == 0:
            raise ValueError(f"Tokens insuficientes en {path} ({len(token_ids)}) para bloques de {block_size}")

        trimmed_len = num_blocks * block_size
        self.data = token_ids[:trimmed_len]
        self.num_blocks = num_blocks

    def __len__(self) -> int:
        return self.num_blocks

    def __getitem__(self, idx: int) -> tuple[torch.Tensor, torch.Tensor]:
        start = idx * (self.seq_len + 1)
        chunk = self.data[start : start + self.seq_len + 1]
        x = chunk[:-1]
        y = chunk[1:]
        return x, y
