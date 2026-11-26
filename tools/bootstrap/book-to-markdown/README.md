# Book-to-Markdown

Book-to-Markdown imports the results of completed PDF preparation into a
local book editor and exports Markdown chapters. In the editor, you can view
the source PDF alongside Working, the version you edit, and Accepted, the
version you compare it with. PDF acquisition and preparation are separate
steps from import and review.
[PDF-to-Markdown](../pdf-to-markdown/README.md)

Run the commands below in PowerShell from the repository root.

## Local HTML review

### Background server on Windows

Use the server controller to review an imported book while keeping the console
free.

```powershell
./tools/bootstrap/book-to-markdown/review-server.ps1 start
./tools/bootstrap/book-to-markdown/review-server.ps1 status
./tools/bootstrap/book-to-markdown/review-server.ps1 restart
./tools/bootstrap/book-to-markdown/review-server.ps1 stop
```

The controller manages only Book Review and starts the `book-review` service
on demand. Compose owns the process. Port 8765 uses the shared Compose project
`amiga`. If you select a different port with `-Port <number>` or `BOOK_REVIEW_PORT`,
the controller uses a separate project for that checkout and port. The service
listens on the container interface and publishes the port only on host loopback.
Choose an imported book from the **Book** list at `http://127.0.0.1:8765/`.
Check `/api/status` for pending rendering or render errors for each opened book.

The controller uses `compose.yaml` beside this program and generates a mount
override that Git ignores. It defaults to `tools/bootstrap/books` for the library,
`tools/bootstrap/book-to-markdown/config.yaml` for model settings and
`.agent/tmp/book-review-diagnostics` in the primary checkout for container
diagnostics. The controller creates that default diagnostics directory on start.
Controller settings and logs stay in `.agent/tmp/book-review`.
Set `BOOK_REVIEW_LIBRARY`, `BOOK_REVIEW_CONFIG` or `BOOK_REVIEW_DIAGNOSTICS` to
use explicitly selected paths. Use an existing, dedicated directory for an
explicit diagnostics path, separate from workflow evidence, session archives
and controller files. Keep the configuration outside the writable library and
diagnostics roots. Existing source PDFs, identity files, preparation directories and
copied PDF pages are read-only mounts. The library catalog is read-only. Existing imported variant directories are
writable so the editor can replace
Working, Accepted and the root `book.json` atomically. Assets stay in those
host-backed directories. Restart with a fresh mount override after importing
another variant or changing its source layout.

The image combines the pinned Python and JavaScript dependencies with bundled
Chromium. Host `book_review.py` continues to default to Edge.

Before switching an existing default-port installation to `amiga`, stop it
with its original controller and settings. Use the same checkout and mount
settings for subsequent lifecycle commands. Stop any old native server before
using Compose. Foreground servers, Compose servers and explicit book-writing
jobs share the variant's
`.book-review-writer` directory. Ownership starts before a selected book is
opened and ends after its writers finish. An interrupted process can leave
an empty claim directory. Confirm that every writer for the exact variant has
exited before removing that claim.

The list finds imported books at `tools/bootstrap/books/<book-id>/<variant>/working/book.json`.
It takes names from the book directories and includes variant names when a book
has multiple imported variants. Import a book before opening it and reload the
catalog after importing to refresh the list. Opening the catalog only lists
existing imports. Selecting a book starts rendering.

Each book has its own URL, Working/Accepted data, render queue, task state, last
PDF page and PDF pane width. Switching books reloads the editor, clearing open
dialogs and temporary selections. Reopening the catalog restores the last
selected book when it is still available. Separate browser tabs can edit
different books without changing each other's selection. The server settings control the render viewport and browser channel. The
shared `config.yaml` still holds the Codex model settings. TOC settings are stored in each book's recipe. Use only one
server or standalone renderer for a book at a time.

Working updates refresh changed chapters while retaining unchanged elements and
their expanded details. Images and document styles are reused by content hash
across Working and Accepted. Source text is cached by recipe and source-page
version; changes to a source page invalidate the affected text.

#### Enable Codex in the container

Check the Windows sign-in with `codex login status`. Use `codex login` and
choose ChatGPT if you need to sign in. Book Review starts its own Codex process
inside the container and uses your ChatGPT sign-in.

To add or change authentication on an existing instance, first let edits
finish. Run `./tools/bootstrap/book-to-markdown/review-server.ps1 stop`
from the owning checkout with that instance's existing port and mount settings.
The controller uses those settings to identify the container. Then select the
private login file and keep the intended library and other mounts for the next start.

```powershell
$CodexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE '.codex' }
$env:BOOK_REVIEW_AUTH = Join-Path $CodexHome 'auth.json'
./tools/bootstrap/book-to-markdown/review-server.ps1 start
```

