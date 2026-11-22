# Stage 08: Proofreading

Stage 08 reads all retained page objects from the latest available JSON in
Stages 07, 05, 04 and 03, in that order. It omits pages whose latest JSON has
empty `segments`. For each selected page, the stage sends its ordered text
objects to the shared Codex client along with short excerpts from the edges of
neighboring retained pages. The first request omits the page image and the
complete Stage 02 positioned text. If the response leaves any items unresolved,
the stage sends a second request for visual evidence. It includes the current
page text, the unresolved items and the full Stage 02 page image. For each
flagged segment, the second response supplies a correction or gives a reason to
leave the segment as written.

Stage 08 uses the `08_proofreading` model settings in `config.yaml`, the shared
request cache and stage metrics. The text and image requests have separate
cache keys. The stage applies only validated `md_text` replacements and keeps
all other JSON fields and the object order. It writes the complete page to
`08_proofreading/page_*_segments.json` only if the final page differs from its
input. Its only page output is the complete changed `page_*_segments.json`.
The stage produces no issue counts or review reports.

Book import requires Stage 08 to succeed. It reads each retained page from the
latest available JSON in Stages 08, 07, 05, 04 and 03. An empty Stage 08 directory
is valid. Restarting with selected pages clears the Stage 08 page files for
those pages.

The console shows each page request, the response call and cache-hit counts,
the image request when needed, and `done` after it finishes the page.
`--trace-codex` reports when a request waits, is sent, completes or fails. The
trace omits request content. With one configured worker, conversion runs on the
thread that invoked it so `Ctrl+C` can interrupt the wait between requests.
Abrupt termination can leave the last stage status as `running`. Check that no
pipeline or proofreading process remains before restarting.
