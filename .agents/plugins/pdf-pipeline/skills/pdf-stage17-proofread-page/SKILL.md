---
name: pdf-stage17-proofread-page
description: >-
  Stage 17: Multimodal semantic proofreading and retrocomputing syntax normalization of per-page Markdown documents using preceding-page (N-1) sliding window context.
---

# Stage 17: Proofread Page (Semantic Normalization & Sliding Window Context)

This skill governs semantic proofreading and retrocomputing syntax normalization of per-page Markdown documents.

It takes embedded per-page Markdown documents (`build/01_page_layout/page_XXXX-embed.md` from Stage 16), provides the preceding page (`build/01_page_layout/page_{XXXX-1}-embed.md`) as **read-only sliding window context** to prevent boundary blindness (unclosed code fences, lowercase continuation phrases, broken words), inspects ambiguous OCR readings against `page_XXXX.png` via `view_file`, and generates clean, publication-grade page Markdown in:
`build/01_page_layout/page_XXXX-proofread.md`

## Purpose & Scope
- **Sliding Window Context (Preceding Page N-1 from `-embed.md`):**
  - The preceding page (`N-1`) is provided exclusively as **read-only reference context**.
  - **Sentence Continuity:** If Page N starts with a lowercase word (e.g., `rupt handler is called...`), the model verifies against Page N-1 (`The inter-`) and preserves lowercase continuation without false capitalization.
  - **Unclosed Code Fences:** If Page N begins with assembly instructions continuing from Page N-1, the model ensures code fences (````m68k````) are properly structured.
  - **Strict Boundary Rule:** The model NEVER emits or duplicates content from Page N-1 into the output for Page N.
- **Retrocomputing Syntax & Hardware Registers:**
  - **Amiga Custom Chip Registers:** All custom register names (e.g. `DMACON`, `DMACONR`, `INTENA`, `INTENAR`, `INTREQ`, `INTREQR`, `ADKCON`, `ADKCONR`, `COP1LCH`, `COP1LCL`, `COP2LCH`, `COP2LCL`, `COPJMP1`, `COPJMP2`, `COPCON`, `BPLCON0`–`BPLCON4`, `BPL1PTH`–`BPL6PTL`, `BLTCON0`, `BLTCON1`, `BLTAFWM`, `BLTALWM`, `BLTCPTH`–`BLTDPTH`, `COLOR00`–`COLOR31`, `AUD0LCH`–`AUD3DAT`, `POTGO`, `SERDAT`, `SERDATR`, `CIAAPRA`, `CIABPRB`) must be wrapped in inline backticks (e.g. `` `DMACON` ``).
  - **Motorola 68000 Registers & Signals:** CPU registers (`D0`–`D7`, `A0`–`A7`, `SP`, `USP`, `SSP`, `PC`, `SR`, `CCR`) wrapped in inline backticks (e.g. `` `D0` ``). Active-low bus and control signals featuring printed overbars must be formatted in native LaTeX overbars: `$\overline{\text{AS}}$`, `$\text{R}/\overline{\text{W}}$`, `$\overline{\text{UDS}}$`, `$\overline{\text{LDS}}$`, `$\overline{\text{DTACK}}$`, `$\overline{\text{BERR}}$`, `$\overline{\text{RESET}}$`, `$\overline{\text{HALT}}$`, `$\overline{\text{BR}}$`, `$\overline{\text{BG}}$`, `$\overline{\text{BGACK}}$`, `$\overline{\text{IPL0}}$`–`$\overline{\text{IPL2}}$`, `$\overline{\text{VPA}}$`, `$\overline{\text{VMA}}$`. (Active-high signals like `` `E` `` remain in backticks).
  - **Hexadecimal & Binary Notation:** Memory addresses, offsets, and bit patterns (e.g. `$DFF000`, `$DFF1FE`, `$000000`, `0x...`, `%01001100`) must be enclosed in inline backticks (e.g. `` `$DFF000` ``).