Use the actual file-based ChatGPT login if it lives at another path. Keep
`auth.json` outside the repository, the image build context and writable
mounts. The controller passes the path to Compose as a read-only mount. At
startup the container copies the file into its temporary Codex home. Use the
same authentication path for later controller commands. If the copied login
expires, refresh the selected login on Windows and recreate the service.
Check Codex with a small edit on a disposable imported book.

#### Move Book Review to the primary checkout

For service development, use a separate port and a disposable, independent
library. For example, select that library with `BOOK_REVIEW_LIBRARY` and run
`./tools/bootstrap/book-to-markdown/review-server.ps1 start -Port 18765`
from the task checkout. Stop that instance from its task checkout when finished.
To try the default-port service
against the usual library, stop the existing instance before starting the
worktree instance. Book ownership allows only one writer for each variant.

After merging changes to the installed service, check its current checkout.
Run `docker compose ls` or inspect the container's
`com.docker.compose.project.config_files` label. The controller checks those
paths before it acts. Even a stopped container created from the worktree must
be removed before the controller in the primary checkout can manage port 8765.

Before removing the worktree, let active edits finish and identify the current
port and mount settings. Use the recorded library, configuration, diagnostics
and optional authentication paths. If any required input is inside the worktree,
make it available in the surviving checkout before removal. With the original
settings and port, run `./tools/bootstrap/book-to-markdown/review-server.ps1 stop`
from the owning checkout. It stops and removes just `book-review`.
Then select the surviving settings and
run these commands from the primary checkout for the usual service.

```powershell
./tools/bootstrap/book-to-markdown/review-server.ps1 start -Port 8765
./tools/bootstrap/book-to-markdown/review-server.ps1 status -Port 8765
```

Confirm the expected library at `http://127.0.0.1:8765/` and in `/api/status`.
If the expected library is imported, open a known book to confirm the service
can use it.

### Starting a new review

Install the local review dependencies once.

```powershell
python -m pip install -r tools/bootstrap/book-to-markdown/requirements.txt
npm install --prefix tools/bootstrap/book-to-markdown --ignore-scripts
```

The renderer uses installed Microsoft Edge by default. To use bundled Chromium
instead, run `python -m playwright install chromium` and pass `--channel=`
to `book_review.py`.

Prepare the selected PDF pages through Stage 08 and check that Stages 02, 06, 07
and 08 succeeded.

For each page, import looks for JSON from Stage 08, 07, 05, 04 and 03, in that
order, and uses the first file it finds. Stages 04, 05, 07 and 08 write only
changed pages, so their directories can be empty after successful preparation.
An empty `segments` list in the selected page JSON excludes that page. Import
keeps all retained pages in physical-page order, regardless of the last
invocation's page range.

When import parses Markdown math, it decodes named and numeric HTML entities
for angle brackets into `<` and `>` before creating PDF text references.
This decoding is confined to angle-bracket entities in parsed math. It leaves
code, preformatted text and ordinary HTML entities as written.

**Import deletes and rebuilds Working, Accepted and the copied PDF pages.** It
discards existing review edits and settings. Run these commands for a fresh
review. To resume a review, open the existing book.

```powershell
$Book = "tools/bootstrap/books/test-book-example-4567/78e52f075e17"
python tools/bootstrap/book-to-markdown/book_review.py import --book $Book
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
./tools/bootstrap/book-to-markdown/review-server.ps1 start
```

The first render initializes Accepted from the new Working book.
Selecting a book also renders when needed, so the separate `render` command
is optional. Choose the imported book in the browser after starting the server.

For a foreground server, run
`python tools/bootstrap/book-to-markdown/book_review.py serve`
and stop it with Ctrl+C. Use `--port` to change the port. Use `--width` to
change the browser viewport used during preparation. The displayed document
width is separate.

### Reconstructing a cloned book

After restoring the variant's matching `source.pdf`, use the accepted recipe at
the variant root to rebuild local editor inputs:

```powershell
$Book = "tools/bootstrap/books/test-book-example-4567/78e52f075e17"
python tools/bootstrap/book-to-markdown/book_review.py import-book --book $Book
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
./tools/bootstrap/book-to-markdown/review-server.ps1 start
```

`import-book` runs PDF Stages 01 and 02 for the full source, generating the OCR
PDF, positioned text and page PNGs. It copies the page PNGs into the editor,
extracts image crops from `book.json` PDF regions, copies the accepted recipe to
`working/book.json`, and exports Markdown. It does not run model conversion,
proofreading or the draft-generating `import` command. The first render creates
Accepted from the restored Working recipe; the root `book.json` stays intact.

