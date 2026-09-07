---
trigger: when assets change, or on demand ("describe assets", "update image descriptions", "generate image descriptions")
description: Inspect diagrams and generate Git-tracked sidecar text descriptions (.txt) using Agent native multimodal vision, driven by RAG_CACHE_FILE hashes.
---

## Diagram & Asset Sidecar Descriptions

All circuit diagrams, timing diagrams, pinouts, and architecture schematics in documentation assets (e.g. `Obsidian/Amiga/Reference/*/assets/`) maintain corresponding Git-tracked text description files: `<image_path>.txt`.

These sidecar files allow the local RAG indexer (`amiga_rag`) to incorporate rich diagram details into vector embeddings 100% offline without requiring cloud API keys or runtime OCR.

### Change Detection via `RAG_CACHE_FILE`
- Image indexing status is strictly governed by the centralized RAG cache file (`amiga_rag_cache.json` specified by `RAG_CACHE_FILE` in `.env`).
- To find assets that need new or updated descriptions, run:
  ```powershell
  python tools/rag/rag_qdrant/assets_manager.py "Obsidian/Amiga" --list-unindexed
  ```
- An image requires description when:
  1. Its SHA256 hash is missing from `image_descriptions` in `CACHE_FILE`.
  2. Its content hash has changed (`hash_mismatch`).
  3. Its `<image_path>.txt` sidecar file is missing on disk.

### Generation Workflow (Agent Native Multimodal Vision)
When requested by the user or when assets are modified:
1. Identify unindexed images using `AssetsManager.get_unindexed_images()`.
2. Inspect the image natively using `view_file` (consuming Antigravity IDE's internal multimodal model quota, not external API keys).
3. Generate a structured, factual technical description adhering to this standard:
   ```markdown
   [Diagram: <Figure Title / Diagram Name>]
   - Subsystem / Component: <e.g. M68000 Bus Arbitration, Paula Audio, Copper Timing>
   - Circuit / Signals / Pinouts: <Extracted signal names: /AS, /DTACK, /BERR, D0-D15, etc.>
   - State & Cycle Transitions: <Timing steps, clock edges, wait states, microcode sequence>
   - Architectural Summary: <Core engineering takeaway and cycle-exact behavior>
   ```
4. Save the description into `<image_path>.txt` and update the SHA256 hash in `RAG_CACHE_FILE` using `AssetsManager.record_description()`.
5. Keep both image and sidecar `.txt` version-controlled in Git.
