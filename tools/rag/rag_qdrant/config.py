import os
from pathlib import Path
from dotenv import load_dotenv

# Try loading .env from current working directory first, then fallback to Amiga repo
cwd_env = Path.cwd() / ".env"
amiga_env = Path(__file__).resolve().parents[3] / ".env"

if cwd_env.is_file():
    load_dotenv(dotenv_path=cwd_env)
elif amiga_env.is_file():
    load_dotenv(dotenv_path=amiga_env)
else:
    load_dotenv()

# Qdrant settings
QDRANT_URL = "http://localhost:6333"
QDRANT_TIMEOUT = 60.0

COLLECTION_NAME = "amiga"

cache_env = os.getenv("RAG_CACHE_FILE")
if not cache_env or not cache_env.strip():
    raise RuntimeError("RAG_CACHE_FILE environment variable is mandatory and must be defined in .env")
CACHE_FILE = Path(cache_env.strip('"\''))

# Embedding settings: strictly CPU-based local embeddings (FastEmbed)
EMBEDDING_PROVIDER = "fastembed"
EMBEDDING_MODEL = "BAAI/bge-base-en-v1.5"
EMBEDDING_DIM = 768
GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")

VISION_MODEL = "gemini-flash-latest"

# Concurrency & Parallelism settings (calculated from hardware)
NUM_WORKERS = os.cpu_count() or 4
VISION_MAX_WORKERS = os.cpu_count() or 4
EMBEDDING_BATCH_SIZE = 64
UPSERT_BATCH_SIZE = 150
