# Book-to-Markdown

This directory implements a deterministic book importer, Markdown/TOC generator
and local HTML review application. It consumes existing prepared data from
[PDF-to-Markdown](../pdf-to-markdown/README.md); review makes no model calls.

PDF preparation ends at Stage 06. The intended handoff uses Stage 02 original
page PNGs and positioned text, Stage 04 filtered page objects and Stage 05
changed page objects. The 05 status entry records its last invocation's page range;
import all retained 04 pages, overlay available 05 replacements and retain unchanged
pages. A partial PDF refresh preserves the rest of the prepared page collection.
Stage 06 extracts graphic assets to `06_extract_crops/assets/<segment_id>.png`.
The generator emits chapter Markdown and copies these prepared assets.
No intermediate page-ordered Markdown export is required.

The [published recipe design](recipe-reconstruction.md) records the intended
bootstrap path from an exact source PDF to reconstructed Markdown. New imports
store PDF text references in `book.json`; loading,
rendering and acceptance resolve them against Stage 02 positioned text. Automatic
source acquisition and reconstruction directly from a PDF remain pending.

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

## Local HTML review

Install the Python packages and local KaTeX assets from the repository root:

```powershell
python -m pip install -r tools/bootstrap/book-to-markdown/requirements.txt
npm install --prefix tools/bootstrap/book-to-markdown --ignore-scripts
```

The default renderer uses installed Microsoft Edge (`--channel msedge`). For
bundled Chromium, run `python -m playwright install chromium` and pass
`--channel ""`. This browser prepares and validates local KaTeX output; it no
longer captures Markdown screenshots. The review browser displays the resulting
HTML directly. Preparation and review use local resources.

Use the existing Test Book variant; the import command refuses to overwrite an
authored model or Accepted bundle:

```powershell
$Book = "tools/bootstrap/book-to-markdown/books/test-book-example-4567/1cea0b35ee62"
python tools/bootstrap/book-to-markdown/book_review.py import --book $Book
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
python tools/bootstrap/book-to-markdown/book_review.py serve --book $Book
```

Open `http://127.0.0.1:8765`. `serve` also renders when necessary, so the separate
`render` command is optional. Run only one renderer/server for a book at a time.
Stop with Ctrl+C. `--port` changes the local server port; `--width` sets the
preparation browser viewport (default 1200 pixels), not the displayed document
width. Accepted and Working text wraps to the actual panel width. Window resizing
does not regenerate HTML or source images.

Drag the divider between PDF and Accepted to set a fixed PDF column width.
The initial width is 600 pixels, with a 240-pixel minimum; focusing the divider
also permits adjustment with the left/right arrow keys. The chosen width is
stored locally in the browser and restored on reload for the same server address
and port. Accepted and Working share the remaining space, with a 240-pixel minimum
each; narrower windows scroll the complete three-column layout horizontally.
These layout changes do not modify the book or request server rendering.

The import reads successful preparation statuses, reads all retained Stage 04 pages,
and uses entire Stage 05 replacements where present.
Stage 03 is never consulted. Thumb indexes are omitted. All imported elements
start visible, including source contents and lists of figures/tables.
One draft chapter preserves the surviving source
order. Stage 02 PNGs are copied byte-for-byte and Stage 06 supplies graphic assets.

`source-ocr.pdf` is the primary prepared PDF by filename convention; `source.pdf`
and `source.json` retain the original and its hash. No PDF path is stored in the
book model. The review displays the original Stage 02 physical page PNGs under
`pdf/pages/`, including pages without retained elements. Each version owns:

```text
working/ or accepted/
  book.json                 # authored chapter/element model and TOC options
  rendered.json             # last complete model plus chapter HTML manifest
  <chapter-name>.md         # complete export
  table-of-contents.md
  assets/
  renders/<revision>/       # immutable chapter HTML, document.css and asset snapshot
```

Unchanged chapter HTML is reused from the previous completed revision. Old
Working revisions remain local so an existing viewer can finish loading them.
Old bitmap revisions/cache may remain from earlier runs but are no longer used.
No automatic cleanup or acceptance history is implemented. Outputs are ignored
by Git. Existing imported books are refreshed without rerunning PDF preparation
or changing their authored models.

### Editing and publication

