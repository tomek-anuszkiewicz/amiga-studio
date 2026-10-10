# Shared Conversion Client

The PDF and HTML programs use `CodexClient` for authentication, request execution,
completed-response caching and metrics. Their own README files document program
configuration and invocation; their prompts define inference behavior.

## Execution

The [transport](transport.py) uses the pinned Codex SDK/app-server runtime and
ChatGPT authentication. API-key billing is not a fallback. Each request starts
a fresh read-only thread with the explicitly selected model and reasoning effort.
Base/developer instructions are empty, repository instructions are disabled, and
shell, web, MCP, app and agent tools are disabled.

Requests contain the prompt and source data, with applicable images sent at
`detail: original`. No separate output schema is sent. The transport verifies
the actual thread configuration and completion before returning a result.
Failures stop conversion without switching model, effort or provider.
Rate, usage-budget and server-capacity error notifications close the affected
transport immediately, including notifications for which the runtime proposes
a retry. The converter does not retry those requests. PDF parallel workers own
separate transports so a timeout cannot close another worker's connection.

HTML performs configuration/capability checks and validates Markdown responses.
PDF parses JSON without local response-schema enforcement. A successful request
confirms execution, not transcription quality.

## Cache and metrics

The [response cache](cache.py) defaults to
`tools/bootstrap/book-to-markdown/books/.cache/codex`. Identity includes the stable
operation ID, model/effort, pinned runtime, contract version, execution settings,
prompt, ordered image-byte hashes and image detail. Stage names and numbers are
execution labels rather than cache identity.

Only completed responses are cached; invalid JSON or invalid HTML-converter
Markdown is not reused. Metrics distinguish new calls, cache hits and execution
time. Token usage returned from cache belongs to the original request.
