# Getting Oriented as a Developer

This guide gives you the project map: how to run the emulator, where its main
parts live, and where to start investigating a change. Detailed setup procedures,
command options and subsystem contracts are linked from the relevant sections.
For an overview of the documentation, start with [the reading guide](README.md).

## Build and Run

The native emulator is a Rust workspace. A normal build does not require the
external test collections, hardware manuals or a running knowledge database.
From the repository root:

```powershell
cargo run --release -p gui
```

The GUI provides the emulator display and Developer Studio. See the
[debugger guide](debugger.md) for execution controls and inspection tools.
A browser frontend is also supported through WebAssembly and Trunk. Installation
and launch commands are in the
[environment setup reference](../.agents/references/developer-setup.md#1-build-run).

## Find the Part You Want to Understand

The [architecture overview](architecture.md) explains the clock model and how the
CPU, memory bus and custom chips work together. The source is organized into a
Rust workspace under [crates/](../crates/):

| Area | Starting point |
| --- | --- |
| 68000 instruction execution | [cpu](../crates/cpu/) |
| Memory mapping and access | [memory_bus](../crates/memory_bus/) |
| Machine scheduling and chip coordination | [machine_loop](../crates/machine_loop/) |
| Custom chips and peripherals | Their individual crates and the [design index](../Obsidian/Amiga/Design/General%20Architecture.md) |
| Execution inspection and frontend | [debugger](../crates/debugger/) and [gui](../crates/gui/) |
| External CPU vectors and diagnostic workloads | [test_runner](../crates/test_runner/) |

Follow a concrete behavior through these boundaries. For example, a CPU memory
access involves the CPU's instruction state, address decoding and bus availability;
a display issue also involves DMA scheduling and the video chip. A local symptom
can originate in a different component.

## Use Specifications and References

Project specifications under
[Obsidian/Amiga/Design](../Obsidian/Amiga/Design/General%20Architecture.md) describe
subsystem responsibilities and timing contracts. Imported hardware manuals live
under `Obsidian/Amiga/Reference/`, while reference emulator sources and silicon
vectors live under `ref_src/`. Large external collections are provisioned
separately and may be absent in a fresh checkout.

These materials serve different purposes. A project specification states the
intended model; a hardware manual or silicon trace provides evidence for it;
a reference emulator offers another implementation to inspect. A disagreement
needs investigation before changing code or expected results.

<a id="processing-raw-documents-into-markdown"></a>
Reference conversion prepares searchable Markdown and assets from raw PDF and
HTML sources. It belongs to initial knowledge-base setup. See the
[setup reference](../.agents/references/developer-setup.md#processing-raw-documents-into-markdown)
and the [conversion contract](../tools/bootstrap/reference-conversion-contract.md)
when working on those tools.

## Navigate with RAG and Graphify

RAG searches hardware and project documentation; Graphify follows source symbols,
call relationships and dependencies. Both help an agent locate focused context
for a question such as which phase commits a write or which tests exercise a
blocked bus access. Their answers still need to be checked against the relevant
source and evidence.

For setup and commands, use the
[RAG reference](../.agents/references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
and [Graphify skill](../.agents/skills/graphify/SKILL.md). These services are useful
for investigation, but they are not prerequisites for running the emulator.

## Make and Evaluate a Change

Start with a specific behavior, reproduce it, and establish which component owns
it. The [testing overview](testing.md) explains the different kinds of evidence
available and their limits. For ordinary completion verification, run:

```powershell
python tests/verify.py --commit
```

This checks the working tree and reports results; it does not create a commit.
Use [tests/README.md](../tests/README.md) for operational details and focused test
commands. If you work through an agent, [Working with an AI Agent](ai_agents.md)
explains how to frame the task, review the result and request a commit.
