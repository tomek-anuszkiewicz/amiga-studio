# Stage 01: Preprocess

## Objective
Deconstructs a physical input PDF into atomic, per-page representations to serve as clean ground truth for all downstream stages.

Stage 01 handles both digital and physical documents through two unified processing pathways:
1. **Born-Digital PDF Pathway (Direct Extraction):**
   - Renders 300 DPI PNGs and extracts native text blocks and page dimensions via PyMuPDF (`fitz.get_text("blocks")`).
   - Runs deterministically with zero network calls and sub-second latency.
2. **Scanned / Visual PDF Pathway (Codex Vision OCR):**
   - Automatically detects pages lacking healthy native text (`total_chars < threshold`, default 20, or 0 text blocks).
   - Triggers Codex Vision OCR worker (`detect_and_ocr.py`) using a single-pass 3-way triage prompt (`prompt_ocr.md`: `text_page`, `pure_graphic`, `blank`).
   - Extracts structured text blocks with normalized integer millirange coordinates `box_2d: [ymin, xmin, ymax, xmax]` in range `[0..1000]`.

> [!IMPORTANT]
> **Uniform Artifact Contracts:**  
> Regardless of whether a page is processed via born-digital text extraction or scanned Codex Vision OCR, Stage 01 outputs the **exact same uniform artifact contracts** in `workspace/01_preprocess/`. Downstream stages (02 through 13) are completely agnostic to whether the source page was a vector PDF or a paper scan.

---

## File Structure

```text
stages/01_preprocess/
├── preprocess.py          # Main entry point: deconstruction, page extraction, and auto-OCR orchestration
├── detect_and_ocr.py      # Internal OCR helper: detect_and_ocr_pages() called by preprocess.py
├── prompt_ocr.md          # Multimodal triage & OCR prompt
└── README.md              # Stage documentation and contracts
```

---

## Inputs
- `<manual.pdf>`: Source PDF document passed via `--pdf`.
- `config.yaml`: Global rendering and OCR threshold configuration.
- `stages/01_preprocess/prompt_ocr.md`: Vision prompt (used if scanned pages trigger OCR).

## Outputs
- `workspace/01_preprocess/page_XXXX.png`: Raster page at the configured DPI (default 300), used for vision classification and all downstream visual crops.
- `workspace/01_preprocess/page_XXXX.json`: Text geometry extraction with word spans, bounding boxes (`box_2d` and `bbox_norm`), and page classification (`text_page`, `pure_graphic`, `blank`).
- `workspace/pages_manifest.json`: Document index, dimensions, block counts, and page list.

Stage 01 reads the source PDF directly and writes no single-page PDFs. Downstream stages use PNG pixels and JSON text/geometry; visual crops retain the rendered resolution rather than native PDF vectors.

---

## Invocation (via Pipeline Orchestrator)

> [!IMPORTANT]
> Always invoke Stage 01 through the main orchestrator (`pipeline.py`) rather than calling `preprocess.py` directly.

### Standard Preprocessing (with automatic scan detection & OCR):
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "path/to/manual.pdf" --workspace "workspace" --from-stage 01 --to-stage 01
```

### Targeted Pages or Page Ranges:
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "path/to/manual.pdf" --workspace "workspace" --page-ranges "1-5, 7, 8, 10-15" --from-stage 01 --to-stage 01
```
