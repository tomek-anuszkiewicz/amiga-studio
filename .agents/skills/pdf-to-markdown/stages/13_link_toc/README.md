# Stage 13: Link Table of Contents

## Objective
Dedicated cross-document wikilink resolution worker:
1. Scans emitted Markdown files for the block enclosed between `<!-- TOC34534 -->` and `<!-- /TOC34534 -->`.
2. Catalogs all Markdown headings across all `.md` files in the output directory.
3. Performs fuzzy matching of each TOC line against cataloged headers.
4. Converts each TOC item into an Obsidian cross-document wikilink:
   `- [[02_the_copper#Copper Registers|Copper Registers]]`
5. Completely strips the temporary `<!-- TOC34534 -->` and `<!-- /TOC34534 -->` delimiters upon completion.

## Inputs
- `<output_dir>/*.md`: Emitted Markdown documents (from Stage 12).

## Outputs
- `<output_dir>/*.md`: Finalized Markdown documents with active cross-file wikilinks and zero leftover marker tags.

## Standalone Invocation
```powershell
python stages/13_link_toc/link_toc.py --output-dir "output_markdown" --workspace "workspace"
```
