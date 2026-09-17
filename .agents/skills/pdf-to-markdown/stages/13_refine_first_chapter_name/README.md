# Stage 13: Refine First Chapter Name

## Objective
Contextual naming refinement worker for the document's opening section:
1. Inspects the first emitted Markdown file in `<input_dir>` (e.g. `00_preliminary.md` or `01_preface.md`).
2. Evaluates the actual text content and layout to establish its canonical title and slug (primarily detecting whether it is the Table of Contents, Preface, or Front Matter).
3. Renames the file in `<output_dir>` (e.g. to `00 - Table of Contents and Front Matter.md`).
4. Updates the `title` property in the Line 1 YAML frontmatter.
5. Synchronizes any cross-file wikilinks across the output directory to point to the new filename.

## Inputs
- `<workspace>/12_generate_properties/*.md`: Markdown documents with Line 1 YAML properties.
- `stages/13_refine_first_chapter_name/prompt.md`: Evaluation prompt.

## Outputs
- `<output_dir>/00_{slug}.md`: Renamed canonical opening document with updated YAML frontmatter.
- `<output_dir>/assets/`: Synchronized visual assets.

## Standalone Invocation
```powershell
python stages/13_refine_first_chapter_name/refine_name.py --workspace "workspace" --config "config.yaml"
```
