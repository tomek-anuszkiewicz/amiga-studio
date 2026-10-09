# Published book recipes and Markdown reconstruction

The intended published `book.json` is a recipe for reconstructing Markdown from
an exact source PDF. The repository should contain the reviewed structure and
conversion decisions; source PDFs and generated bundles remain local. This is
the target end-to-end design. Text templates are implemented in the editor
import/load path; a standalone bootstrap reconstruction command is still pending.

## Intended bootstrap flow

1. Obtain the source PDF from the book catalog and verify the complete SHA-256
   recorded in the variant's `source.json`. The 12-character directory suffix
   is a naming convention, not sufficient verification. Automatic acquisition
   and recipe execution are pending.
2. Preserve native text. Where pages lack text, prepare the full source with the
   exact OCR profile used during authoring. The current implementation and its
   pins are described in [Stage 01](../pdf-to-markdown/stages/01_text_layer/README.md).
3. Extract positioned text from the prepared PDF, using the recipe's extraction
   and coordinate conventions. Identify the prepared text content used during
   authoring; an arbitrary existing `source-ocr.pdf` is not evidence of matching
   OCR settings.
4. Resolve recipe elements from PDF text and graphic regions, apply reviewed
   formatting and explicit corrections, and assemble chapters and the generated
   TOC. Reconstruction should not need new model calls or repeat classification.

The existing importer instead needs local Stage 02/04/05/06 artifacts. Its
`working/book.json` and `accepted/book.json` contain `md_text` and asset filenames.
New imports use Stage 04/05 as authoring input: Markdown/HTML structure stays
literal while matching source text becomes Stage 02 block/range references.
Source selections may occur in a different order from PDF extraction. Unmatched
transcription/corrections remain explicit literals. These recipes load from PDF
text ranges without consulting converted Markdown. Existing authored models with
complete Stage 04/05 Markdown references remain readable until explicitly regenerated.
Acceptance copies a local bundle,
preserving templates; it does not publish a recipe or change
Git tracking.

## Source text and authored decisions

For source-derived prose and headings, the published recipe should reference
positioned PDF text rather than duplicate the transcription in `md_text`.
Element IDs, order, type, visibility, chapter organization, heading level and
numbering remain authored decisions. Chapter titles and output filenames accept
literal text and PDF text references using the same syntax as `md_text`. The
loader resolves them before presentation and export, retaining their authored
templates on save and acceptance.

Current `pdf_regions` contain physical page numbers and pixel rectangles for
the 300-DPI page images. They locate content visually but do not define an exact
text selection. A rectangle can intersect neighboring columns, a fraction of a
line or several table cells. A recipe needs ordered text selections with an
explicit coordinate frame and extraction order, including disconnected regions
and elements spanning pages. The importer preserves converted Markdown structure
and selects equal source character ranges from positioned blocks intersecting
the element region; it does not replace Markdown with concatenated block text.
Source geometry remains separate from those text selections. Region-based
character selection requires character geometry when
authoring selections; existing authored references still load. PDF references
identify physical page, extracted `block_id` and zero-based
Unicode character ranges with exclusive ends. Rectangles choose candidates;
references select exact characters. Alignment preserves output but does not
certify semantic source attribution. Imported elements currently use one page;
manually authored templates can combine references from multiple prepared pages.

The PDF text layer does not encode Markdown structure. Tables need reviewed
cell/row structure, spanning and reading order; code and ASCII art need explicit
spacing rules; math needs an authored representation. Those decisions cannot be
recovered by concatenating PDF spans. Graphics need a source crop and its
coordinate/rotation rules rather than a dependency on a local `assets/` file.

Proofreading corrections, descriptions and other intentionally authored content
must remain explicit recipe data. Re-extracting the original PDF would otherwise
discard accepted corrections. A source reference plus an explicit replacement
or transformation can be represented by literal template content between
source references. Import retains unmatched model transcription/corrections as
explicit literals alongside PDF references, adding a heading-level prefix where
required. Textless converted objects remain textless. Tables retain their prepared
markup with matching cell text replaced by PDF ranges. This exact-character
alignment is not a reviewed semantic cell map. Standalone reconstruction from the original PDF alone
still needs a source-cell map and a representation of these corrections; the
current import is not that reconstruction command. See the
[template contract](README.md#editing-and-publication)
for syntax, escaping and literal corrections. All text uses that syntax;
there is no legacy plain-Markdown book format.

## Reproducibility boundary

Source SHA-256 belongs to the variant's `source.json`; library and OCR settings
belong to the tracked preparation profile. The book stores source selections and
authored reconstruction decisions without duplicating those identities or adding
page-text hashes. The loader resolves referenced preparation segments or PDF
blocks and checks character bounds for PDF ranges. Markdown references require
their Stage 04/05 artifacts, not only the original PDF. Rebuilding Stage 02 and verifying the original PDF
directly remain outside this importer; standalone reconstruction is not implemented.

The OCR profile pins PyMuPDF/MuPDF versions, language data by SHA-256 and
render/save parameters. New preparation writes a source-adjacent provenance
record. Existing prepared PDFs remain reusable by existence and cannot acquire
historical provenance from the current installation. Their original runtime is
unknown unless separate evidence exists.

Version pins do not establish byte-identical PDFs across platforms or custom
library builds. Prepared-PDF hashes identify actual artifacts; recipe text
identity and cross-platform reconstruction still require a defined contract.
No existing Accepted bundle is certified reproducible by this design note.
