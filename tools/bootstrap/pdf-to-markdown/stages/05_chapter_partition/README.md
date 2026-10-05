# Stage 05: Chapter & Section Partition

## Objective
Slices the global `reduced_stream.json` into independent, chapter-level JSON stream files using clean numeric naming (`01_chapter_1_introduction.json`, `02_chapter_2_coprocessor_hardware.json`):
1. **Front Matter / Preface**: All preliminary title pages, copyright notices, and preface segments prior to the Table of Contents are partitioned into `00_preface.json`.
2. **Table of Contents & Lists**: All TOC listings, List of Figures, and List of Tables segments up to Chapter 1 are partitioned into `00_toc.json`.
3. **Semantic Chapter Boundaries**: Chapter boundaries are driven directly by `type == "chapter"` emitted by the vision model in Stage 02. If a chapter opening segment is immediately followed by a title/subtitle on the same page (e.g. `Chapter 1` followed by `INTRODUCTION`), both blocks are fused into a unified chapter header (`Chapter 1: INTRODUCTION`). Subsections within the chapter stay inside their respective chapter stream.

## Inputs
- `workspace/04_stream_reduction/reduced_stream.json`: Normalized node stream from Stage 04.

## Outputs
- `workspace/05_chapter_partition/{index:02d}_{slug}.json`: Partitioned section stream files.
- `workspace/chapters_manifest.json`: Document section manifest mapping section index, slug, title, and target Markdown file.

## Standalone Invocation
```powershell
python stages/05_chapter_partition/partition_chapters.py --workspace "workspace"
```
