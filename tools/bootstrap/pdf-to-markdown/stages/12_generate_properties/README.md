# Stage 12: Generate Obsidian Properties

## Objective
Enriches raw Markdown chapter files emitted from Stage 11 with publication-grade Obsidian YAML frontmatter properties using Gemini LLM:
1. Discovers emitted chapter files in `<workspace>/11_emit_markdown`.
2. Analyzes the opening chapter / front matter to infer the canonical **`book`** title (e.g. `"M68000 Family Programmer's Reference Manual"`, `"Amiga Hardware Reference Manual"`).
3. Evaluates each chapter's content and structure to generate:
   - `title`: Publication-grade chapter title (e.g. `"Section 3: Instruction Set Summary"`).
   - `book`: The canonical book title inferred from the first chapter.
   - `chapter`: Normalized chapter designator (e.g. `"Section 3"`, `"Chapter 1"`, `"Table of Contents"`).
   - `tags`: 4–8 lowercase kebab-case domain tags.
4. Prepends the canonical Line 1 YAML properties block bounded by `---`.
5. Synchronizes visual assets to `<output_dir>/assets/`.

## Inputs
- `<workspace>/11_emit_markdown/*.md`: Clean emitted Markdown documents.
- `<workspace>/11_emit_markdown/assets/`: Visual assets.

## Outputs
- `<output_dir>/*.md`: Markdown documents with Line 1 Obsidian YAML frontmatter.
- `<output_dir>/assets/`: Synchronized visual assets.

## Standalone Invocation
```powershell
python stages/12_generate_properties/generate_properties.py --workspace "workspace" --config "config.yaml"
```
