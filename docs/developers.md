# Getting Oriented as a Developer

Start here to run the emulator, find its main components or investigate a change.
Each section points to detailed setup procedures, command options or subsystem
requirements. For the wider documentation, see the [reading guide](README.md).

## Build and Run

The native emulator builds from a Rust workspace. You can build and launch it
without external test collections, hardware manuals or a running knowledge
database. Run the following command from the repository root.

```powershell
cargo run --release -p gui
```

The GUI provides the emulator display and Developer Studio. See the
[debugger guide](debugger.md) for execution controls and inspection tools.
A browser frontend uses WebAssembly and Trunk. Find its installation and
launch commands in the
[environment setup reference](../.agents/references/developer-setup.md#1-build-run).

## Find the Part You Want to Understand

The architecture overview explains the clock model and how the CPU, memory bus
and custom chips work together. [Architecture overview](architecture.md)

The table shows where to start in the Rust workspace under [crates/](../crates/).

| Area | Starting point |
| --- | --- |
| 68000 instruction execution | [cpu](../crates/cpu/) |
| Memory mapping and access | [memory_bus](../crates/memory_bus/) |
| Machine scheduling and chip coordination | [machine_loop](../crates/machine_loop/) |
| Custom chips and peripherals | Their individual crates and the [design index](../Obsidian/Amiga/Design/General%20Architecture.md) |
| Execution inspection and frontend | [debugger](../crates/debugger/) and [gui](../crates/gui/) |
| External CPU vectors and diagnostic workloads | [test_runner](../crates/test_runner/) |

Trace the behavior through the components involved. For a CPU memory access,
start with the CPU's instruction state, then follow address decoding and check
whether the bus is available. For a display issue, also inspect DMA scheduling
and the video chip. The cause can be in a different component from the one
where you see the symptom.

## Use Specifications and References

Project specifications under
[Obsidian/Amiga/Design](../Obsidian/Amiga/Design/General%20Architecture.md) describe
subsystem responsibilities and timing contracts. Imported hardware manuals live
under `Obsidian/Amiga/Reference/`, while reference emulator sources and generated
instruction vectors live under `ref_src/`. Large external collections are provisioned
separately and may be absent in a fresh checkout.

A project specification defines the intended model. Use hardware manuals and
silicon traces as evidence for the model. A reference emulator gives you another
implementation to inspect. If the sources disagree, investigate before changing
code or expected results.

<a id="processing-raw-documents-into-markdown"></a>
Reference conversion prepares searchable Markdown and assets from raw PDF and
HTML sources. It belongs to initial knowledge-base setup. See the
[bootstrap instructions](../tools/bootstrap/AGENTS.md), the
[PDF converter](../tools/bootstrap/pdf-to-markdown/README.md), the
[HTML converter](../tools/bootstrap/html-to-markdown/README.md), and the
[book review workflow](../tools/bootstrap/book-to-markdown/README.md).
The [documentation bootstrap sequence](../tools/bootstrap/book-to-markdown/README.md#documentation-bootstrap-from-an-accepted-bookjson)
starts from an accepted `book.json` and matching PDF, then reviews, publishes,
and indexes the Markdown. Preparing and committing recipes for all four manuals
remains in the [roadmap](../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion).

## Navigate with RAG and Graphify

RAG searches hardware and project documentation. Graphify follows symbols, calls
and dependencies in source code. In Graphify, select the `project` graph for our
code and allowed documents or `reference` for imported source code. Bootstrap
creates or refreshes both graphs. When creating a worktree, the tool copies the
existing graph directories from the primary checkout. Either RAG or Graphify can
help you find where a write takes effect or which tests attempt a blocked bus
access. Check the answers against the relevant source and evidence.

For setup and commands, use the
[RAG reference](../.agents/references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
and [graph query skill](../.agents/skills/TA-Graphify-query/SKILL.md). These services are useful
for investigation, but they are not prerequisites for running the emulator.

## Make and Evaluate a Change

Start with a specific behavior. Reproduce it and identify the component that owns
it. The testing overview explains the available evidence and its limits.
[Testing overview](testing.md)

Run this command for ordinary completion verification.

```powershell
python tools/verification/verify.py commit
```

The command checks the working tree and reports results. Commit creation is a
separate action. For the procedure and focused test commands, see
[tools/verification/README.md](../tools/verification/README.md).

If you work through an agent, see how to frame the task, review the result and
request a commit. [Working with an AI Agent](ai_agents.md)
