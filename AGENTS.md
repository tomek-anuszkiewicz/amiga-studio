# Amiga 500 Emulator: Agent Instructions

## Working Agreements

Read these shared rules before task work; linked files require explicit reads:
- [Scoped changes and root-cause resolution](.agents/rules/structural-root-cause.md): Stay within the requested task, reuse definitions, and fix mechanisms.
- [Specification compliance](.agents/rules/spec-compliance.md): Surface conflicts with design specifications and never silently change golden references.
- [Path privacy](.agents/rules/no-external-paths.md): Keep committed paths portable.
- [Atomic commits](.agents/rules/git-commits.md): Validate and commit each discrete task with a corresponding diary entry.

Answer in the user's language; repository content and commits are English. Read [language-policy.md](.agents/rules/language-policy.md) for language checks, [audio-transcription.md](.agents/rules/audio-transcription.md) when audio is supplied, and [model-reasoning-advisory.md](.agents/rules/model-reasoning-advisory.md) when reasoning-effort advice is relevant. Do not guess unavailable model metadata.

Run validation commands explicitly. Do not install, configure, or use lifecycle, code, or Git hooks. Use `.agent/tasks/` for English execution plans and `.agent/tmp/` for disposable diagnostics and artifacts. Follow the [execution-plan lifecycle](.agents/rules/structural-root-cause.md#execution-plan-lifecycle) when closing tasks. Treat existing temporary artifacts as unverified until their assumptions are checked.

First identify the question and whether existing evidence resolves it. Use scripts for concrete measurement, verification, or data processing; do not repair a tool unless that repair is needed for the task.

## Task-Specific Rules and Navigation

Read applicable domain rules before editing that domain. Use [Graphify](.agents/skills/graphify/SKILL.md) for source navigation and [amiga-rag.md](.agents/rules/amiga-rag.md) for domain/reference retrieval.

| Task | Rules and procedures |
| --- | --- |
| Rust implementation | [Rust practices](.agents/rules/rust-best-practices.md), [performance](.agents/rules/performance-and-readability.md), [inlining](.agents/rules/method-inlining.md), [file cohesion](.agents/rules/file-size-and-cohesion.md), [workspace boundaries](.agents/rules/workspace-structure-and-reexports.md), [clean refactoring](.agents/rules/clean-break-refactoring.md) |
| CPU instructions | [Opcode conventions and required implementation procedure](.agents/rules/opcode-naming.md) |
| Chip, DMA, or bus behavior | [Hardware topology](.agents/rules/hardware-bus-topology.md), [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) |
| Tests or defect repair | [Testing policy](.agents/rules/unit-testing-policy.md), [repro-first](.agents/rules/repro-first.md) |
| GUI | [egui practices and visual verification](.agents/rules/egui-best-practices.md) |
| Documentation | [Documentation maintenance](.agents/rules/docs-maintenance.md), [vault links](.agents/rules/vault-linking-and-graph-integrity.md), [information hierarchy](.agents/rules/information-hierarchy.md), [writing style](.agents/rules/practitioner-voice-and-tone.md) |
| Commit or milestone | [Diary](.agents/rules/diary-maintenance.md), [roadmap](.agents/rules/roadmap-maintenance.md) |
| Merge or worktree | [Merge policy and required procedures](.agents/rules/git-merge-commits.md) |
| Reference conversion bootstrap | [Conversion programs](docs/developers.md#processing-raw-documents-into-markdown), [developer-led workflow](tools/bootstrap/reference-conversion-contract.md#development-workflow), [converter testing scope](.agents/rules/unit-testing-policy.md#bootstrap-converter-scope) |
| Long-running checks | [Parallel execution](.agents/rules/parallel-execution.md) |

Codex discovers skill names and descriptions in `.agents/skills/` and reads full instructions when selected; a separate skill catalog is unnecessary here. Rule files are referenced Markdown and must be read as routed above. Roles live in `.codex/agents/*.toml`; delegate only when requested by the user or applicable instructions. MCP configuration is `.codex/config.toml`.

## Machine Invariants

- Native desktop and `wasm32-unknown-unknown` remain supported. Core emulation consumes external buffers; it does not perform host filesystem, thread, or wall-clock I/O.
- The synchronization unit is the Color Clock (CCK), approximately 3.54 MHz PAL / 3.58 MHz NTSC. A 68000 bus cycle takes four CPU clocks, or two CCK cycles (CCK1 and CCK2). A blocked Chip RAM access returns `BusResult::WaitState`; the CPU waits without advancing its active micro-step.
- The top-level machine owns CPU, memory bus, Agnus, Denise, Paula, and both CIAs, tracking master CCKs with `cck: u64`. Peer chips hold no direct references and do not call one another. The machine loop and memory bus route coordination and interrupt arbitration.
- Agnus exclusively generates Chip RAM DMA addresses and drives RGA. Denise and Paula latch bus data without direct memory reads. Signal and register effects follow modeled physical phases and propagation delays.
- State snapshots remain queryable and serializable with `serde::Serialize` and `serde::Deserialize`, separate from runtime handles.
- Follow physical dependencies when planning: bus arbitration and clocks, autonomous DMA, display, peripherals, then firmware integration.
- Guest memory is big-endian. Use explicit endian conversion, never host-endian pointer casts or `transmute`. Bitwise operations and raw transfers may bypass swaps only when their semantics commute with byte reversal.
- Guest execution must not panic the host. Unmapped reads return `$FF` / `$FFFF` (configurable); odd-address word/long accesses raise Address Error, Vector 3. Runtime emulation uses no `unwrap` or `expect`.
- ALU operations and cycle counters use explicit wrapping arithmetic. Execution, memory access, and interrupt hot paths allocate no heap memory. Project rules prohibit custom `macro_rules!` and const-generic instruction handlers.

## Verification and Completion

Every discrete task is validated and committed atomically in English. **Every commit includes a new DIARY.md Section 10 entry.** At minor roadmap points or milestones, compact settled entries and synchronize affected design specifications and the remaining roadmap.

Per-commit commands:
```powershell
python tools/harness/pre_flight.py --quick
cargo test -p test_runner --test test_architecture_rules -- --quiet
```
The architecture suite is separate from the quick gate. Run relevant domain tests as well. CPU changes require full SingleStepTests (`SINGLESTEP_FULL=1`); CPU/bus changes require `test_dma_cartesian`. Use targeted filters during development, then run the required full domain verification before completion.

Milestone completion requires `python tools/harness/pre_flight.py --milestone`, [semantic parity review](.agents/skills/audit-semantic-parity/SKILL.md) of modified subsystems, design synchronization and checkpoint updates, diary compaction, and pruning completed roadmap steps. Do not claim unrun checks passed. **Prohibition of Blind Golden Hash Modifications:** Never silently change golden hashes or cycle references; follow the conflict and approval procedure in specification-compliance rules.

## Authoritative References

- [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) routes to subsystem specifications under `Obsidian/Amiga/Design/`.
- Hardware manuals and PRMs live under `Obsidian/Amiga/Reference/`; use RAG before scanning large manuals.
- `rag_qdrant` is the canonical CLI for the `projects_docs` collection, `amiga` source; the optional adapter is [tools/amiga-rag-mcp-server](tools/amiga-rag-mcp-server).
- Reference emulators and suites live under `ref_src/`; 68000 silicon vectors are under `ref_src/SingleStepTests-680x0/68000/v1`.
