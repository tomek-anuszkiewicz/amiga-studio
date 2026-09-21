---
name: index-amiga-rag
description: >
  Operational procedure for incremental Qdrant vector reindexing of Amiga hardware
  manuals, Obsidian design notes, and circuit schematic sidecars. Covers both amiga
  and obsidian sources, asset sidecar audit, status verification, and offline-only
  constraints. Invoked by doc_ingestor after any ingestion run.
---

# Skill: Incremental Amiga RAG Reindexing

This skill defines the standardized procedure for keeping the local Qdrant vector
database (`amiga` collection) synchronized with reference documentation and design notes
whenever content under `Obsidian/Amiga/` changes.

**Zero cloud API keys required.** SHA-256 hash cache skips unchanged files in < 1s.

---

## 1. When to Trigger

| Trigger | Source Flag |
| :--- | :--- |
| Reference manuals added/modified under `Obsidian/Amiga/Reference/` | `--source amiga` |
| Design specs modified under `Obsidian/Amiga/Design/` | `--source obsidian` |
| Circuit schematic `.txt` sidecars generated or updated | `--source amiga` |
| After any `pdf-to-markdown` or `html-to-markdown` ingestion run | `--source amiga` |
| After modifying personal architecture notes in Obsidian vault | `--source obsidian` |

---

## 2. Infrastructure

- **Vector Database:** Local Qdrant (`http://localhost:6333`, collection: `amiga`)
- **CLI Runner:** `tools/rag/bin/amiga_rag.ps1` (PowerShell entry point)
- **Core Indexer:** `tools/rag/rag_qdrant/indexer.py`
- **Cache File:** Configured via `RAG_CACHE_FILE` in `.env` — stores SHA-256 hashes for incremental skips
- **MCP Tools:** `rag_search`, `rag_status`, `rag_list_sources` (live queries without re-indexing)

---

## 3. Execution Workflow

### Step 1: Audit Diagram & Asset Sidecars (If Images Were Touched)

If any circuit diagrams, timing charts, or pinout schematics were added or modified:

```powershell
python tools/rag/rag_qdrant/assets_manager.py "Obsidian/Amiga" --list-unindexed
```

For any image without a companion `.txt` sidecar, generate one via multimodal vision
per the `describe-diagram-assets` skill before indexing. The indexer embeds the `.txt`
content — not the image pixels — so sidecars must exist first.

### Step 2: Run Incremental Indexing

**Hardware manuals & reference docs (amiga source):**
```powershell
.\tools\rag\bin\amiga_rag.ps1 . --source amiga
```

Or target only the reference vault directly:
```powershell
python tools/rag/rag_qdrant/indexer.py "Obsidian/Amiga/Reference" --source amiga
```

**Personal architecture notes & design specs (obsidian source):**
```powershell
.\tools\rag\bin\amiga_rag.ps1 <PATH_TO_VAULT> --source obsidian
```

The content-hash skip mechanism re-embeds only new or modified chunks. Unchanged files
complete in < 1s regardless of collection size.

### Step 3: Verify Status & Collection Health

```powershell
python tools/rag/rag_qdrant/indexer.py --status
```

Or via MCP tools: `rag_status` → check point count, `rag_list_sources` → confirm both
`amiga` and `obsidian` sources are present.

### Step 4: Smoke-Test Query

Verify the newly indexed content is retrievable with a targeted query:

```powershell
python tools/harness/rag_search.py "<topic from ingested doc>" --source amiga
```

Confirm at least one result returns from the expected document. If zero results appear
for a freshly indexed file, check `.env` for `RAG_CACHE_FILE` path validity and confirm
Qdrant is running (`http://localhost:6333/dashboard`).

---

## 4. Offline Diagram Vision Workflow

Circuit schematics and timing diagrams use Git-tracked sidecar text files:

1. Agent inspects the image via multimodal vision (`view_file` on the binary).
2. Agent authors `<image_path>.txt` with a structured technical description:
   - Signal names, bus widths, clock domains, pin labels
   - Causal flow (input → logic → output)
   - Any timing constraints or phase relationships visible in the diagram
3. Sidecar is committed alongside the image.
4. Next indexing run automatically picks up the `.txt` and embeds its content.

---

## 5. Troubleshooting

| Symptom | Fix |
| :--- | :--- |
| `rag_search` returns zero results for a new doc | Check `RAG_CACHE_FILE` in `.env`; delete stale cache and re-run |
| Indexer errors on a specific file | Check for encoding issues; ensure `.txt` sidecars exist for all images |
| Qdrant connection refused | Start Qdrant: `docker run -p 6333:6333 qdrant/qdrant` |
| Old content still surfacing after deletion | Qdrant requires explicit point deletion; re-run with `--rebuild` flag if supported |