- **Deduplication Around Tables & Visual Assets:** Removes accidental duplicate lines, phantom table headers, or echoed raw OCR rows dumped immediately before or after embedded tables or visual asset breakdowns (`<details>`, `![...](...)`). Genuine prose context is preserved.
- **Processor Model Numbers & Chip Part Designations:** Motorola processor, coprocessor, and peripheral model numbers end in numeric digits (zeros), NEVER the capital letter 'O' (e.g. `MC68000` NOT `MC68OOO`, `MC68HC000` NOT `MC68HCOOO`, `MC68EC000` NOT `MC68ECOOO`, `MC68008` NOT `MC68OO8`, `MC68010` NOT `MC68O1O`, `MC68020` NOT `MC68O2O`, `68000` NOT `68OOO`).
- **OCR Error Correction & Scan Artifact Healing:**
  - **Zero vs. Capital O Confusion:** Corrects common OCR substitutions in hex and registers (e.g. `$DFFO00` -> `$DFF000`, `DO` -> `D0` when functioning as a data register, `BPLCONO` -> `BPLCON0`, `CIABPRA` vs `CIAAPRA`).
  - **One vs. Lowercase L / Capital I Confusion:** (e.g. `l000` -> `1000`, `I0` -> `10`, `$00000I` -> `$000001`).
  - **Split Words & Spacing:** Heals broken words and split numbers caused by OCR spacing/kerning (e.g. `in terrupt` -> `interrupt`, `co processor` -> `coprocessor`, `-10 0` -> `-100`).
  - **Misread Punctuation & Units:** (e.g. `samples!line` -> `samples/line`, `frames!second` -> `frames/second`).
- **Code Blocks, Monospaced Text & Anti-Leak Invariant:**
  - Detects Motorola 68000 assembly (````m68k```` / ````asm````), C source code (````c````), and preformatted ASCII/hex dumps (````text````).
  - Enforces clean columnar indentation (Labels at col 0, mnemonics at col 8, operands at col 16, comments at col 40).
  - **Anti-Leak Invariant:** Regular English prose sentences with punctuation in body text must NEVER be enclosed in code blocks. However, ```` ```text ... ``` ```` blocks inside `<details>` (diagram breakdowns and text tables) must be proofread for OCR and spelling errors while strictly preserving their clean plain-text formatting (no inline backticks, bolding, or Markdown headings).
- **Formatting & Structural Governance:**
  - **Mathematical Expressions:** Standard KaTeX syntax (`$inline$` and `$$display$$`).
  - **Run-in Paragraph Headings:** Format ONLY the title prefix in bold up to the period/colon (e.g. `**1.2.3.4 ACCRUED EXCEPTION BYTE.** The prose...`). Never bold the whole paragraph or convert to `#` headings.
  - **Callout Governance (NOTE, WARNING, CAUTION):** When explicitly designated in the source publication (e.g. centered or framed NOTE/WARNING/CAUTION blocks), format them in standard Markdown callout alert syntax (`> [!NOTE]`, `> [!WARNING]`, etc.). Do NOT invent callouts spontaneously for ordinary prose paragraphs.
  - **Table of Contents, Lists & Subject Indexes:** Format TOC, lists, and index entries as clean, standard nested Markdown lists with accurate hierarchy and indentation. Strip dot leaders, page markers, and physical printed page/chapter numbers (e.g. `2-1`, `3-3, 3-4`, `5-15`) from TOC, lists, and subject indexes.

## Input & Output
- **Input (Read-Only):**
  - Target Page Markdown: `build/01_page_layout/page_XXXX-embed.md` (Stage 16 output)
  - Preceding Page Reference: `build/01_page_layout/page_{XXXX-1}-embed.md` (Stage 16 output, read-only sliding window context; omitted for page 1)
  - Visual Layout Render: `build/01_page_layout/page_XXXX.png` (inspected via `view_file` to resolve OCR ambiguities)
- **Output:**
  - `build/01_page_layout/page_XXXX-proofread.md` (Refined, normalized per-page Markdown document)

---

## CLI Verification & Helper Tool

Inspect proofreading status or deterministically verify finalized pages on disk:
```bash
# Check overall proofreading progress
python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "<manual_dir>" --status

# List pending pages awaiting proofreading
python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "<manual_dir>" --pending

# Deterministically verify that specific pages exist and are non-empty (used by Stage Lead)
python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "<manual_dir>" --verify-pages 1-30
```

---

## Execution Protocol

All detailed proofreading instructions and domain syntax rubrics are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned page in the chunk payload:
1. **Read Target Page:** Read `build/01_page_layout/page_XXXX-embed.md`.
2. **Read Sliding Window Context:** Read `build/01_page_layout/page_{XXXX-1}-embed.md` strictly as read-only sliding window context (omit for page 1).
3. **Inspect Visual Render (Optional):** Call `view_file` on `build/01_page_layout/page_XXXX.png` to resolve OCR ambiguities or distorted symbols.
4. **Write Normalized Output:** Call `write_to_file` to write the refined, normalized Markdown directly to `build/01_page_layout/page_XXXX-proofread.md`.
5. **Completion Contract:** When all assigned pages in the chunk are processed, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 17,
     "chunk_id": "<CHUNK_ID>",
     "pages_proofread": ["page_XXXX", ...]
   }
   ```
   Do not echo page content in the return message.