Every `md_text` is a PDF text template using the syntax below.
Chapter `title` and `markdown_file` use the same syntax: they can contain plain
text, PDF references, or both. Resolution happens before chapter labels, export
filenames and TOC links are generated. The resolved filename must be a unique,
bundle-local `.md` filename; references do not bypass filename checks. The authored
JSON retains templates, while snapshots carry derived `resolved_title` and
`resolved_markdown_file` values for presentation and export. The initial imported
chapter title references its first heading; its output filename is an identifier.
New imports keep conversion Markdown/HTML structure as literal recipe syntax,
replacing matching source text with `{{text page=3 block=1 chars=0:6}}` references.
Stage 05 replacements supply their whole page; unchanged pages use Stage 04.
Positioned Stage 02 blocks intersecting the element region supply candidate text.
Exact matching can reorder source selections, such as a wrapped label beside an
assembly listing. Fences, their language tags and HTML tags/attributes remain
literal structure. Unmatched transcription/corrections remain explicit literals;
matching equal characters does not certify semantic attribution of repeated text.
Loading a new recipe resolves PDF ranges without consulting converted page Markdown.
Existing authored books using `{{markdown stage=05 page=3 segment=page_0003_seg_001}}`
remain readable; they are not silently rewritten. Regeneration is a separate request.
Chapter titles retain the imported heading's recipe.

`{{text page=3 block=1 chars=0:6}}` selects Unicode characters from a Stage 02
block, with a zero-based start and exclusive end. Ordered references can combine
blocks; intervening literal text retains Markdown formatting and corrections.
Escape literal backslashes as `\\`, opening braces as `\{` and closing braces as
`\}` in the decoded template (JSON requires its own additional escaping).
Resolution is single-pass: braces or template-looking text fetched from the PDF
remain ordinary content and are never executed. Rendered Markdown receives the
original characters, without extra escapes that would break math or code.
For preview and export, paragraph soft breaks become explicit Markdown hard
breaks so source lines remain visible in ordinary Markdown viewers. This
presentation formatting does not change templates or resolved source text;
headings, lists, tables, code, math and raw HTML retain their Markdown structure.

Source identity is recorded once in `source.json`; the pinned preparation process
recreates the PDF text layer. Loading reads only Stage 02 pages named by the
references and checks block existence and character bounds. It does not hash
page text or duplicate source metadata in the book. Markdown references load
the named Stage 04/05 page and segment. PDF geometry locates an element in the
source pane; import does not concatenate intersecting PDF blocks to replace its
converted text. Both reference types resolve in one pass, including source text
that contains braces. A missing referenced segment is an error. PDF range
templates remain supported for authored selections; loading them does not
require character geometry. Authored literals can still be added manually to a
template, but import does not insert transcription literals.
`resolved_md_text` is derived preview/snapshot data,
never persisted in authored `book.json`. All text uses the same template syntax;
there is no separate plain-Markdown compatibility format.

Edit `working/book.json` externally for text, type, order, visibility, source
regions, chapter organization or TOC options. Preserve logical IDs for ordinary
edits. Hide front matter, source contents or other unwanted elements manually
with `visible: false`, and decide where the authored book begins and ends.
The generated TOC is built automatically from visible headings; it does not
require hiding the source contents. Use `heading_level` consistently with the leading heading hashes. `toc`
is mandatory, defaults to depth 2, and supports actual heading depths 1, 2 and 3.
The retired `toc.enabled` property in older books is ignored on load.
Chapter titles do not add duplicate TOC entries. Links use stable element-ID
anchors; generated TOC content has no invented source regions.

Click an element to select it, then use **Element actions**, right-click, or
Enter/Shift+F10 on a focused element to open its menu. The same actions are
available from PDF, Accepted and Working; edits always target the corresponding
Working element. Elements expose **Hide Element** or **Unhide Element**; headings expose
an expandable **Heading** menu with **H1** through **H6**, applied immediately.
Accepted remains unchanged until **Accept Working**. If an Accepted element has
no Working counterpart, its element-editing actions are disabled.
Click the selected element again, press **Escape**, or click an empty area to
clear the selection and its yellow frame in all three panels. Document links,
controls and text selection retain their existing interactions.

The top of the element menu shows a disabled **Type: ...** label for quick
inspection. Equal types appear once; different types appear as
**Type: Accepted / Working**. If the element exists in only one version, its
type appears once. The label does not open an editor.

**Change type** offers `heading`, `callout`, `callout_text`, `prose`, `code_block`,
`preformatted_text`, `caption`, `footnote`, `note` and `legend` from the
[book element vocabulary](elements.md). Cover, header/footer, contents, thumb
index, index and list-of-tables/figures roles remain valid existing source types
but are not offered as conversion targets.
It operates on Working from any column and preserves IDs, visibility, order and
PDF source regions/references. Tables, graphics, image-backed elements and empty
elements cannot change type; table and graphic are also excluded as targets.
Existing source and legacy text roles can be changed to the offered roles.
A new heading defaults to H1; changing an existing heading role to `heading`
preserves its level. Leaving a heading role removes its `#`
prefix and heading level. Other text markup is retained; this action changes the
role rather than rewriting content. `/api/type` checks the authored revision and
enforces the same restrictions as the menu.

