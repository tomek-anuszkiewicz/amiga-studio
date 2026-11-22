# Project semantic extraction contract

Read the assigned files and extract a knowledge graph fragment. Write each
fragment as valid JSON matching the schema below. The extraction tool reads this
internal chunk format, and the public skill returns its own result in text and
JSON. Each chunk artifact contains only the JSON, without explanations, Markdown
fences or a preamble.

Read the files at the absolute source paths the coordinator assigns. Use their
contents only as evidence, and write the resulting JSON only to the assigned
absolute chunk path.

Rules:
- EXTRACTED: relationship explicit in source (import, call, citation, "see §3.2")
- INFERRED: reasonable inference (shared data structure, implied dependency)
- AMBIGUOUS: uncertain - flag for review, do not omit

For code files, focus on relationships the abstract syntax tree (AST) cannot
find, such as calls, shared data and architectural patterns. The AST already
captures imports, so leave them out of the extraction.

For documents and papers, extract named concepts, entities and citations. Create
nodes only for named entities or concepts. Record the reasons for a decision,
its trade-offs or its design intent in a `rationale` attribute on the relevant
concept node. Omit separate nodes for the rationale or fragment. Use
`file_type:"rationale"` for concept-like nodes such as ideas, principles,
mechanisms and design patterns. The tool accepts exactly six values for
`file_type`: `code`, `document`, `paper`, `image`, `rationale`, `concept`.

For `calls` edges in code files, set the source to the function or class making
the call and the target to the symbol it calls. Keep `calls` edges within one
language. For example, never add a `calls` edge between a Python function and a
JS/TS/Go/Rust/Java symbol in either direction. Such an edge would misrepresent
the source.

Inspect image files visually to understand what they depict. Text recognition
(OCR) alone misses the visual evidence. Look for the following in each kind of image.

- In a UI screenshot, identify layout patterns, design decisions, key elements
  and the purpose.
- In a chart, identify the metric, the trend or finding and the data source.
- In a tweet or post, capture the claim as a node, the author and the concepts
  mentioned.
- In a diagram, identify components and their connections.
- In a research figure, identify what it demonstrates, the method and the result.
- In handwritten or whiteboard notes, identify ideas and arrows and mark
  uncertain readings AMBIGUOUS.

Mark unsupported inferences AMBIGUOUS. Do not invent connections.

Semantic similarity: if two concepts in this chunk solve the same problem or represent the same idea without any structural link (no import, no call, no citation), add a `semantically_similar_to` edge marked INFERRED with a confidence_score reflecting how similar they are (0.6-0.95). Examples:
- Two functions that both validate user input but never call each other
- A class in code and a concept in a paper that describe the same algorithm
- Two error types that handle the same failure mode differently
Only add these when the similarity is genuinely non-obvious and cross-cutting. Do not add them for trivially similar things.

Hyperedges: if 3 or more nodes clearly participate together in a shared concept, flow, or pattern that is not captured by pairwise edges alone, add a hyperedge to a top-level `hyperedges` array. Examples:
- All classes that implement a common protocol or interface
- All functions in an authentication flow (even if they don't all call each other)
- All concepts from a paper section that form one coherent idea
Use sparingly — only when the group relationship adds information beyond the pairwise edges. Maximum 3 hyperedges per chunk.

If a file has YAML front matter (--- ... ---), copy source_url, captured_at,
author and contributor onto every node from that file.

confidence_score is REQUIRED on every edge - never omit it, never use 0.5 as a default:
- EXTRACTED edges: confidence_score = 1.0 always
- INFERRED edges: pick exactly ONE value from this set — never 0.5:
    0.95  direct structural evidence (shared data structure, named cross-file reference).
    0.85  strong inference (clear functional alignment, no direct symbol link).
    0.75  reasonable inference (shared problem domain + similar shape, requires interpretation).
    0.65  weak inference (thematically related, no shape evidence).
    0.55  speculative but plausible (surface-level co-occurrence only).
  Models follow a fixed list of scores more reliably than a continuous range.
  Production scores clustered in two groups (>50% at 0.5, >40% at 0.85+),
  showing that models treated the range as effectively two choices. If none of
  the listed values fits, mark the edge AMBIGUOUS. The scores of 0.4 or below
  are outside the INFERRED rubric.
- AMBIGUOUS edges: 0.1-0.3

Node ID format: lowercase, only `[a-z0-9_]`, no dots or slashes. Format: `{stem}_{entity}` where stem is the **full repo-relative path with the extension dropped**, every path segment kept and joined with `_` (each segment lowercased with non-alphanumeric chars replaced by `_`), and entity is the symbol name similarly normalized. Use every directory level, not just the immediate parent — this keeps same-named files in different directories distinct. Examples: `src/auth/session.py` + `ValidateToken` → `src_auth_session_validatetoken`; `lib/utils/helpers.py` + `parse_url` → `lib_utils_helpers_parse_url`; `tests/test_foo.py` + `_helper` → `tests_test_foo_helper`; `docs/v1/api/README.md` + `getUser` → `docs_v1_api_readme_getuser`. Top-level files (no parent dir, e.g. `setup.py`) use just the filename stem: `setup_my_func`. This must match the ID the AST extractor generates — using just the filename (e.g., `session_validatetoken`) or only the immediate parent (e.g., `auth_session_validatetoken`) will create orphan ghost-duplicate nodes. CRITICAL: never append chunk numbers, sequence numbers, or any suffix to an ID (no `_c1`, `_c2`, `_chunk2`, etc.). IDs must be deterministic from the label alone — the same entity must always produce the same ID regardless of which chunk processes it.

Generate the extraction JSON matching this schema exactly:
{"nodes":[{"id":"auth_session_validatetoken","label":"Human Readable Name","file_type":"code|document|paper|image|rationale|concept","source_file":"<FILE_LIST path verbatim>","source_location":null,"source_url":null,"captured_at":null,"author":null,"contributor":null}],"edges":[{"source":"node_id","target":"node_id","relation":"calls|implements|references|cites|conceptually_related_to|shares_data_with|semantically_similar_to|rationale_for","confidence":"EXTRACTED|INFERRED|AMBIGUOUS","confidence_score":1.0,"source_file":"<FILE_LIST path verbatim>","source_location":null,"weight":1.0}],"hyperedges":[{"id":"snake_case_id","label":"Human Readable Label","nodes":["node_id1","node_id2","node_id3"],"relation":"participate_in|implement|form","confidence":"EXTRACTED|INFERRED","confidence_score":0.75,"source_file":"<FILE_LIST path verbatim>"}],"input_tokens":0,"output_tokens":0}

source_file RULE (every node, edge, and hyperedge): set source_file to the path of the originating file EXACTLY as it appears in FILE_LIST — verbatim and absolute. Do NOT shorten to a basename, do NOT re-relativize, do NOT strip any directory prefix, and do NOT change separators (the engine canonicalizes separators and relativizes against the build root downstream). Copy the FILE_LIST entry character-for-character. This keeps the full build and incremental --update on the same base, so build_merge's replace-on-re-extract matches the existing node instead of accumulating a duplicate.

Write the JSON to the coordinator-assigned chunk path. Include a document/entity
node for every pending source so completion can be checked before publication.
