import os
import json
import uuid
import hashlib
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional

from qdrant_client import QdrantClient
from qdrant_client.http import models

from .config import (
    QDRANT_URL,
    COLLECTION_NAME,
    CACHE_FILE,
    EMBEDDING_PROVIDER,
    EMBEDDING_MODEL,
    EMBEDDING_DIM,
    GEMINI_API_KEY,
)
from .chunker import MarkdownChunker
from .vision import VisionAnalyzer


class KnowledgeIndexer:
    def __init__(self):
        self.client = QdrantClient(url=QDRANT_URL)
        self.chunker = MarkdownChunker()
        self.cache = self._load_cache()
        self.vision = VisionAnalyzer(self.cache.setdefault("image_descriptions", {}))
        self.fastembed_model = None
        self.genai_client = None
        self._init_embedder()

    def _init_embedder(self):
        if EMBEDDING_PROVIDER == "fastembed":
            try:
                from fastembed import TextEmbedding
                self.fastembed_model = TextEmbedding(model_name=EMBEDDING_MODEL)
            except Exception as e:
                print(f"[Warning] Failed to load FastEmbed model: {e}")
        elif EMBEDDING_PROVIDER == "gemini":
            if GEMINI_API_KEY:
                try:
                    from google import genai
                    self.genai_client = genai.Client(api_key=GEMINI_API_KEY)
                except Exception:
                    pass

    def _load_cache(self) -> Dict[str, Any]:
        if CACHE_FILE.is_file():
            try:
                with open(CACHE_FILE, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                pass
        return {"sources": {}, "image_descriptions": {}}

    def _save_cache(self):
        CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(CACHE_FILE, "w", encoding="utf-8") as f:
            json.dump(self.cache, f, indent=2, ensure_ascii=False)

    def _file_hash(self, file_path: Path) -> str:
        hasher = hashlib.sha256()
        with open(file_path, "rb") as f:
            while chunk := f.read(65536):
                hasher.update(chunk)
        return hasher.hexdigest()

    def ensure_collection(self, force_recreate: bool = False):
        """Creates Qdrant collection if not already existing or reconciles dimension."""
        collections = [c.name for c in self.client.get_collections().collections]
        if COLLECTION_NAME in collections:
            info = self.client.get_collection(COLLECTION_NAME)
            current_dim = info.config.params.vectors.size
            if current_dim != EMBEDDING_DIM and (force_recreate or info.points_count == 0):
                # Recreate collection if dimension changed
                self.client.delete_collection(COLLECTION_NAME)
                collections.remove(COLLECTION_NAME)

        if COLLECTION_NAME not in collections:
            self.client.create_collection(
                collection_name=COLLECTION_NAME,
                vectors_config=models.VectorParams(
                    size=EMBEDDING_DIM,
                    distance=models.Distance.COSINE
                )
            )
            # Create payload index on 'source' field for fast filtering
            self.client.create_payload_index(
                collection_name=COLLECTION_NAME,
                field_name="source",
                field_schema=models.PayloadSchemaType.KEYWORD
            )

    def get_embeddings(self, texts: List[str]) -> List[List[float]]:
        """Fetch embeddings via FastEmbed (local) or Gemini API."""
        if not texts:
            return []

        if self.fastembed_model:
            # FastEmbed local inference
            embeddings = [v.tolist() for v in self.fastembed_model.embed(texts)]
            return embeddings

        if self.genai_client:
            embeddings = []
            batch_size = 50
            for i in range(0, len(texts), batch_size):
                batch = texts[i:i + batch_size]
                res = self.genai_client.models.embed_content(
                    model=EMBEDDING_MODEL,
                    contents=batch
                )
                for emb in res.embeddings:
                    embeddings.append(emb.values)
            return embeddings

        raise RuntimeError("No embedding provider configured or available.")

    def delete_file_points(self, file_path_str: str):
        """Removes all points associated with a specific file path."""
        self.client.delete(
            collection_name=COLLECTION_NAME,
            points_selector=models.FilterSelector(
                filter=models.Filter(
                    must=[
                        models.FieldCondition(
                            key="file_path",
                            match=models.MatchValue(value=file_path_str)
                        )
                    ]
                )
            )
        )

    def index_directory(
        self,
        directory: Path,
        source_name: str,
        force: bool = False,
        progress_cb=None
    ) -> Dict[str, Any]:
        """Indexes all markdown files and referenced images from directory incrementally."""
        self.ensure_collection(force_recreate=force)
        dir_path = directory.resolve()
        source_name = source_name.strip().lower()
        source_cache = self.cache.setdefault("sources", {}).setdefault(source_name, {})

        # Discover all markdown files, ignoring hidden and temporary folders
        ignored_parts = {".git", ".antigravity", ".venv", "venv", "__pycache__", "node_modules", ".system_generated"}
        md_files = [
            p for p in dir_path.rglob("*.md")
            if not any(part in ignored_parts for part in p.parts)
        ]
        active_paths = {str(p.resolve()) for p in md_files}

        stats = {
            "scanned": len(md_files),
            "indexed": 0,
            "updated": 0,
            "skipped": 0,
            "deleted": 0,
            "total_points": 0,
            "images_analyzed": 0
        }

        # Clean up files deleted on disk from Qdrant and cache
        cached_paths = list(source_cache.keys())
        for old_path in cached_paths:
            if old_path not in active_paths:
                self.delete_file_points(old_path)
                del source_cache[old_path]
                stats["deleted"] += 1

        # Process active files
        for i, file_path in enumerate(md_files):
            file_path_str = str(file_path.resolve())
            current_hash = self._file_hash(file_path)

            file_cached = source_cache.get(file_path_str)
            if not force and file_cached and file_cached.get("hash") == current_hash:
                stats["skipped"] += 1
                if progress_cb:
                    progress_cb(i + 1, len(md_files), file_path.name, "skipped")
                continue

            # File is new or modified: remove old vectors if existing
            if file_cached:
                self.delete_file_points(file_path_str)
                is_update = True
            else:
                is_update = False

            # Chunk file
            chunks = self.chunker.chunk_markdown(file_path, dir_path, source_name)
            if not chunks:
                source_cache[file_path_str] = {
                    "hash": current_hash,
                    "chunks": 0,
                    "last_indexed": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                }
                stats["skipped"] += 1
                continue

            # Process vision/images for chunks
            texts_to_embed = []
            for chunk in chunks:
                chunk_text = chunk["content"]
                img_desc_list = []
                for img_p in chunk["images"]:
                    desc = self.vision.analyze_image(img_p)
                    if desc:
                        img_desc_list.append(f"Image [{Path(img_p).name}]: {desc}")
                        stats["images_analyzed"] += 1

                full_embed_text = chunk_text
                if img_desc_list:
                    full_embed_text += "\n\n[Associated Diagrams & OCR]:\n" + "\n".join(img_desc_list)

                texts_to_embed.append(full_embed_text)

            # Generate embeddings
            embeddings = self.get_embeddings(texts_to_embed)

            # Build Qdrant points
            points = []
            for chunk, emb, embed_text in zip(chunks, embeddings, texts_to_embed):
                point_id = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{source_name}:{chunk['chunk_id']}"))
                payload = {
                    "source": source_name,
                    "file_path": chunk["file_path"],
                    "relative_path": chunk["relative_path"],
                    "header": chunk["header"],
                    "content": chunk["content"],
                    "full_context": embed_text,
                    "images": chunk["images"],
                    "chunk_id": chunk["chunk_id"]
                }
                points.append(models.PointStruct(id=point_id, vector=emb, payload=payload))

            # Upsert into Qdrant
            if points:
                self.client.upsert(collection_name=COLLECTION_NAME, points=points)
                stats["total_points"] += len(points)

            source_cache[file_path_str] = {
                "hash": current_hash,
                "chunks": len(points),
                "last_indexed": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            }

            if is_update:
                stats["updated"] += 1
            else:
                stats["indexed"] += 1

            if progress_cb:
                status_str = "updated" if is_update else "indexed"
                progress_cb(i + 1, len(md_files), file_path.name, status_str)

        self._save_cache()
        return stats

    def get_sources_stats(self) -> List[Dict[str, Any]]:
        """Returns statistics of all indexed sources."""
        self.cache = self._load_cache()
        sources = self.cache.get("sources", {})
        result = []
        for name, files in sources.items():
            total_files = len(files)
            total_chunks = sum(f.get("chunks", 0) for f in files.values())
            timestamps = [f.get("last_indexed", "") for f in files.values() if f.get("last_indexed")]
            last_updated = max(timestamps) if timestamps else "N/A"
            result.append({
                "source": name,
                "files_count": total_files,
                "chunks_count": total_chunks,
                "last_updated": last_updated
            })
        return result

    def search(
        self,
        query: str,
        sources: Optional[List[str]] = None,
        limit: int = 5,
        score_threshold: float = 0.50
    ) -> List[Dict[str, Any]]:
        """Semantic search in Qdrant with optional multi-source filtering."""
        query_embeddings = self.get_embeddings([query])
        if not query_embeddings:
            return []
        query_vector = query_embeddings[0]

        query_filter = None
        if sources:
            if isinstance(sources, str):
                sources = [s.strip().lower() for s in sources.split(",") if s.strip()]
            elif isinstance(sources, list):
                sources = [s.strip().lower() for s in sources if isinstance(s, str) and s.strip()]

            if len(sources) == 1:
                query_filter = models.Filter(
                    must=[models.FieldCondition(key="source", match=models.MatchValue(value=sources[0]))]
                )
            elif len(sources) > 1:
                query_filter = models.Filter(
                    must=[models.FieldCondition(key="source", match=models.MatchAny(any=sources))]
                )

        results = self.client.query_points(
            collection_name=COLLECTION_NAME,
            query=query_vector,
            query_filter=query_filter,
            limit=limit,
            score_threshold=score_threshold
        )

        formatted = []
        for hit in results.points:
            payload = hit.payload or {}
            formatted.append({
                "score": round(hit.score, 4),
                "source": payload.get("source"),
                "file_path": payload.get("file_path"),
                "relative_path": payload.get("relative_path"),
                "header": payload.get("header"),
                "content": payload.get("content"),
                "images": payload.get("images", [])
            })
        return formatted
