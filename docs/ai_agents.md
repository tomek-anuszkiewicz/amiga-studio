# Autonomous AI Agent Engineering & Pair-Programming Guide

This repository is engineered from the ground up for autonomous AI agent pair-programming with strict architectural guardrails.

> 📖 **Case Study & Engineering Narrative:** For an in-depth retrospective on how this emulator was built with zero hand-written human code through architectural sparring, minimal frame prototyping, and an evolving harness, see [How This Emulator Was Written: Pair-Programming with an AI Agent](how_this_emulator_was_written.md).

---

## 1. Architectural Guardrails & Invariants

All automated modifications and agent sessions must strictly adhere to [`AGENTS.md`](../AGENTS.md) and modularized rules under [`.agents/rules/`](../.agents/rules/):


---

## 2. Knowledge Retrieval: RAG & Graphify

### A. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- **Knowledge Base Scope:** Connects to local Qdrant database (`http://localhost:6333`, collection: `projects_docs`), indexing Commodore Hardware Reference Manuals, M68000 PRMs, technical specs, and design specs under `Obsidian/Amiga/`.
- **Project Integration ([`amiga-rag.md`](../.agents/rules/amiga-rag.md)):**
  - The `rag_qdrant` CLI is the canonical index and retrieval interface; the MCP server is an optional adapter.
  - After a successful `audit-docs-quality` run under its normal trigger conditions, set `RAG_INDEX_JSON` and incrementally index `docs`, `Obsidian/Amiga/Design`, and `Obsidian/Amiga/Reference` with the three commands in `index-amiga-rag`. Do not index the `Obsidian/Amiga` parent; the CLI has no `--reindex` mode.
- **Trusted Data Boundary & Provenance:**
  - The local RAG vector store and Graphify AST graphs operate exclusively on a trusted local boundary.
  - Only authoritative Commodore/Motorola hardware reference manuals and internal design specs should be placed in `Obsidian/Amiga/`.
  - Pinned upstreams in `tools/bootstrap/bootstrap.ps1` and strict architectural isolation between guest 68000 emulation and host LLM prompting prevent indirect prompt injection risks.
  - **Query Tools:** Query via MCP tool: `rag_search(query="<topic>", sources=["amiga", "devnotes"])`. `devnotes` is maintained by a separate project and is retrieval-only here.

### B. Code Structure & Relationships: AST Knowledge Graph (`graphify`)
- **What is Indexed:** Graphify parses Abstract Syntax Trees (AST), symbol relationships, and call hierarchies across two distinct codebases:
  - **Active Emulator Crates (`crates/`):** Core Rust workspace (`m68000`, `memory_bus`, `debugger`, `gui`, `config`, `rtc`, `test_runner`).
  - **Reference Emulator Sources (`ref_src/`):** Clean C++ reference implementation (`ref_src/vAmiga`) and external test harnesses.
- **Incremental Knowledge Graph Updates ([Graphify skill](../.agents/skills/graphify/SKILL.md)):**
  - Graphify maintains an AST cache that extracts only modified files in 1–2 seconds without LLM calls.
  - After modifications to code in `crates/` or `ref_src/`, run `graphify update .` from the repository root (or via `.\tools\bootstrap\bootstrap.ps1 -Graphify`).
  - This keeps a single unified knowledge graph in `graphify-out/` connecting active emulator crates and reference implementations.
- **Query Tools:**
  - `graphify query "<question>"`: Query symbol dependencies, call hierarchies, and architectural boundaries.
  - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
  - `graphify explain "<concept>"`: Extract a focused subgraph explaining a subsystem or module.

---

## 3. Specialized Agent Skills (`.agents/skills/`)

Browse [`.agents/skills/`](../.agents/skills/) for available procedures. Each
`SKILL.md` contains its name, description, use conditions and execution steps.
Project rules live in [`.agents/rules/`](../.agents/rules/); [AGENTS.md](../AGENTS.md)
routes required reads and shared obligations. No separate skills or rules register
is maintained in this guide.

## Bootstrap Conversion Programs (`tools/bootstrap/`)

Initial reference conversion is handled by Python CLI programs documented in their own READMEs:

- [`pdf-to-markdown`](../tools/bootstrap/pdf-to-markdown/README.md): A 14-stage PDF conversion pipeline with Gemini processing, asset extraction, and optional manual correction.
- [`html-to-markdown`](../tools/bootstrap/html-to-markdown/README.md): HTML and downloaded crawl conversion with Gemini transcription, asset processing, and validation helpers.

