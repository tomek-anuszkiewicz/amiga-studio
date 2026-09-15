# Stage 05: Chapter & Section Partition

## Objective
Slices the global `reduced_stream.json` into independent, chapter-level JSON stream files using clean numeric naming (`01_introduction.json`, `02_the_copper.json`).

## Preamble Invariant
Any segments occurring before the first detected Level 1 chapter heading (such as title page, copyright notices, preface, or front matter) are automatically prepended to the first unit stream. Zero content is discarded or orphaned.

## Inputs
- `workspace/reduced_stream.json`: Normalized node stream.

## Outputs
- `workspace/chapters/{index:02d}_{slug}.json`: Partitioned section stream files.
- `workspace/chapters_manifest.json`: Index mapping section index, slug, title, and target Markdown file.

## Standalone Invocation
```powershell
python stages/05_chapter_partition/partition_chapters.py --workspace "workspace"
```
