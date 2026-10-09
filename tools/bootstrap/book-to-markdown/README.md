# Book-to-Markdown

This directory implements a deterministic book importer, Markdown/TOC generator
and local bitmap review application. It consumes existing prepared data from
[PDF-to-Markdown](../pdf-to-markdown/README.md); review makes no model calls.

PDF preparation ends at Stage 06. The intended handoff uses Stage 02 original
page PNGs and positioned text, Stage 04 filtered page objects and Stage 05
changed page objects. The 05 status entry records its selected page range;
overlay replacements on those selected 04 pages and retain unchanged pages.
Stage 06 extracts graphic assets to `06_extract_crops/assets/<segment_id>.png`.
The generator emits chapter Markdown and copies these prepared assets.
No intermediate page-ordered Markdown export is required.

## Source catalog and variant layout

The [bootstrap instructions](../AGENTS.md#preparing-the-book-source-collection)
describe source acquisition during book authoring.

The current source catalog uses these book directory names:

| PDF to obtain | Directory under `books/` |
| --- | --- |
| Amiga Hardware Reference Manual, second edition (1989) | `amiga-hardware-reference-manual-2nd-edition` |
| Amiga A500/A2000 Technical Reference Manual | `amiga-a500-a2000-technical-reference-manual` |
| M68000 Programmer's Reference Manual | `m68000-programmers-reference-manual` |
| M68000 User's Manual, Revision 8 | `m68000-users-manual-rev8` |

Book IDs use lowercase names separated by hyphens. The hash suffix identifies
the exact PDF variant, and `source.json` stores its full hash. Source PDFs remain
local and are ignored by Git; the catalog metadata is tracked.

The same convention applies to every book, including the composite Test Book:

```text
books/<book-id>/<first-12-characters-of-sha256>/
  source.pdf          # local original PDF
  source.json         # tracked full SHA-256
  pdf-to-markdown/    # local default preparation workspace, created on first run
```

The Test Book variant is `books/test-book-example-4567/1cea0b35ee62/`, with full
SHA-256 `1cea0b35ee622aec0e4c728a170edb6b9324a1baab1f14fe8c67a7f7f2fc45c0`.
Its [page tracker](books/test-book-example-4567/1cea0b35ee62/test_pages_tracker.md)
lives beside `source.pdf` and records the four source variants, their full hashes
and the 160-page assembly order. The prefix names the variant directory;
the full hash identifies the exact source PDF. Future book preparation uses this layout.

## Element vocabulary

The initial [element list](elements.md) records source roles used in book authoring. Notes and legends use generic `note` and `legend` types; authored groups associate them with tables or images, and notes may also stand independently in the text. Image elements retain the existing `graphic` identifier. Grouping is authored separately from PDF table transcription.

Chapter, appendix and section titles use `heading` with a `heading_level` property for hierarchy depth and an optional `number` string for their source numbering.
The dedicated contents, index, table-list and figure-list heading types retain their roles and are treated as level-1 headings.

`code_block` represents source code and similar listings. `preformatted_text` preserves original character layout for ASCII art, dumps and other aligned text, rendered in a fenced `text` block.

## Local bitmap review

Install the Python packages and local KaTeX assets from the repository root:

```powershell
python -m pip install -r tools/bootstrap/book-to-markdown/requirements.txt
npm install --prefix tools/bootstrap/book-to-markdown --ignore-scripts
```

The default renderer uses installed Microsoft Edge (`--channel msedge`). For
bundled Chromium, run `python -m playwright install chromium` and pass
`--channel ""`. The screenshot implementation uses Chromium's DevTools protocol;
Firefox and WebKit are not supported. Rendering and review use local resources.

Use the existing Test Book variant; the import command refuses to overwrite an
authored model or Accepted bundle:

```powershell
$Book = "tools/bootstrap/book-to-markdown/books/test-book-example-4567/1cea0b35ee62"
python tools/bootstrap/book-to-markdown/book_review.py import --book $Book --hide-test-toc
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
python tools/bootstrap/book-to-markdown/book_review.py serve --book $Book
```

Open `http://127.0.0.1:8765`. `serve` also renders when necessary, so the separate
`render` command is optional. Run only one renderer/server for a book at a time.
Stop with Ctrl+C. `--port` changes the local server port; `--width` changes bitmap
width from its 1200-pixel default and invalidates render caches. Panel scaling
uses the existing geometry and does not rerender.

The import reads successful preparation statuses, selects Stage 04 pages using
the Stage 05 recorded range, and uses entire Stage 05 replacements where present.
Stage 03 is never consulted. Thumb indexes are omitted. The explicit
`--hide-test-toc` option hides only segments 001/002 on physical pages 7, 8 and 9;
lists of figures/tables remain. One draft chapter preserves the surviving source
order. Stage 02 PNGs are copied byte-for-byte and Stage 06 supplies graphic assets.

`source-ocr.pdf` is the primary prepared PDF by filename convention; `source.pdf`
and `source.json` retain the original and its hash. No PDF path is stored in the
book model. The review displays the original Stage 02 physical page PNGs under
`pdf/pages/`, including pages without retained elements. Each version owns:

```text
working/ or accepted/
  book.json                 # authored chapter/element model and TOC options
  rendered.json             # last complete model plus bitmap geometry
  <chapter-name>.md         # complete export
  table-of-contents.md
  assets/
  renders/<revision>/       # immutable strips and source-asset snapshot
```

Working also keeps `renders/.cache/`. Old revisions remain local so an existing
viewer can finish loading its Working images. No automatic cache cleanup or
acceptance history is implemented. Outputs are ignored by Git.

### Editing and publication

Edit `working/book.json` externally for text, type, order, visibility, source
regions, chapter organization or TOC options. Preserve logical IDs for ordinary
edits. Use `heading_level` consistently with the leading heading hashes. `toc`
defaults to enabled and depth 2, and supports actual heading depths 1, 2 and 3.
Chapter titles do not add duplicate TOC entries. Links use stable element-ID
anchors; generated TOC content has no invented source regions.

Working headings expose levels 1–6 when clicked. The command checks the current
authored fingerprint and heading identity, then saves the level and leading
hashes together. A stale command reports an error. Accepted remains read-only.
The toolbar also enables/disables the generated TOC and selects depth 1, 2 or 3
through the same revision-aware save and rendering queue.

A separate rendering thread polls only authored JSON and referenced asset bytes.
It finishes its immutable input snapshot before reading the newest pending
state. Every affected chapter is regenerated as complete HTML using
`markdown-it-py` with tables/raw HTML and parsed math nodes rendered by local
KaTeX. It waits for images/fonts/math, measures complete chapter geometry and
captures full-width strips with shared rounded boundaries and local overlay
bounds. Content, properties, referenced asset bytes, settings and local geometry
form cache keys; source bbox changes alone do not invalidate bitmaps.
Unfenced code/preformatted objects become fenced exports without changing their
authored text. Wide preformatted blocks and tables scale to fit the fixed bitmap.
Changing render settings refreshes Accepted from its own authored model, without
accepting Working edits, so both columns use the same presentation settings.

The server publishes only complete snapshots and announces changes using SSE.
Pending edits and parse/render errors retain the previous complete view; the
error is visible. Reconnection reloads the latest complete snapshot. Derived
outputs do not schedule rendering. PNG URLs include the revision so geometry
cannot be mixed with an earlier image under the same address.

The first successful current rendering initializes Accepted with the identical
bundle. Later changes require **Accept Working**. Acceptance is enabled only
when the completed snapshot matches current authored data/assets and the queue
has no pending or failed cycle. It generates full Markdown and TOC, prepares a
complete copy, rechecks input freshness, then replaces Accepted under the server
lock. Failed preparation leaves Accepted untouched; failed final rename restores
its backup. Startup recovers an interrupted rename. Obsolete files disappear
with the old bundle. This action does not commit, publish or upload anything.

Working Markdown exports are generated on import/explicit `render` and first
initialization. Interactive cycles update PNGs/snapshot without rewriting all
exports. Refresh a complete current Working export with:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py export --book $Book
```

### Navigation and limits

All three columns contain bitmap images. Offscreen images load lazily with
reserved heights. Hover/selection matches exact IDs; Copy ID copies that identity.
The chapter selector, shared Accepted/Working collapse state and Previous/Next
Change controls navigate logical content. Hidden/removed/added elements show
explicit side status, without invented counterpart highlights.

Manual scrolling transfers leadership. A reference line at 25% of the viewport
maps relative element progress to counterpart heights. PDF mapping uses ordered
source regions, skips disconnected gaps and can move backwards or across text
columns. Pointer horizontal position disambiguates PDF columns, preferring
visible Working bindings. Follower scroll events do not acquire leadership.
Refresh retains the latest logical position, selection and collapsed chapters.
TOC link hit regions navigate real target headings; scrolling the TOC alone does
not invent a PDF binding.

This first release has heading-level editing only. Type/text/split/merge GUI
operations, adjustable column widths, zoom and acceptance undo are deferred.
Unconverted empty table/cover content remains in the model and PDF column;
the importer does not infer replacement text or invent raster crops. Technical
execution and checks do not certify source transcription or visual quality.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Handling guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).

The instruction combines source-faithful OCR correction with limited Markdown formatting, including inline-code emphasis of technical terms. It preserves table structure, code/ASCII alignment, chapter organization and links. HTML blocks use HTML formatting and escaping rather than Markdown inline syntax. Image/table descriptions remain separately requested tasks.

The prompt is self-contained and preserves chapter Markdown structure and filenames.

## Table and image descriptions

Two initial prompts support separately requested descriptions:

- [Table description](prompts/describe-tables.md) explains table entries, values and relationships, including merged HTML cells.
- [Image description](prompts/describe-images.md) explains the inspected image's components, labels and visual relationships.

The prompts return an English Markdown description for insertion into the source group's `<details>` section while preserving the source material. These are prompt drafts for manual agent use; automatic execution and insertion are not implemented. Input preparation and image inspection guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).