Text elements expose **Edit Markdown**, **Bold**, **Split** and **Merge with next**. The Markdown
dialog shows resolved source text with its Markdown/HTML syntax, rather than PDF
template references. Menu **Bold** immediately wraps the selected element's text
in `**...**`, preserving heading prefixes and paragraph boundaries; menu
**Remove Bold** removes strong markup immediately. Neither opens an editor.
In the Markdown dialog, **Add Bold** and **Remove Bold** operate on the selected
text, or the whole section when nothing is selected. The same controls are
available during raw Markdown editing. **Save** writes Working; **Cancel** leaves
it unchanged. Edited heading hashes update the heading role and level; removing
the heading prefix converts that element to prose.

**Split** opens the Markdown source. Place the cursor at the boundary or select
the opening text, then choose **Split at cursor / after selection**. The dialog
shows two editable sections; **OK** saves them as consecutive elements in the
same chapter. Both must contain text. The first keeps its ID and the second gets
a new ID; both retain the original PDF regions, without inferring separate crop
geometry. Splitting is a structural edit; element **Discard** restores the first
element's Accepted properties but does not remove the new element. Use
**Discard Changes** to undo the complete split.

**Merge with next** opens both elements' resolved Markdown in one editable field,
separated by a newline. **OK** replaces them with one element at the first
element's position and ID. All source pages and distinct PDF bounding boxes from
both elements are retained, including merges across PDF pages. The next element
means the immediate successor in the authored book order, including hidden
elements and chapter boundaries; the result belongs to the first element's
chapter. The action is disabled at the end of the book or when either element
has no editable text. **Cancel** preserves both elements. Like Split, this is a
structural edit; use **Discard Changes** to restore the complete Accepted bundle.

**Fix all callouts** scans the entire Working book in authored order, including
hidden elements. Each adjacent `callout` + `callout_text` pair becomes one
Markdown callout (`> [!NOTE] NOTE`, for example), preserving the original label
as its title, body formatting and PDF references/regions. The label keeps its ID,
chapter and visibility. Already formatted callouts are left alone. Orphan labels
or bodies, invalid labels, empty bodies and pairs with different visibility are
left unchanged and reported by ID in the completion dialog. The operation checks
the authored revision and saves valid pairs together. **Discard Changes** restores
the full Accepted bundle. This is an explicit editing action; import is unchanged.
The review renderer displays callout markers as titled advisory blocks.

**Merge tables** immediately joins the selected table with the next authored
element, which must also be a Markdown table. Column counts, names/order and
alignment must match; header emphasis and whitespace differences are ignored.
The first header is retained and the second table's data rows are appended.
The first element keeps its ID, chapter and visibility; PDF regions from both
elements are retained. HTML, malformed tables, surrounding text and incompatible
columns report an error without saving. Hidden elements are included in authored
order; move intervening captions/notes first. Use **Discard Changes** to restore
the complete Accepted bundle.

**Move up** and **Move down** immediately move the selected Working element one
position in authored book order, including hidden elements. At a chapter boundary,
the element moves into the neighboring chapter, before or after its neighboring
element. Text, visibility, IDs and PDF source regions remain unchanged. The
corresponding action is disabled at the beginning or end of the book. Use
**Discard Changes** to restore the Accepted order.

Markdown, split, merge and move commands (`/api/markdown`, `/api/split`, `/api/merge`, `/api/merge-tables`, `/api/move`) check the authored
revision before saving. Unchanged text retains PDF character-range references,
including ranges cut at a split; inserted or replaced text becomes escaped
literal recipe content. Edited legacy Stage 04/05 references are first migrated
to PDF-range recipes. New asset links must resolve before the edit is saved.
Failed saves keep the dialog's draft for correction. Working is rendered and
exported through the existing queue; **Accept Working** copies the completed
bundle to Accepted.
**Hide Page** is available from element menus in all three panels, including
Accepted, and from the PDF page background. It hides every Working element
mapped to the contextual PDF page in one revision-aware `/api/hide-page` command;
Accepted stays unchanged. On PDF the clicked page determines the context. In
Markdown the selected element's source regions and relative click position
identify the page, using the same proportional mapping as scroll synchronization.
An element spanning multiple pages is hidden as a whole. The action is disabled
without a PDF source page or when that page has no visible Working elements.

