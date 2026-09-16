# Stage 12: Refine First Chapter Name

## Objective
Contextual naming refinement worker for the document's opening section:
1. Inspects the first emitted Markdown file in `<output_dir>` (e.g. `00_preliminary.md` or `01_preface.md`).
2. Evaluates the actual text content and layout to establish its canonical title and slug (primarily detecting whether it is the Table of Contents, Preface, or Front Matter).
3. Renames the file in `<output_dir>` (e.g. to `00_front_matter.md` or `00_table_of_contents.md`).
4. Updates the title property in the Line 1 YAML frontmatter.
5. Synchronizes any cross-file wikilinks across the output directory to point to the new filename.

## Inputs
- `<output_dir>/*.md`: Emitted Markdown documents from Stage 11.
- `stages/12_refine_first_chapter_name/prompt.md`: Evaluation prompt.

## Outputs
- `<output_dir>/00_{slug}.md`: Renamed canonical opening document with updated YAML frontmatter.

## Standalone Invocation
```powershell
python stages/12_refine_first_chapter_name/refine_name.py --output-dir "output_markdown" --workspace "workspace"
```
