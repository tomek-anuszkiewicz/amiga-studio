# Stage 05: Chapter & Section Partition

## Objective
Slices the global `reduced_stream.json` into independent, chapter-level JSON stream files using clean numeric naming (`01_chapter_1_introduction.json`, `02_chapter_2_coprocessor_hardware.json`):
1. **Front Matter / Preface**: All preliminary title pages, copyright notices, and preface segments prior to the Table of Contents are partitioned into `00_preface.json`.
2. **Table of Contents & Lists**: All TOC listings, List of Figures, and List of Tables segments up to Chapter 1 are partitioned into `00_toc.json`.
3. **Chapter Start Welding**: When a major chapter heading begins (e.g. `Chapter 1` followed immediately by `INTRODUCTION`), both blocks are fused into a single unified chapter boundary (`Chapter 1: INTRODUCTION`), ensuring internal subsections stay within the chapter.
4. **Typographical Normalization**: Cleans OCR/kerning errors (such as `HARDW ARE` -> `HARDWARE`) in chapter titles and slugs.

## Inputs
- `workspace/04_reduced_stream/reduced_stream.json`: Normalized node stream.

## Outputs
- `workspace/05_chapters_raw/{index:02d}_{slug}.json`: Partitioned section stream files.
- `workspace/chapters_manifest.json`: Index mapping section index, slug, title, and target Markdown file.

## Standalone Invocation
```powershell
python stages/05_chapter_partition/partition_chapters.py --workspace "workspace"
```