**Show hidden elements** is a client-only view option for both versions and PDF
source overlays: hidden content has a violet tint and dotted overlay. It does
not change `visible`, Markdown exports, the generated TOC or
acceptance. Original PDF page pixels remain intact. A PDF page is hidden when no visible Working element maps to it,
including pages without any mapped elements. **Show hidden elements** reveals
all pages again for source inspection. Page visibility follows Working even before acceptance, without a
separate saved page property. Hidden content is prepared
in review HTML so the checkbox requires no server request.

Visibility commands use `/api/visibility` with an element ID, boolean `visible`
and expected authored revision. Like TOC (`/api/toc`) and heading edits
(`/api/heading`), they save Working, queue rendering and notify clients through
SSE; the viewer replaces complete revisions after fetching status and HTML.
Stale commands report an error. The heading command saves the level and leading
hashes together. The toolbar selects generated TOC depth 1, 2 or 3.
**Discard** in a Working element's menu (also accessible through PDF) restores
that element's text and properties from Accepted through `/api/discard-element`.
It preserves its current position and all other Working edits. It is enabled
only for changed properties on an element present in both versions; the command
checks the Working and Accepted revisions. It does not restore asset file bytes
or chapter organization. **Discard Changes** still restores the complete bundle.
`/api/accept` copies the newest complete Working bundle to Accepted.

A separate rendering thread polls only authored JSON and referenced asset bytes.
The server reuses loaded inputs while the file signatures of the authored model,
referenced source JSON and assets remain unchanged. External edits invalidate
that cache; the PDF block lookup is shared across templates within each load.
Client refresh replaces only versions whose rendered revision changed. Accepted
stays mounted during Working edits; the PDF panel is rebuilt only when page
metadata, element IDs, visibility or source regions change. Text-only edits
refresh the existing overlays' model references without replacing page images.
It finishes its immutable input snapshot before reading the newest pending
state. Every affected chapter is regenerated as complete HTML using
`markdown-it-py` with tables/raw HTML and parsed math nodes rendered by local
KaTeX. It waits for local images/fonts/math and saves complete, inert HTML
fragments. Content, properties, referenced asset bytes and preparation settings
form chapter cache keys; source bbox changes alone do not regenerate HTML.
Unfenced code/preformatted objects become fenced exports without changing their
authored text. The viewer isolates each chapter in Shadow DOM, loads its immutable
stylesheet and assets, and prepares both versions before replacing the displayed
views. Text is selectable/copyable; native links and details remain interactive.
Text wraps and images fit the current panel width. Wide tables retain their text
size and overflow into one horizontal scrollbar at the bottom of each Accepted
or Working panel, shared across its chapters. Preformatted blocks scroll
horizontally to preserve code/ASCII whitespace. Element bounds are read from the
current DOM, not persisted as PNG coordinates. Resizing updates browser layout
and logical scroll synchronization without measuring or scaling every table.
Changing render settings refreshes Accepted from its own authored model, without
accepting Working edits, so both columns use the same presentation settings.

The server publishes only complete snapshots and announces changes using SSE.
Pending edits and parse/render errors retain the previous complete view; the
error is visible. Reconnection reloads the latest complete snapshot. Derived
outputs do not schedule rendering. HTML, CSS and asset URLs include the revision
so a completed view never reads graphics from a later authored edit. Failure to
load a complete HTML view retains the previous displayed version and disables
acceptance in that viewer.

The first successful current rendering initializes Accepted with the identical
bundle. Later changes require **Accept Working**. Acceptance is enabled only
when the completed snapshot matches current authored data/assets and the queue
has no pending or failed cycle. It generates full Markdown and TOC, prepares a
complete copy, rechecks input freshness, then replaces Accepted under the server
lock. Failed preparation leaves Accepted untouched; failed final rename restores
its backup. Startup recovers an interrupted rename. Obsolete files disappear
with the old bundle. This action does not commit, publish or upload anything.

**Discard Changes** restores the entire Working bundle from the current Accepted
version through `/api/discard`: authored data, referenced assets, chapter HTML,
Markdown and TOC exports. It is available for a complete current Working revision
that differs from Accepted. The command checks both revisions and prepares the
replacement before swapping directories, with rollback and startup recovery.
Previous immutable Working render directories remain available to existing
clients. Accepted is unchanged; the viewer refreshes through the same SSE/status
flow, and both columns show the accepted content. This discards all unaccepted
edits rather than undoing just the last action.