[`bootstrap_documentation.ps1`](../tools/bootstrap/bootstrap_documentation.ps1) dispatches conversion during bootstrap. Routine agent work consumes the generated Markdown and assets; see [Processing Raw Documents into Markdown](developers.md#processing-raw-documents-into-markdown) for the tooling overview.

---

## 4. Invoking Skills in Codex

Codex discovers repository skills from `.agents/skills/`, loading each skill's
name and description first and its full instructions when selected. Use `$` to
mention a skill (for example, `$audit-code-quality` or `$test-runner`), or `/skills`
to select one. Codex may also select a skill when a task matches its description.

The skill files include execution ordering, targeted commands,
verification, and reporting. Conditional detail is linked from the owning skill,
such as the [manual diff checklist](../.agents/skills/audit-code-quality/references/manual-diff-checklist.md).

`AGENTS.md` retains shared
obligations and required actions; task rules route to procedures that must run.
Rules are explicitly read Markdown, while skill selection starts from `name`
and `description`. Keep trigger boundaries in that description, not only in a
later section of the skill body.

See [OpenAI's skill documentation](https://learn.chatgpt.com/docs/build-skills)
for discovery and invocation behavior.

### Agent Tooling Maintenance

- Rules own obligations, scope, and exceptions. Skills own steps, commands,
  diagnostics, and templates; conditional examples live in the owning skill's
  `references/` directory. Mechanical requirements belong in scripts and tests.
- Keep verification orchestration and coverage result processing in `tests/`,
  worktree utilities in `tools/harness/`, and concrete audits and checks in `tests/checks/`.
  The diary appender lives in `.agents/skills/compact-diary/scripts/`.
  A specialized script used by at most one skill belongs under that skill's
  `scripts/`; a shared script uses the directory matching its role. Supporting
  references count as part of their owning skill, not as additional consumers.
- Adding, removing, or renaming a rule requires updating active links and
  affected procedure references. No manual audit registry is required.
  Historical diary references remain historical. Reach design specifications
  through domain rules or the design index; do not duplicate every document link.
- Passive rules do not need duplicate skills. Active procedures must remain
  reachable through their owning skills. Report concrete executed checks and
  their limits; static checks do not establish complete behavioral enforcement.
### Recording and Validating Changes

Commit policy is in [git-commits.md](../.agents/rules/git-commits.md); diary policy
is in [diary-maintenance.md](../.agents/rules/diary-maintenance.md). Inspect the
diff, stage intended paths, run the per-commit gates, and append observed results:

```powershell
python .agents/skills/compact-diary/scripts/log_diary.py `
  --title "<Commit-sized change>" `
  --subsystems "<relative components>" `
  --changes "<change 1>; <change 2>" `
  --rationale "<reason or trade-off>" `
  --results "<observed verification>"
```

Entries use `### [YYYY-MM-DD HH:MM CET/CEST] — <Title>`; use `--date` for an
explicit local timestamp. Record failures and unrun checks honestly.

Use an English Conventional Commit message, for example:

```text
docs(rules): separate policy from execution procedures

Explain the concrete change and rationale.
Verification: list checks actually run and their observed results.
```

Usual types are `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, and `chore`;
scope names identify the affected subsystem or tooling. Implementation, tests,
design updates, and diary for the same concern belong together.

For an explicit language scan, use `python tests/checks/check_polish.py <path>`
or `python tests/checks/check_polish.py --staged`. The milestone gate includes
the staged scan. Invoke validation explicitly; use no lifecycle or Git hooks.

---

## 5. Native Codex Agents (`.codex/agents/`)

Codex discovers standalone agent TOML files in `.codex/agents/`. Each defines `name`, `description`, and `developer_instructions`. All four project agents explicitly select `gpt-6.1-sol` and set `model_reasoning_effort`: `high` for `code_reviewer`, `cpu_verifier`, and `image_analyst`; `medium` for `doc_curator`. Tools and permissions inherit from the parent session unless a role configures an override; `image_analyst` sets `sandbox_mode = "workspace-write"` for assigned output assets. Delegate when the user or applicable project/skill instructions request it. These instructions define responsibilities, not executable tool allowlists. Verify effective model and effort in a fresh client session after changing these files before claiming runtime activation.

Subagents run in their own **isolated context windows** for assigned engineering
tasks. The parent invokes each agent with an explicit scope and receives a compact
verdict or diff. Scripted document conversion manages its own stage inputs, model
calls, and persisted outputs.

| Subagent | File | Context Profile | Primary Responsibility |
| :--- | :--- | :--- | :--- |
| **`cpu_verifier`** | [`cpu_verifier.toml`](../.codex/agents/cpu_verifier.toml) | 🔬 Precision / Low volume | Tom Harte single-step silicon test execution, cycle-exact ALU/CCR verification, and timing regression isolation. |
| **`code_reviewer`** | [`code_reviewer.toml`](../.codex/agents/code_reviewer.toml) | 🔍 Adversarial / Medium volume | 18-point pre-commit and architectural compliance audit against AGENTS.md rules, file size limits, and inlining policy. |
| **`image_analyst`** | [`image_analyst.toml`](../.codex/agents/image_analyst.toml) | Illustration / Scoped assets | Detailed descriptions of assigned illustrations, schematics, and diagrams; explicitly assigned lossless crop extraction and technical sidecars. |
| **`doc_curator`** | [`doc_curator.toml`](../.codex/agents/doc_curator.toml) | 📐 Structural / Low volume | Semantic parity between `Obsidian/Amiga/Design/` specs and Rust code, vault graph integrity, ROADMAP.md pruning, DIARY.md compaction. |

For separately assigned document illustrations, give `image_analyst` the source image, any crop region, and the requested description format. Assign output paths when requesting description files or crop assets. Crop coordinates are integer pixels on the original image, measured from the top left with exclusive upper bounds. The agent preserves the source and verifies saved crops; the parent integrates text, descriptions, and assets into the document. The parent handles GUI inspection through [egui-vision-debugger](../.agents/skills/egui-vision-debugger/SKILL.md).

### Documentation and Conversion Workflows

Use `doc_curator` for assigned design-specification maintenance. The parent authors
articles and devlogs through [author-methodology-doc](../.agents/skills/author-methodology-doc/SKILL.md).
PDF/HTML conversion runs through the [standalone bootstrap programs](developers.md#processing-raw-documents-into-markdown),
whose stages control model invocation, artifact handoffs, and conversion-time descriptions.
The PDF [config.yaml](../tools/bootstrap/pdf-to-markdown/config.yaml) currently selects
models by role (`vision`, `prose`, `table`, `fast`) and thinking budgets per stage;
it still uses Gemini. Migration to Codex Vision is planned in [ROADMAP.md](../ROADMAP.md).
Validated documentation is indexed through [index-amiga-rag](../.agents/skills/index-amiga-rag/SKILL.md).

## 6. Configuration and Explicit Validation

`AGENTS.md` routes universal and domain-specific rule reading. Rule files are referenced Markdown, not automatically activated by frontmatter triggers. Repository skills are discovered directly; no JSON catalog is required.

The Amiga RAG stdio adapter is declared in [`.codex/config.toml`](../.codex/config.toml). Project configuration requires a trusted project in Codex. MCP connectivity and Graphify runtime repairs are separate from format conversion.

Create Git commits only on explicit user request or when closing a completed minor roadmap point (for example, 1.1 or 1.2). Routine task completion leaves validated changes uncommitted and unstaged. Every commit includes its own DIARY.md Section 10 entry; at minor roadmap points or milestones, compact settled entries into milestone digests. Run validation commands explicitly, using `python tests/verify.py --commit` and, at stage closure, also `python tests/verify.py --milestone` once for the same snapshot. The separate milestone set contains only extra audits. vAmigaTS captures and chipset workloads are excluded from automatic verification. The commit set includes architecture and all standard tests, with change triggers only for DMA/SingleStep and independent `--verify-dma` / `--verify-singlestep` test sets, runnable alone or alongside a completion profile. Language checks support file paths or `python tests/checks/check_polish.py --staged` for staged additions. Do not install or use lifecycle, code, or Git hooks.

Format references: [Codex agents](https://learn.chatgpt.com/docs/agent-configuration/subagents), [project instructions](https://learn.chatgpt.com/docs/agent-configuration/agents-md), and [configuration](https://learn.chatgpt.com/docs/config-file/config-reference).
