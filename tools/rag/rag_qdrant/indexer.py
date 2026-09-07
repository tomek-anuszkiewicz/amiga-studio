import os
import json
import uuid
import hashlib
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional
from concurrent.futures import ThreadPoolExecutor, as_completed

from qdrant_client import QdrantClient
from qdrant_client.http import models

from .config import (
    QDRANT_URL,
    COLLECTION_NAME,
    QDRANT_TIMEOUT,
    CACHE_FILE,
    EMBEDDING_PROVIDER,
    EMBEDDING_MODEL,
    EMBEDDING_DIM,
    GEMINI_API_KEY,
    NUM_WORKERS,
    VISION_MAX_WORKERS,
    EMBEDDING_BATCH_SIZE,
    UPSERT_BATCH_SIZE,
)
from .chunker import MarkdownChunker
from .vision import VisionAnalyzer


class KnowledgeIndexer:
    def __init__(self):
        self.client = QdrantClient(url=QDRANT_URL, timeout=QDRANT_TIMEOUT)
        self.chunker = MarkdownChunker()
        self.cache = self._load_cache()
        self.vision = VisionAnalyzer(self.cache.setdefault("image_descriptions", {}))
        self.fastembed_model = None
        self._init_embedder()

    def _init_embedder(self):
        try:
            from fastembed import TextEmbedding
            self.fastembed_model = TextEmbedding(
                model_name=EMBEDDING_MODEL,
                threads=NUM_WORKERS
            )
        except Exception as e:
            print(f"[Warning] Failed to load FastEmbed model: {e}")

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
        """Fetch embeddings via CPU-based FastEmbed (multi-core local)."""
        if not texts:
            return []

        if not self.fastembed_model:
            raise RuntimeError("FastEmbed model failed to initialize or is unavailable.")

        # FastEmbed local multi-threaded inference via ONNX Runtime C++ engine (threads=NUM_WORKERS)
        embeddings = [
            v.tolist() for v in self.fastembed_model.embed(
                texts,
                batch_size=EMBEDDING_BATCH_SIZE,
                parallel=None
            )
        ]
        return embeddings

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
        progress_cb=None,
        plan_cb=None
    ) -> Dict[str, Any]:
        """Indexes all markdown files and referenced images from directory using multi-core parallelism."""
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

        # 1. Clean up files deleted on disk from Qdrant and cache
        cached_paths = list(source_cache.keys())
        for old_path in cached_paths:
            if old_path not in active_paths:
                self.delete_file_points(old_path)
                del source_cache[old_path]
                stats["deleted"] += 1

        if not md_files:
            self._save_cache()
            return stats

        # 2. Parallel scan & hash to identify modified/new vs skipped files
        def _check_file(file_p: Path):
            p_str = str(file_p.resolve())
            h = self._file_hash(file_p)
            size = file_p.stat().st_size
            cached_entry = source_cache.get(p_str)
            needs_idx = force or (cached_entry is None) or (cached_entry.get("hash") != h)
            is_upd = cached_entry is not None and needs_idx
            return file_p, p_str, h, needs_idx, is_upd, size

        total_scanned_bytes = sum(f.stat().st_size for f in md_files)
        total_files = len(md_files)
        files_to_index = []
        total_to_index_bytes = 0
        total_skipped_bytes = 0
        scanned_bytes = 0
        scanned_count = 0

        if progress_cb:
            progress_cb(0, total_scanned_bytes, f"Checking {total_files} files across {NUM_WORKERS} workers...", "hashing", True, f"({0:>{len(str(total_files))}}/{total_files})")

        with ThreadPoolExecutor(max_workers=NUM_WORKERS) as executor:
            try:
                futures = {executor.submit(_check_file, p): p for p in md_files}
                for future in as_completed(futures):
                    file_p, p_str, h, needs_idx, is_upd, size = future.result()
                    scanned_count += 1
                    scanned_bytes += size
                    if needs_idx:
                        files_to_index.append((file_p, p_str, h, is_upd, size))
                        total_to_index_bytes += size
                    else:
                        stats["skipped"] += 1
                        total_skipped_bytes += size

                    if progress_cb:
                        count_tag = f"({scanned_count:>{len(str(total_files))}}/{total_files})"
                        progress_cb(scanned_bytes, total_scanned_bytes, file_p.name, "hashing", True, count_tag)
            except KeyboardInterrupt:
                executor.shutdown(wait=False, cancel_futures=True)
                raise

        if plan_cb:
            plan_cb({
                "scanned_files": total_files,
                "scanned_bytes": total_scanned_bytes,
                "to_index_files": len(files_to_index),
                "to_index_bytes": total_to_index_bytes,
                "skipped_files": stats["skipped"],
                "skipped_bytes": total_skipped_bytes,
                "cpu_workers": NUM_WORKERS,
                "vision_workers": VISION_MAX_WORKERS,
            })

        if not files_to_index:
            if progress_cb:
                progress_cb(total_scanned_bytes, total_scanned_bytes, "All files up to date", "skipped", True, f"({total_files}/{total_files})")
            self._save_cache()
            return stats

        # 3. Parallel markdown chunking across CPU workers
        chunked_results = []
        all_referenced_images = set()

        def _chunk_worker(item):
            file_p, p_str, h, is_upd, size = item
            chunks = self.chunker.chunk_markdown(file_p, dir_path, source_name)
            return file_p, p_str, h, is_upd, size, chunks

        processed_bytes = 0
        chunked_count = 0
        total_to_chunk = len(files_to_index)
        chunk_digits = len(str(total_to_chunk))
        with ThreadPoolExecutor(max_workers=NUM_WORKERS) as executor:
            try:
                futures = [executor.submit(_chunk_worker, item) for item in files_to_index]
                for future in as_completed(futures):
                    file_p, p_str, h, is_upd, size, chunks = future.result()
                    chunked_count += 1
                    processed_bytes += size
                    chunked_results.append((file_p, p_str, h, is_upd, chunks))
                    for ch in chunks:
                        all_referenced_images.update(ch.get("images", []))
                    if progress_cb:
                        count_tag = f"({chunked_count:>{chunk_digits}}/{total_to_chunk})"
                        progress_cb(processed_bytes, total_to_index_bytes, file_p.name, "chunked", True, count_tag)
            except KeyboardInterrupt:
                executor.shutdown(wait=False, cancel_futures=True)
                raise

        # 4. Parallel Vision Analysis for referenced diagrams/images
        if all_referenced_images and self.vision.available:
            total_images = len(all_referenced_images)
            v_digits = len(str(total_images))
            def _vision_progress(completed, total, name):
                if progress_cb:
                    count_tag = f"({completed:>{v_digits}}/{total})"
                    progress_cb(completed, total, name, "vision", False, count_tag)

            self.vision.analyze_images_parallel(
                list(all_referenced_images),
                max_workers=VISION_MAX_WORKERS,
                progress_cb=_vision_progress
            )
            stats["images_analyzed"] = len(self.cache.get("image_descriptions", {}))
            self._save_cache()

        # 5. Incremental File-by-File Embedding, Upserting & Immediate Cache Checkpointing
        total_chunks_to_index = sum(len(chunks) for _, _, _, _, chunks in chunked_results)
        total_files_to_index = len(chunked_results)
        completed_chunks = 0
        completed_files = 0
        f_digits = len(str(total_files_to_index))
        c_digits = len(str(total_chunks_to_index))

        if progress_cb and total_chunks_to_index > 0:
            progress_cb(0, total_chunks_to_index, f"Starting incremental indexing across {total_files_to_index} files...", "indexing", False, f"({0:>{f_digits}}/{total_files_to_index})")

        for file_p, p_str, h, is_upd, chunks in chunked_results:
            completed_files += 1
            if not chunks:
                source_cache[p_str] = {
                    "hash": h,
                    "chunks": 0,
                    "last_indexed": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                }
                self._save_cache()
                stats["skipped"] += 1
                if progress_cb:
                    count_tag = f"({completed_files:>{f_digits}}/{total_files_to_index})"
                    progress_cb(completed_chunks, total_chunks_to_index, file_p.name, "skipped", False, count_tag)
                continue

            file_point_tuples = []
            file_texts = []
            for chunk in chunks:
                chunk_text = chunk["content"]
                img_desc_list = []
                for img_p in chunk.get("images", []):
                    desc = self.vision.analyze_image(img_p)
                    if desc:
                        img_desc_list.append(f"Image [{Path(img_p).name}]: {desc}")

                full_embed_text = chunk_text
                if img_desc_list:
                    full_embed_text += "\n\n[Associated Diagrams & OCR]:\n" + "\n".join(img_desc_list)

                point_id = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{source_name}:{chunk['chunk_id']}"))
                payload = {
                    "source": source_name,
                    "file_path": chunk["file_path"],
                    "relative_path": chunk["relative_path"],
                    "header": chunk["header"],
                    "content": chunk["content"],
                    "full_context": full_embed_text,
                    "images": chunk["images"],
                    "chunk_id": chunk["chunk_id"]
                }
                file_texts.append(full_embed_text)
                file_point_tuples.append((point_id, payload))

            # Compute embeddings for this file's chunks
            embeddings = self.get_embeddings(file_texts)

            # Build Qdrant PointStructs
            file_points = [
                models.PointStruct(id=pt[0], vector=emb, payload=pt[1])
                for pt, emb in zip(file_point_tuples, embeddings)
            ]

            # Delete old points for this file if it was an update
            if is_upd:
                self.delete_file_points(p_str)

            # Immediately upsert points for this file to Qdrant
            for i in range(0, len(file_points), UPSERT_BATCH_SIZE):
                self.client.upsert(
                    collection_name=COLLECTION_NAME,
                    points=file_points[i:i + UPSERT_BATCH_SIZE]
                )

            # Immediately checkpoint this file to disk!
            source_cache[p_str] = {
                "hash": h,
                "chunks": len(file_points),
                "last_indexed": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            }
            self._save_cache()

            completed_chunks += len(chunks)
            stats["total_points"] += len(file_points)
            if is_upd:
                stats["updated"] += 1
            else:
                stats["indexed"] += 1

            if progress_cb:
                count_tag = f"({completed_files:>{f_digits}}/{total_files_to_index})"
                progress_cb(completed_chunks, total_chunks_to_index, file_p.name, "indexing", False, count_tag)

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

    def get_qdrant_sources_stats(self) -> Dict[str, Any]:
        """Queries Qdrant live database points count for collection and per source."""
        self.ensure_collection()
        try:
            info = self.client.get_collection(COLLECTION_NAME)
            total_points = info.points_count
        except Exception:
            total_points = 0

        sources = ["amiga", "obsidian"]
        for s in self.cache.get("sources", {}).keys():
            if s not in sources:
                sources.append(s)

        result = {"total_points": total_points, "total_files": 0, "sources": {}}
        total_cached_files = 0
        for s in sorted(sources):
            try:
                cnt = self.client.count(
                    collection_name=COLLECTION_NAME,
                    count_filter=models.Filter(
                        must=[models.FieldCondition(key="source", match=models.MatchValue(value=s))]
                    ),
                    exact=True
                ).count
            except Exception:
                cnt = 0
            cached_files = len(self.cache.get("sources", {}).get(s, {}))
            total_cached_files += cached_files
            result["sources"][s] = {
                "vectors": cnt,
                "cached_files": cached_files
            }
        result["total_files"] = total_cached_files
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
