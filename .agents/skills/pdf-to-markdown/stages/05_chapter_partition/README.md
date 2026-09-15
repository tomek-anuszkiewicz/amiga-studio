# Stage 05: Chapter & Section Partition

## Objective
Slices the global `reduced_stream.json` into independent, chapter-level JSON stream files using clean numeric naming (`01_introduction.json`, `02_the_copper.json`).

## Preamble & TOC Invariant
Any segments occurring before the first detected Level 1 chapter heading (such as title page, copyright notices, preface, and table of contents) are partitioned into their own dedicated unit: `00_toc.json` (`Table of Contents`). Zero content is discarded or orphaned, and Chapter 1 remains isolated as `01_chapter_1.json`.

## Inputs
- `workspace/reduced_stream.json`: Normalized node stream.

## Outputs
- `workspace/chapters/{index:02d}_{slug}.json`: Partitioned section stream files.
- `workspace/chapters_manifest.json`: Index mapping section index, slug, title, and target Markdown file.

## Standalone Invocation
```powershell
python stages/05_chapter_partition/partition_chapters.py --workspace "workspace"
```
