# Shared Conversion Client

The PDF, HTML and explicit Book Review Codex commands use `CodexClient` to
authenticate, run requests, cache completed responses and record metrics. To
configure or run a program, use its README. To find the rules for what the
model should produce, use its prompt.

## Execution

The transport sends model requests through the pinned Codex SDK/app-server
runtime using ChatGPT authentication. It has no fallback to API-key billing.
[Transport](transport.py)

Each request starts
a fresh read-only thread with the explicitly selected model and reasoning effort.
Base/developer instructions are empty, repository instructions are disabled, and
shell, web, MCP, app and agent tools are disabled.

Each request carries the prompt and source data. The transport sends applicable
images at `detail: original`. It sends no separate output schema. The transport
checks the settings actually used by the thread and confirms completion before
returning a result. Failures stop conversion without switching model, effort or
provider. The transport accepts an `imageView` event only if its resolved path
matches the resolved path of an image attached to the same request. It logs the
repeated view and continues waiting for the final response. It rejects all other
image paths and tool items.
For HTTP 429/503, `rateLimitExceeded` and `serverOverloaded` errors, the
converter closes the affected transport before retrying the same request
through a fresh transport. It waits 60 seconds before the first retry. With
consecutive retryable failures the wait doubles, following the sequence 60,
120, 240 seconds and so on, with no retry limit. A successful
response resets the next retry wait to 60 seconds.
Each attempt has its own request timeout. Time spent waiting to retry falls
outside that timeout. The converter prints those waits even when tracing is
off. You can interrupt them with Ctrl+C.
Other failures, including usage and session-budget limits, still stop conversion.
The converter stops runtime retries for capacity notifications immediately and
controls the timing of each retry itself. In parallel PDF stages, each worker
has its own transport and calculates its retry waits independently.
In parallel PDF stages, coordinator interruption or a worker failure signals
all clients to cancel active transports and interrupt request delays and retry
waits before the coordinator drains its workers.
`llm.post_request_delay_seconds` optionally delays a new request to the same
client until the specified time has elapsed since the previous model response.
The delay excludes response generation time. A cache hit neither waits nor
resets the delay. Set `llm.concurrency: 1` to apply the delay to the entire PDF
conversion stage.

Set `CONVERSION_TRACE=1` to print a concise live trace to stderr. It shows
waits imposed by request spacing, requests sent to Codex, and request completion
or failure. The trace gives the worker, thread and turn IDs for sent requests.
It omits prompt, image and response content and fragments of streaming
notifications. Cached results produce no trace events. An allowed `imageView`
is an event within the same request. The converter handles it without sending
a follow-up turn.

The HTML converter checks configuration and capabilities and validates Markdown
responses. PDF Stages 03 and 05 parse JSON without enforcing a local response
schema. Stage 08 validates proofreading patch targets before applying the
patches. A successful request confirms execution. Transcription quality
requires a separate assessment.

## Cache and metrics

The default location is `tools/bootstrap/.cache/codex`. The cache identifies
a request by its stable operation ID, model and effort, pinned runtime,
contract version, execution settings, prompt, ordered image-byte hashes and
image detail. Stage names and numbers label execution only.
[Response cache](cache.py)

The cache stores only completed responses. It excludes invalid JSON, invalid
HTML-converter Markdown and JSON rejected by an explicitly supplied application
validator. That validator also checks cached responses when the client reuses
them. Metrics distinguish new calls, cache hits and execution time. Token usage
returned from cache belongs to the original request.

JSON parsing failures include the full model response, stage, operation and input
image paths in exception notes printed to the console. Stage 05 also identifies
the failing table. These diagnostics do not repair or cache invalid responses.