The command checks `source.json` when present and compares a tracked
`source-ocr.json` with the OCR profile and processed page selection. OCR PDF
bytes may differ across preparation runs; `import-book` reports that difference
and validates that the saved text ranges still resolve. A matching source PDF
and stable positioned text are required for reliable source references.
It refuses to replace an existing Working or Accepted review. If a recipe
uses an edited image that requires more than a single PDF crop, it needs the
edited asset or an explicit image transformation recipe. `import-book` reports
this case without substituting another image. After reconstruction, run
[source validation](#source-validation) and inspect the PDF. A range can pass
validation while pointing to the wrong printed occurrence.

### Continuing a review

Edit Working in the browser. Compare it with the original prepared pages in
the PDF pane and with the Accepted baseline. **Accept Changes** publishes the
completed Working revision to Accepted and the variant-root `book.json`.
**Discard Changes** restores the complete Working bundle from Accepted. An
element's **Discard** restores only that element's properties. These actions
operate on local review files without committing or uploading them.

Use **Book Actions → Clean direct book text** to preview possible PDF references
for plain book text. Confirm each proposed occurrence against the PDF before
applying it. **Show source differences** helps with that review by marking
source-backed corrections in yellow, accepted text without a source address
in blue and pending plain text in purple. Transcription quality still depends
on checking the source.

Use **Book Actions → List unmatched book text** to find text without a PDF
source reference, including accepted literals, chapter titles and hidden
elements. Each listed fragment includes its PDF page, element and context.
The list omits HTML tags and math command names but includes words and
numbers inside formulas. This action is read-only and applies no matches.
The statistics count all fragments and report how many contain at least three
letters. The list displays only those fragments. It counts only letters
toward the threshold, excluding digits and punctuation.
Use **Go to Book** beside a fragment to navigate to its element and highlight
the text. Navigation also reveals a hidden element when necessary.

Use **Book Actions → Rename Chapters** to edit Working chapter titles and Markdown
filenames together. The generated table of contents has its own entry. Each
chapter title is separate from its filename. The editor aligns changed titles
to PDF text blocks where possible. **Clean direct book text** checks title text
that still has no reference, including an edited contents title.
Filenames must be unique `.md` names within the bundle. Saving a rename updates
the Working Markdown exports and contents links and removes the old generated
filenames. Accept Changes publishes the renamed recipe and files.

Refresh a complete Working Markdown export after the current revision renders:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py export --book $Book
```

After accepting the final revision, copy its Markdown chapters and referenced
images into `Obsidian/Amiga/Reference/<book-directory>/`:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py publish --book $Book
```

The directory name comes from the book directory above the variant. Publication
takes the files already in Accepted, then converts generated block links to
ordinary Markdown links to chapter headings and leaves review-only element
anchors and Obsidian block IDs out of the published copy.
Repeated heading text receives a numeric suffix in the published chapter and
contents entry so every link names one heading. Publication also writes each
visible image's `description` from the accepted root `book.json` to
`assets/<image-name>.txt`. The command skips the sidecar when an image has no description. It omits
hidden images. The command requires the root and Accepted recipes
to match and refuses to replace an existing Reference directory. It leaves
Working and Accepted intact.
The published contents use relative Markdown links you can follow in
Obsidian and a standard Markdown preview. The review exports still use Obsidian
block links so identical headings retain distinct destinations during editing.
Before a commit, review each published Markdown file and add the `book`,
`chapter`, and content-based tags described in
[Reference Book Metadata](reference-book-metadata.md).
The commit check reports missing structural metadata. An agent asks before
editing frontmatter.

## Book authoring sequence

1. Prepare the source PDF through [Stage 08](../pdf-to-markdown/stages/08_proofreading/README.md).
2. Import once, then arrange and correct chapters and elements in Working.
3. Run [source validation](#source-validation) and inspect source differences
   against the PDF. A valid address can still point to the wrong repeated text.
4. Review page corrections, chapter organization, formatting, tables, graphics,
   and PDF source selections.
5. Review the rendered book and exports, accept the intended revision, and
   commit its `book.json` with the matching `source.json`. Preparing the four
   manual recipes remains in the
   [roadmap](../../../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion).

With a matching PDF and a committed recipe, you can reconstruct the book,
add descriptions, publish Markdown and index it for RAG.

## Documentation bootstrap from an accepted `book.json`

Repeat this sequence for each reviewed book. The variant needs its accepted
`book.json`, matching `source.pdf`, and `source.json`; check the full source
SHA-256 as described in [source catalog](#source-catalog-and-variant-layout).
The RAG CLI and shared index state file are configured in the
[developer setup](../../../.agents/references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag).

1. Run `book_review.py import-book --book <variant-directory>` to rebuild the
   editor from the accepted recipe and PDF. Then render and open the review
   server as in [reconstructing a cloned book](#reconstructing-a-cloned-book).
   `import-book` refuses an existing Working or Accepted bundle. Use it to
   reconstruct the book from the recipe and resume active reviews by opening
   the existing book.
2. Validate the reconstructed book and inspect its text, tables, graphics,
   chapter links, and PDF references. If image or table descriptions are still
   missing, use the Book Review Codex actions on Working, inspect the results
   against the PDF, and choose **Accept Changes**. This updates Accepted and
   the variant-root `book.json`; descriptions already in the recipe need no
   new model request. Review ASCII art in its source context.
3. Run `book_review.py publish --book <variant-directory>` after acceptance.
   It writes Markdown, images, and accepted image-description sidecars to
   `Obsidian/Amiga/Reference/<book-directory>/`. It refuses an occupied target;
   inspect the existing publication before any replacement. Review the
   published files and add the required [reference metadata](reference-book-metadata.md).
4. After validating the published Markdown, follow the
   [RAG indexing procedure](../../../.agents/skills/TA-RAG-index-amiga/SKILL.md)
   for all three Amiga documentation scopes. The CLI skips unchanged files.

5. Check the index status and search for distinctive chapter text, table
   descriptions, and image descriptions under source `amiga`. Compare returned
   fragments with the published Markdown and image `.txt` sidecars. The
   [RAG indexing workflow](../../../.agents/skills/TA-RAG-index-amiga/SKILL.md)
   has the status and search commands.

## Source catalog and variant layout

The default local catalog stores source variants under `tools/bootstrap/books/`.

```text
tools/bootstrap/books/<book-id>/<first-12-characters-of-sha256>/
  source.pdf          # local source or assembled composite PDF
  source.json         # tracked full SHA-256
  pdf-to-markdown/    # local preparation workspace
```

The current composite Test Book variant is
`test-book-example-4567/78e52f075e17`. Its
[page tracker](../books/test-book-example-4567/78e52f075e17/test_pages_tracker.md)
records the source variants and 160-page assembly order. The other source
variants have their own `source.json` files. Git tracks the metadata, while
source PDFs and generated outputs stay local.

When acquiring a source PDF, compare its **full** SHA-256 with the variant's
`source.json`, copy the matching file to `source.pdf`, and verify the copied
file's hash. For example:

```powershell
Get-FileHash -LiteralPath './downloaded.pdf' -Algorithm SHA256
```

If the hash differs, obtain the matching source or create a separate variant.
Changing an existing variant's hash cannot make a different PDF equivalent.

## Editing and publication

`working/book.json` stores the authored review model. The variant-root
`book.json` stores the accepted recipe for later publication. Generated
Markdown, renders and assets are replaceable local outputs. The PDF supplies
source text and graphics, and the recipe records chapter order, element roles
and visibility, Markdown structure, source regions and explicit corrections.

Source-derived `md_text`, chapter titles and filenames use Stage 02 physical
page, block and Unicode character ranges. For example,
`{{text page=3 block=1 chars=0:6}}` selects characters 0 through 5.
`{{accepted text="Vision"}}` records visible text without a PDF address.
Formatting around references remains literal Markdown or HTML.
The editor derives `resolved_md_text` for display from authored recipe content.
The [element vocabulary](elements.md) lists supported types and roles.

Import aligns the latest prepared page text with Stage 02 PDF text. OCR and
repeated phrases can make source attribution uncertain or incorrect, so the
reviewer must check the actual PDF. The recipe stores reviewed decisions so
`import-book` can use them during later reconstruction without repeating model
inference.

### Source validation

```powershell
python tools/bootstrap/book-to-markdown/book_review.py validate --book $Book
```

This read-only command checks Working recipe references and reports visible
literal content without source addresses. It prints JSON and exits with code 1
when the audit fails. A valid range still needs to be checked against the PDF
to confirm that it identifies the correct printed occurrence.

Math formatting and spacing commands need no source address. Their text
arguments still need one. Source validation reports other math commands as
`unreferenced_math_command`. The renderer checks whether it supports their
syntax.

## Codex Working task

Use **Book Actions → Codex...** for tasks across Working, or the context menu
to describe one selected image or table. The suggested tasks and description
rules live in [`prompts/`](prompts/); the authored `description` field and its
rendering are defined in [elements.md](elements.md#notes-legends-and-groups).

Codex edits Working at the current revision. Accepted and PDF sources are
read-only. One-element transformations preserve source content by default and
follow the [transformation prompt](prompts/codex-edit.md). Review
changes before acceptance. **Cancel task**, Escape, and
**Stop task** interrupt the active request. Working retains edits that have
already been saved. The model and effort are selected in
[config.yaml](config.yaml).

Task and element-transform diagnostics use `book-codex-agent/<request-id>/`
and `book-codex/<request-id>/` within the selected diagnostics directory.
Foreground host runs use the current checkout's `.agent/tmp`.
Shared authentication and cache behavior are in the
[conversion client README](../conversion/README.md).

## Tests

```powershell
python tools/verification/verify.py test python directory tools/bootstrap/book-to-markdown/tests
```
