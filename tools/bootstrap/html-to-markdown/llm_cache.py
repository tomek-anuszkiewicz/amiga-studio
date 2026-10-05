#!/usr/bin/env python3
"""
llm_cache.py: Content-addressable local disk cache for Gemini API requests.
Stores cached responses in Obsidian/Amiga/Reference/.cache/gemini.
"""

import hashlib
import json
import os
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Any

# Resolve repository root and reference cache directory
REPO_ROOT = Path(__file__).resolve().parents[3]
CACHE_DIR = REPO_ROOT / "Obsidian" / "Amiga" / "Reference" / ".cache" / "gemini"


def compute_image_hash(img: Any) -> str:
    """Compute sha256 hash of PIL Image or raw bytes."""
    h = hashlib.sha256()
    try:
        h.update(str(img.size).encode("utf-8"))
        h.update(str(img.mode).encode("utf-8"))
        h.update(img.tobytes())
    except Exception:
        if isinstance(img, (bytes, bytearray)):
            h.update(img)
        else:
            h.update(str(img).encode("utf-8"))
    return h.hexdigest()


def compute_cache_key(
    model: str,
    prompt: str,
    images: Optional[List[Any]] = None,
    budget: Optional[int] = None,
    temperature: Optional[float] = None,
    response_mime_type: Optional[str] = None,
) -> str:
    """Computes a deterministic SHA-256 cache key from request parameters."""
    h = hashlib.sha256()
    h.update(f"model:{model}\n".encode("utf-8"))
    h.update(f"temp:{temperature}\n".encode("utf-8"))
    h.update(f"budget:{budget}\n".encode("utf-8"))
    h.update(f"mime:{response_mime_type}\n".encode("utf-8"))
    h.update(f"prompt:{prompt}\n".encode("utf-8"))
    if images:
        for idx, img in enumerate(images):
            h.update(f"img_{idx}:{compute_image_hash(img)}\n".encode("utf-8"))
    return h.hexdigest()


def get_cache_path(cache_key: str) -> Path:
    """Returns path to cached json file, partitioned by first 2 chars."""
    sub = cache_key[:2]
    d = CACHE_DIR / sub
    d.mkdir(parents=True, exist_ok=True)
    return d / f"{cache_key}.json"


def load_from_cache(cache_key: str) -> Optional[str]:
    """Retrieves cached response text if available."""
    p = get_cache_path(cache_key)
    if p.exists():
        try:
            with open(p, "r", encoding="utf-8") as f:
                data = json.load(f)
                return data.get("response_text")
        except Exception:
            return None
    return None


def save_to_cache(cache_key: str, model: str, prompt: str, response_text: str):
    """Atomically writes response to disk cache."""
    p = get_cache_path(cache_key)
    tmp = p.with_suffix(".tmp")
    data = {
        "key": cache_key,
        "model": model,
        "created_at": datetime.now().isoformat(),
        "prompt_preview": prompt[:200] if prompt else "",
        "response_text": response_text,
    }
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False)
        os.replace(tmp, p)
    except Exception:
        try:
            tmp.unlink(missing_ok=True)
        except Exception:
            pass
