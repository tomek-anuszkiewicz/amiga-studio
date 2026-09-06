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
QDRANT_URL = os.getenv("QDRANT_URL", "http://localhost:6333")
COLLECTION_NAME = os.getenv("QDRANT_COLLECTION", "unified_knowledge")
CACHE_FILE = Path(os.getenv("RAG_CACHE_FILE", r"D:\GoogleDrive\AI\qdrant\rag_cache.json"))

# Embedding settings: 'fastembed' (local, free, 0 API quota) or 'gemini' (cloud)
EMBEDDING_PROVIDER = os.getenv("RAG_EMBEDDING_PROVIDER", "fastembed").lower()
GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")

if EMBEDDING_PROVIDER == "gemini":
    EMBEDDING_MODEL = os.getenv("RAG_EMBEDDING_MODEL", "gemini-embedding-001")
    EMBEDDING_DIM = 768
else:
    EMBEDDING_MODEL = os.getenv("RAG_EMBEDDING_MODEL", "BAAI/bge-small-en-v1.5")
    EMBEDDING_DIM = 384

VISION_MODEL = os.getenv("RAG_VISION_MODEL", "gemini-2.5-flash")

# Default source tag fallback
DEFAULT_SOURCE = os.getenv("RAG_DEFAULT_SOURCE")
