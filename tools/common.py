"""Common configuration constants and filesystem paths for RELIC-10M.
Keep clean and under 180 lines.
"""

from pathlib import Path

# Filesystem layout
ROOT_DIR: Path = Path(__file__).resolve().parent.parent
DATA_DIR: Path = ROOT_DIR / "data"
ARTIFACTS_DIR: Path = ROOT_DIR / "artifacts"
REPORTS_DIR: Path = ROOT_DIR / "reports"
TOKENIZER_DIR: Path = ARTIFACTS_DIR / "tokenizer"

# Dataset separation
STORY_SEP: str = "\n<|endoftext|>\n"

# Special tokens
BOS_TOKEN: str = "<|bos|>"
EOS_TOKEN: str = "<|eos|>"
PAD_TOKEN: str = "<|pad|>"
SPECIAL_TOKENS: list[str] = [BOS_TOKEN, EOS_TOKEN, PAD_TOKEN]

# Architecture specifications
VOCAB_SIZE: int = 8192
HIDDEN_SIZE: int = 288
INTERMEDIATE_SIZE: int = 768
NUM_HIDDEN_LAYERS: int = 6
NUM_ATTENTION_HEADS: int = 6
NUM_KEY_VALUE_HEADS: int = 2
HEAD_DIM: int = 48
MAX_POSITION_EMBEDDINGS: int = 512
MAX_TRAIN_SEQ_LEN: int = 256
ROPE_THETA: float = 10000.0
RMS_NORM_EPS: float = 1e-6
