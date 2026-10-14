# Working with an AI Agent

This guide is for developers starting to work on the Amiga 500 emulator with an
AI coding agent. It explains how to frame a task, how the agent finds relevant
code and hardware evidence, and how to judge the result. The working pattern is
simple: you define the desired behavior and resolve design decisions; the agent
investigates, implements and verifies a scoped change that you can review.

For build and environment setup, use the [developer guide](developers.md).
For the project's development story and lessons learned, read
[How This Emulator Was Written](how_this_emulator_was_written.md).

## Start with a Concrete Task

Open an agent session in the repository root. [AGENTS.md](../AGENTS.md) is the
entry point for project instructions: it routes the agent to the rules and
procedures relevant to the task. You do not need to repeat the whole rule set in
each prompt.

Describe the behavior you want, the evidence you already have, and the boundaries
of the work. A failing test, a trace, a screenshot or a specific source location
is a useful starting point. State whether you want an explanation, a proposed
design, an implementation or an independent review.

For example:

> Investigate why this instruction takes two extra Color Clocks in the attached
> trace. Explain the failing phase before changing code, preserve the silicon
> reference, and run the relevant verification after the fix.

For a first session, an investigation is also useful:

> Explain how a blocked Chip RAM access reaches the CPU and which test verifies
> that the active micro-step does not advance. Show the relevant code and design
> references.

The first prompt requests a repair; the second requests investigation and an
explanation. That distinction determines whether the agent should edit files.
When scope or behavior is unclear, establish the intended model before asking
for implementation.

## Follow the Change from Evidence to Verification

The agent first inspects the existing implementation, applicable design
specifications and available test evidence. For a defect, it reproduces the
failure before changing the mechanism. Expect it to explain the failing data
flow, clock phase or state transition, rather than adjusting values until a test
passes.

Hardware timing gives this project unusually concrete review criteria. A 68000
bus cycle spans two Color Clocks, blocked Chip RAM access must preserve the active
CPU micro-step, and peer chips coordinate through the machine loop and memory bus.
The [architecture overview](architecture.md) introduces these relationships;
[design specifications](../Obsidian/Amiga/Design/General%20Architecture.md) provide
the subsystem contracts. A conflict with a specification or protected reference
requires a decision from you before the agent changes that contract.

During implementation, keep the conversation focused on the current uncertainty:
what has been established, what remains unexplained, and what observation will
resolve it. You can correct assumptions or narrow the task while the agent works.
For extended tasks, execution plans live in `.agent/tasks/`; disposable captures
and diagnostics live in `.agent/tmp/`.

At completion, expect a reviewable diff and a report of the actual checks run,
their results, and any remaining limitations. Read the diff as well as the report.
Passing tests establish the behavior they exercise; they do not establish every
hardware property or prove that the chosen design is appropriate.

## How the Agent Finds Its Bearings

Two tools help the agent retrieve focused context instead of reading the entire
repository or a hardware manual into each session.

**RAG retrieves domain evidence.** The `rag_qdrant` CLI and optional Amiga RAG MCP
adapter search project specifications and imported Commodore/Motorola references.
The `amiga` source contains project and hardware material; `devnotes` is a separate
source of engineering guidance. Retrieved passages are evidence with provenance,
and the agent checks relevant source sections or diagrams when needed. See the
[retrieval rule](../.agents/rules/amiga-rag.md) and
[developer guide](developers.md) for scope and setup.

**Graphify navigates source relationships.** Its graph connects the active Rust
workspace with external reference implementations, helping locate symbols,
callers and dependencies. For example:

```powershell
graphify query "How does the CPU handle a blocked bus access?"
graphify path "Cpu" "BusResult"
```

The graph helps locate source; the implementation and tests provide the behavior.
The agent refreshes the code graph after code changes using `graphify update .`.
Setup and traversal details are in the
[Graphify skill](../.agents/skills/graphify/SKILL.md).

Reference emulators and imported manuals are provisioned separately. Their
conversion is an initial setup operation; everyday agent sessions consume the
prepared sources. The [developer guide](developers.md) explains that setup.

## Use Skills for Repeatable Procedures

A skill packages a procedure: when to use it, which steps to execute, and what
evidence to report. Project rules define obligations; skills explain how to carry
out specialized work. Browse [`.agents/skills/`](../.agents/skills/) when you need
a procedure, or name one in your request, such as `$test-runner`.

For example, CPU instruction work has a dedicated implementation procedure and
silicon verification suite; GUI layout work uses captured frames for visual
inspection. These procedures give the agent a repeatable way to test its work.
The relevant `SKILL.md` is the source of truth for its scope and steps.

You can also request focused work from the configured roles in
[`.codex/agents/`](../.codex/agents/):

| Role | Useful when you need |
| --- | --- |
| `code_reviewer` | An independent review of code, architecture and rule compliance. |
| `cpu_verifier` | Diagnosis of cycle differences against physical 68000 silicon vectors. |
| `doc_curator` | Maintenance of design specifications and their links to implementation. |

Delegation happens when requested by you or required by an applicable procedure.
Each helper receives a scoped assignment and reports back to the main session.
The role configuration contains its current model and permissions.

## Review and Record the Result

Routine task completion leaves validated changes available for review. It does
not itself authorize staging or committing. Ask for a commit when the change is
ready; closing a completed minor roadmap point also authorizes its completion
commit under the [commit policy](../.agents/rules/git-commits.md).

The standard completion command is:

```powershell
python tools/verification/verify.py commit
```

Despite its name, this command verifies the current working tree; it does not
create a commit. The [testing guide](testing.md) explains its checks, focused
diagnostics and additional milestone requirements. Each authorized commit has a
corresponding `DIARY.md` entry recording the change and observed validation.

If you are changing the agent infrastructure itself, use
[Maintaining Agent Tooling](../.agents/references/agent-tooling.md) for script ownership, configuration
and recording procedures.
