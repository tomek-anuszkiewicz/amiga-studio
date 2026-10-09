# Bootstrap: Temporary Textbook Focus

These instructions apply to `tools/bootstrap/` and its descendants, alongside the repository-wide [instructions](../../AGENTS.md).

The current priority is preparing the textbook and developing the surrounding PDF-to-Markdown workflow: PDF preparation, book/chapter generation, recipes, prompts, proofreading, descriptions, and directly related documentation. Follow the [temporary textbook skill](../../.agents/skills/textbook-pdf-markdown/SKILL.md) for this work.

Keep work tied to the user's current book task. Emulator development and unrelated bootstrap expansion remain outside this temporary focus unless explicitly requested. A new explicit user request takes precedence over this default priority.

This focus remains active until the user changes it. When that happens, revise or remove this section and the temporary skill together; retain generally applicable conversion contracts.

## Converter development

For book import, recipe generation and reconstruction, read the local
[book instructions](book-to-markdown/AGENTS.md). PDF text/graphics are the source;
`book.json` stores the reconstruction decisions, rather than references to complete
converted Markdown segments. Fix the generating code when the recipe is wrong;
do not substitute manual repairs to generated book data.

Follow the [developer-led workflow](reference-conversion-contract.md#development-workflow): make the requested code, configuration or prompt change, convert the user-selected fragment only when requested, and let the user assess the output. Automatic tests follow the [bootstrap converter scope](../../.agents/rules/unit-testing-policy.md#bootstrap-converter-scope). Report technical execution separately from conversion quality.

## Preparing the book source collection

Source collection is an authoring prerequisite, not part of the ordinary user's chapter-generation workflow. When source acquisition is requested, use the [book catalog and variant layout](book-to-markdown/README.md#source-catalog-and-variant-layout):

1. Find and download the required manual and edition locally. Download filenames and hosting locations may differ between mirrors.
2. Calculate the full SHA-256, for example with `Get-FileHash -LiteralPath './downloaded.pdf' -Algorithm SHA256`.
3. Compare all 64 hexadecimal characters with the variant's `source.json` value. A title, page count or 12-character prefix alone does not identify the accepted PDF. For a mismatch, obtain the matching source or deliberately author a separate variant; do not replace an existing variant's hash to accept another download.
4. Copy the matching PDF to `book-to-markdown/books/<book-id>/<first-12-characters-of-sha256>/source.pdf` and verify the copied file's full SHA-256 again.

For Test Book reconstruction, use the page tracker linked from the catalog to preserve its source mapping and assembly order, then verify the full resulting hash.

## Review server during development

Use the [review server controller](book-to-markdown/README.md#background-server-on-windows)
to run the local browser review server in the background and inspect its PID,
HTTP status and logs. After changing review Python code, restart the running
server and check `/` and `/api/status` before reporting the change ready for review.
After viewer HTML/CSS/JavaScript changes, reload the browser; no Python restart
is required. Changes to the renderer's `review/document.css` require a restart
because the renderer snapshots that stylesheet at startup. Working book data is
detected by the existing render queue.
If the server was stopped, start it when the requested work includes browser
review. Report HTTP availability separately from pending rendering or render errors.

## Manual chapter processing

For requested proofreading, use the [proofreading prompt](book-to-markdown/prompts/proofreading.md) and the generated chapter. Inspect linked images when needed. Keep the generated source chapter available for comparison and retain the edited copy outside generator-managed output cleanup. Preserve chapter Markdown structure and filenames; table and image descriptions are separate requested tasks.

For a requested [table](book-to-markdown/prompts/describe-tables.md) or [image description](book-to-markdown/prompts/describe-images.md), supply one already identified group in a `<details>` section: its name or caption, associated notes and legend, and the Markdown/HTML table or image reference. Use that grouping directly and inspect the actual image for image descriptions. These prompts support manual agent processing; do not describe automatic execution or insertion as implemented.
