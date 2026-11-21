# Working with an AI Agent

This guide is for developers starting to work on the Amiga 500 emulator with an
AI coding agent. It explains how to frame a task, how the agent finds relevant
code and hardware evidence, and how to judge the result. The main session routes
your request to the appropriate roles and explains the results. Saving agreed
behavior selects planning and applicable test design. An expressly scoped
end-to-end delivery selects the operations needed to reach that boundary.
For other work, choose the operation and its scope.

## Find your way around the skills

Follow the [workflow map](workflow-map.svg) from a saved plan to local
publication. Open the SVG on its own to follow its skill and procedure links.
The [workflow details](workflow-details.svg) cover capture and service recovery,
approvals, resuming work, review decisions and interrupted publication or
closure. The map's [skill directory](workflow-map.svg#directory) groups other
operations by purpose.

![Workflow from a saved plan through conditional local publication and closure](workflow-map.svg)

The main session confirms a fresh archived capture event before formal
planning for a new task or starting a saved plan. It starts the plan in its
own worktree and points to the authoritative copy. The implementer works there,
runs focused checks and records the completed source. [Capture startup](../tools/workflow/capture/README.md#confirm-telemetry-before-work), [plan execution](../.agents/references/workflow-guidance.md#start-or-deliver-a-saved-plan).

When you authorize local delivery, the main session identifies the selected
task branch and commit and records the local master tip. A clean, unchanged,
approved tested tip with committed closure can take the short path when it
contains that master tip. A divergent single source merges master in the
task checkout. Combined Sources merges the selected original tips in a
separate assembly checkout. Working commits or missing tested-tip requirements
take the full route. [Source selection](../.agents/rules/git-merge-commits.md#discover-and-select-deliveries).

On the full route, the implementer reconciles code and tests, and the main session
reconciles documentation and prepares the batch. The whole candidate needs
passing canonical verification and four independent reviews. Applicable
specification and vault evidence belongs in preparation. A required issue or
a missing gate follows the checkpoint and repair procedure before a proposed
commit can be accepted. [Batch policy](../.agents/rules/git-commits.md#readiness-before-freeze), [review checkpoint](../.agents/references/workflow-guidance.md#human-review-checkpoint).

All routes meet at the exact sealed commit. Pass one full English scan for that
commit in the task or assembly checkout before conditional publication to local
master. If master moves, recheck the source selection and repeat the affected
acceptance steps and scan. The surviving publication record supports graph
refresh, closure of the selected capture intervals, record preservation and
authorized cleanup. An authorized push can follow publication separately.
[Integration and closure](../tools/verification/README.md#local-integration).

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
> trace. Explain the failing phase before changing code, preserve the reference
> data, and run the relevant verification after the fix.

For a first session, an investigation is also useful:

> Explain how a blocked Chip RAM access reaches the CPU and which test verifies
> that the active micro-step does not advance. Show the relevant code and design
> references.

The first prompt requests a repair; the second requests investigation and an
explanation. That distinction determines whether the agent should edit files.
When scope or behavior is unclear, establish the intended model before asking
for implementation.

## Follow the Change from Evidence to Verification

When you ask to save agreed software behavior, planning and applicable test
design happen under the same request. The roles inspect requirements, existing
implementation and reusable evidence, then define a few meaningful scenarios
with independent expected results and link them to the saved revision.
The main session asks about unresolved behavior and specific protected edits
found during planning or design. Implementation follows the agreed scope.
For a defect, it reproduces the failure before changing the mechanism. Expect
it to explain the failing data flow, clock phase or state transition.

Hardware timing gives this project unusually concrete review criteria. A 68000
bus cycle spans two Color Clocks, blocked Chip RAM access must preserve the active
CPU micro-step, and peer chips coordinate through the machine loop and memory bus.
The [architecture overview](architecture.md) introduces these relationships;
[design specifications](../Obsidian/Amiga/Design/General%20Architecture.md) provide
the subsystem contracts. A conflict with a specification or protected reference
requires a decision from you before the agent changes that contract.

In an ordinary planning conversation, edit one working plan through successive
messages. At approval, keep the selected text recoverable so the agent can show
what changed and which decisions still apply. Formal planning and handoffs keep
their own results. During implementation, a material change to approved behavior
or acceptance is recorded beside the plan. The approved requirements stay in
force until the material change is decided. [Drafts and version checks](../.agents/references/workflow-contract.md#working-drafts-and-version-checks), [active plan amendments](../.agents/references/workflow-guidance.md#amend-an-active-plan).

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
[source provenance](../.agents/skills/TA-research-hardware-manuals/references/provenance.md) and
[developer guide](developers.md) for scope and setup.

**Graphify navigates source relationships.** Its `project` graph covers our code and allowed documents; its independent
`reference` graph covers imported source code. Compare results separately and
use them to locate symbols, callers and dependencies. For example:

```powershell
python tools/graphify/read.py query "How does the CPU handle a blocked bus access?" --graph graphify-out/graph.json
python tools/graphify/read.py path "Cpu" "BusResult" --graph graphify-out/graph.json
```

The graph helps locate source; the implementation and tests provide the behavior.
Refresh behavior and execution options belong to the [Graphify tool reference](../tools/graphify/README.md).
The caller selects [query](../.agents/skills/TA-query-graph/SKILL.md),
[path](../.agents/skills/TA-find-graph-path/SKILL.md) or
[symbol explanation](../.agents/skills/TA-explain-graph-symbol/SKILL.md).
These read operations do not refresh either graph. Structural refresh and project
semantic extraction are separate selected operations.

Reference emulators and imported manuals are provisioned separately. Their
conversion is an initial setup operation; everyday agent sessions consume the
prepared sources. The [developer guide](developers.md) explains that setup.

## Use Skills for Repeatable Procedures

A skill packages one operation with inputs, a procedure and a stopping point.
All repository skill and role names begin with `TA-`. Browse
[`.agents/skills/`](../.agents/skills/) or name the operation in your request.
The shared contract and relevant advance rules define what an author needs before
designing the change; detailed result criteria belong to the assigned review lane.
Those standards still apply when their full checklist is outside the author's
default reading.

For example, `TA-diagnose-cpu` reports a selected vector mismatch without repairing
it, while `TA-fix-findings` implements explicitly selected findings. GUI capture
returns an image and its path; interpretation and selected repairs remain separate
assignments. The relevant `SKILL.md` defines the operation's scope and steps.

You can also request focused work from the configured roles in
[`.codex/agents/`](../.codex/agents/):

| Role | Useful when you need |
| --- | --- |
| `TA-planner` | A bounded implementation plan, acceptance criteria and unresolved decisions. |
| `TA-test-designer` | Meaningful scenarios, independent expectations and observation methods. |
| `TA-implementer` | Agreed implementation and tests, or selected finding repairs. |
| `TA-test-auditor` | Test and assertion assessment, or a separately requested coverage measurement. |
| `TA-code-reviewer` | One independent review lane or assigned specification/documentation assessment. |
| `TA-hardware-researcher` | Evidence from the selected manuals, reference models or measured hardware sources. |
| `TA-cpu-verifier` | Diagnosis of state, clock and bus differences in selected 68000 vectors. |

Delegation happens when requested by you or required by an applicable procedure.
Each helper receives a scoped assignment and reports back to the main session.
The main session announces the role, skill, model and reasoning effort, including
where each setting comes from. For implementation, selected repairs and
optimization, an authorized write anywhere under `crates/` selects Astra high
for the whole assignment. The default for tool-only assignments is Sol high.
Read access to a crate does not change the pair. The behavior, architecture
and semantic parity reviewers use Sol high by default, including for crate
changes. For the two creation skills, the main session must apply both resolved
settings before work. Your explicit choice replaces the field you name.
[Runtime procedure](../.agents/references/workflow-runtime.md#resolve-one-invocation), [skill matrix](workflow-skill-matrix.md).

Prepared batches receive four independent review lanes covering behavior,
architecture, tests/evidence, and specifications/instructions. The
[review contract](../.agents/references/review-common.md) owns their isolation,
inputs and scoped approval rules. The main session dispatches four fresh instances
of `TA-code-reviewer`, each using its own fixed lane skill. When the lanes report
required findings, a fresh advisory reviewer upholds or rejects each collected
finding. Upheld findings go to repair selection. Rejected findings close on
the reviewed candidate. The main session retains the primary reports and applies
the [review checkpoint and finding decisions](../.agents/references/workflow-guidance.md#challenge-required-findings).

Each formal invocation announces its scope and round limit, normally three and never
more than four. It returns both `result.json` and `result.md` under
`.agent/tmp/workflow-TA/<task-id>/<work-item-id>/<run-id>/`. The
[invocation contract](../.agents/references/workflow-contract.md) records candidate
and instruction identities, findings, actual checks, retained evidence and limits.
Use the [result tool](../tools/workflow/README.md) to inspect the selected skill's
schema, validate JSON and render matching text. A missing validator dependency is
a prerequisite gap; installation requires its own authorization.
A completed review may request changes. Missing evidence or an exhausted budget
returns control instead of starting repairs; a linked continuation needs actual
user authorization or a recorded delegation. Markdown budgets and write boundaries
are behavioral limits, not claims of hard runtime enforcement.

## Review and Record the Result

After an operation returns, the main session explains what happened, what it means
for the task and the most useful next step. For a review finding, expect the
explanation to follow an ordinary use of the system and identify the step where
behavior or required evidence fell short. When a choice is needed, the session
offers one to three relevant options with their effects, prerequisites and
copyable instructions naming the executor, skill and exact scope. Select an option by its
label or number in the conversation, paste its instruction, defer work or give
a different request. You do not need to read raw JSON or assemble a specialist
assignment. Completed work may need no next operation.

For example, when a focused test demonstrates a code defect, expect a repair
option naming `TA-implementer` and `TA-fix-findings`, the actual finding ID and
input references, affected files and round limit. The displayed instruction uses
those real inputs. If the cause is unknown, an investigation may be more useful;
a test audit is appropriate when the expectation itself is unresolved. A required
failure remains visible if you defer it. Permission to repair does not approve
weakening a test, and inability to repair is not evidence that the test is wrong.

The session keeps the worker's original report. It saves linked guidance when
you can select an operation, a decision needs to identify the displayed version,
or a pending handoff must survive resumption. Only the current option set is
selectable. Before relying on a delayed selection, the session checks the live
inputs and refreshes choices if the change affects their prerequisites. When a
formal plan finishes with no pending decision, the session replies directly and
keeps the formal result pair. [Receiving procedure](../.agents/references/workflow-guidance.md).

The [skill matrix](workflow-skill-matrix.md) defines the standard recommendation
paths. Its source row is the skill that returned; each destination column is a
possible next skill. Cell codes state conditions, such as a ready plan, missing
test evidence or identified findings. The agent checks those conditions and offers
the useful options, rather than treating every marked cell as a required step.
An empty cell means the agent does not recommend that transition in the standard
workflow. You can still explicitly request another skill, subject to its normal
scope, authorization and prerequisites. The matrix is guidance, not a permission
system. After your directly selected operation, suggestions resume from its row.

Starting a saved plan uses its own worktree and branch. The agent links the
authoritative working plan so you can open it directly. Resuming returns to those
copies. For other implementation work, the agent establishes the location if you
have not supplied one. The
[location rule](../AGENTS.md#choose-the-task-checkout)
defines this boundary.

Routine task completion leaves validated changes available for review. It does
not itself authorize staging or committing. Ask for a commit when the change is
ready, or request batch preparation to authorize staging without creating commits.
A commit request authorizes preparation and mechanical execution without another
confirmation. The agent identifies the included sessions and changes and splits
independent concerns into coherent commits automatically, reporting their count,
IDs and scopes. Roadmap closure retains its required completion commit under the
[commit policy](../.agents/rules/git-commits.md).

Prepare one complete candidate against `HEAD`, then repeat the command after corrections:

```powershell
python tools/verification/verify.py commit
```

Despite its name, this command verifies the working tree; it does not create a
commit. The [testing guide](testing.md) explains what its evidence establishes.
[Commit policy](../.agents/rules/git-commits.md) owns preparation, diary entries,
review, evidence continuation and freeze. Failed or missing required checks
prevent batch readiness, and results cover the combined candidate.

Local publication is a separate operation. `TA-close-task` consumes a sealed
commit with applicable acceptance evidence and a passing full English scan.
It conditionally advances the recorded local target or reports a stale/missing
prerequisite. It does not merge master, repair conflicts, run tests, create commits
or remove the worktree. `TA-cleanup-task` handles separately authorized cleanup
after confirmed publication and committed closure records.

Earlier steps have separate owners: `TA-sync-master`,
`TA-resolve-merge-conflicts`, `TA-sync-docs`, `TA-prepare-batch`,
`TA-verify-batch` and `TA-commit-batch`. The main session runs the applicable
operations in an expressly scoped end-to-end delivery, subject to required
checks, review decisions and approved boundaries.
Commit-only requests do not authorize merging, pushing or worktree removal.
The [integration rule](../.agents/rules/git-merge-commits.md) and
[tool reference](../tools/verification/README.md#local-integration) preserve exact
source/target selection, the narrow confirmed-tested exception, conditional
publication and safe cleanup. Several sources still require one assembled and
verified combined candidate; independently publishing tested branches is insufficient.

If you are changing the agent infrastructure itself, use
[Maintaining Agent Tooling](../.agents/references/agent-tooling.md) for script ownership, configuration
and recording procedures.
