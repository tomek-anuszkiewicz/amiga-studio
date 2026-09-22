# Stage 14: Link Table of Contents

## Objective
Dedicated cross-document wikilink resolution worker:
1. Scans emitted Markdown files for the block enclosed between `<!-- TOC34534 -->` and `<!-- /TOC34534 -->`.
2. Catalogs all Markdown headings across all `.md` files in the input directory.
3. Performs fuzzy matching of each TOC line against cataloged headers.
4. Converts each TOC item into an Obsidian cross-document wikilink:
   `- [[02_the_copper#Copper Registers|Copper Registers]]`
5. Completely strips the temporary `<!-- TOC34534 -->` and `<!-- /TOC34534 -->` delimiters upon completion.

## Inputs
- `<workspace>/13_refine_first_chapter_name/*.md`: Markdown documents with refined canonical opening file.

## Outputs
- `<output_dir>/*.md`: Finalized Markdown documents with active cross-file wikilinks and zero leftover marker tags.
- `<output_dir>/assets/`: Synchronized visual assets.

## Standalone Invocation
```powershell
python stages/14_link_toc/link_toc.py --output-dir "output_markdown" --workspace "workspace" --config "config.yaml"
```