Working Markdown exports are generated on import/explicit `render` and first
initialization. Interactive cycles update HTML/snapshot without rewriting all
exports. Refresh a complete current Working export with:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py export --book $Book
```

### Navigation and limits

PDF contains original physical page bitmaps, loaded lazily with reserved heights.
Accepted and Working contain direct HTML with stable element-ID containers.
Changed elements have a 5-pixel orange vertical marker in the left margin of
both columns, spanning the element's height without shifting the content.
Hover/selection matches exact IDs; Copy ID copies that identity. Selecting text
does not open the heading menu. Document styling is isolated from the toolbar.
Hovering or focusing an element shows no tooltip or property popup.
The toolbar counts changed elements, using `1 change` for one;
pending rendering and errors appear there instead of the count.
The chapter selector, shared Accepted/Working collapse state and Previous/Next
Change controls navigate logical content. Change navigation stops at the first
and last change instead of wrapping; the corresponding buttons are disabled,
and both are disabled when there are no changes. Navigation scrolls all panels:
when the changed element is hidden or absent in a panel, that panel uses its
following visible common element, or a preceding one if none follows. The selected
change remains selected and the hidden-content checkbox is unchanged. Hidden/removed/added elements show
explicit side status, without invented counterpart highlights.

Manual scrolling transfers leadership. A reference line at 25% of the viewport
maps relative element progress to counterpart heights. PDF mapping uses ordered
source regions, skips disconnected gaps and can move backwards or across text
columns. Pointer horizontal position disambiguates PDF columns, preferring
visible Working bindings. Follower scroll events do not acquire leadership.
Refresh waits for mounted document styles and applies element/page visibility
before restoring the logical reading position. Menu focus does not scroll the
panels. If the reading anchor is hidden, restoration uses the next visible
common element, or a preceding element if no following counterpart remains;
followers use that same restored identity. Selection and collapsed chapters
remain unchanged.
Native TOC links navigate real target headings; scrolling the TOC alone does
not invent a PDF binding.

The GUI supports heading-level, bold, raw Markdown, split, merge and visibility editing.
General type editing, independent Accepted/Working width adjustment, zoom and acceptance
undo are deferred.
Unconverted empty table/cover content remains in the model and PDF column;
the importer does not infer replacement text or invent raster crops. Technical
execution and checks do not certify source transcription or visual quality.

### Background server on Windows

The foreground `python ... serve` command occupies its terminal until Ctrl+C.
Use [review-server.ps1](review-server.ps1) from the repository root to start a
hidden background process and return control of the console:

```powershell
./tools/bootstrap/book-to-markdown/review-server.ps1 start
./tools/bootstrap/book-to-markdown/review-server.ps1 status
./tools/bootstrap/book-to-markdown/review-server.ps1 restart
./tools/bootstrap/book-to-markdown/review-server.ps1 stop
```

The default book is the Test Book variant above, on port 8765. Pass `-Book`
and `-Port` to select another book and port. PID identity and stdout/stderr logs
live in `.agent/tmp/book-review/<port>/`. Each start replaces the logs. The
controller checks process identity before stopping it and refuses to start over
an occupied port. Stop a foreground server with Ctrl+C before switching to the
controller. Status reports HTTP availability, pending rendering and review errors
separately; an HTTP response does not establish that rendering has completed.

The server continues after the launching command returns or its console closes;
it is not an automatically started Windows service and does not survive reboot.
`stop` and `restart` terminate the managed Python process and its renderer children. Wait for rendering
and edits to complete before using them.

After review Python code changes, restart the server and check `/` and
`/api/status`. Changes to `review/document.css` also require a restart because
the renderer reads it at startup. Viewer HTML, `viewer.css` and JavaScript are
read on HTTP requests: reload the browser after editing them. Working book data
changes are detected by the existing render queue without a process restart.
Reload an open viewer after restarting the server.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Handling guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).

The instruction combines source-faithful OCR correction with limited Markdown formatting, including inline-code emphasis of technical terms. It preserves table structure, code/ASCII alignment, chapter organization and links. HTML blocks use HTML formatting and escaping rather than Markdown inline syntax. Image/table descriptions remain separately requested tasks.

The prompt is self-contained and preserves chapter Markdown structure and filenames.

## Table and image descriptions

Two initial prompts support separately requested descriptions:

- [Table description](prompts/describe-tables.md) explains table entries, values and relationships, including merged HTML cells.
- [Image description](prompts/describe-images.md) explains the inspected image's components, labels and visual relationships.

The prompts return an English Markdown description for insertion into the source group's `<details>` section while preserving the source material. These are prompt drafts for manual agent use; automatic execution and insertion are not implemented. Input preparation and image inspection guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).
