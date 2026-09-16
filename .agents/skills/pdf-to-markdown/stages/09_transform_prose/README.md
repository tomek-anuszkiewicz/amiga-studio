# Stage 09: Transform Prose & Tag TOC

## Objective
Formats narrative body text, programming code listings, section headings, and table of contents entries:
1. Backticks hardware registers (`DMACON`, `INTENA`) and hexadecimal addresses (`$DFF000`).
2. Encloses programming code listings in explicitly tagged language blocks (`m68k`, `c`, `text`).
3. Enforces the anti-leak rule: regular English prose sentences are never wrapped in code blocks.
4. Wraps Table of Contents blocks in unique delimiters:
   ```markdown
   <!-- TOC34534 -->
   - Section Title
   <!-- /TOC34534 -->
   ```
5. Preserves explicit node types (`prose`, `code_block`, `toc`) in the JSON stream.

## Inputs
- `workspace/08_transform_graphics/{index:02d}_{slug}.json`: Chapter streams from Stage 08.
- `stages/09_transform_prose/prompt.md`: Formatting instructions and code/register rules.

## Outputs
- `workspace/09_transform_prose/{index:02d}_{slug}.json`: Formatted chapter streams with `node.rendered_markdown`.

## Standalone Invocation
```powershell
python stages/09_transform_prose/format_prose.py --workspace "workspace"
```
