# Amiga 500 Emulator: Agent Instructions

## Shared Contract

Work within the requested outcome and necessary technical changes. Inspection
requests authorize investigation; implementation requests authorize scoped edits.
Propose optional behavior before adding it. Preserve unrelated files, staged
versions, user data and concurrent work. This is a private, pre-release project:
prefer the simplest correct current implementation and complete internal migrations.

Read affected requirements before design or editing; nearby code supplies examples,
not authority over specifications or hardware evidence. Applicable result standards
remain binding even when their detailed reviewer checklist is not loaded by the
author. In-scope corrections complete the assignment; findings grant no unrelated
write authority. Handle specification conflicts immediately under
[reference integrity and requirements](.agents/rules/spec-compliance.md).

Before new implementation, identify the checkout and branch and establish the
[task checkout choice](.agents/rules/git-merge-commits.md#choose-the-task-checkout).
Reuse an explicit location choice and existing authorization. Ordinary changes stay
unstaged; batch preparation authorizes only selected staging, and commits,
publication, cleanup and push require their corresponding authorization.
Architectural checkers/policy data, weakened gates and protected references retain
their [explicit approval boundaries](.agents/rules/unit-testing-policy.md#protected-verification).

Use `.agent/tasks/` for active plans and `.agent/tmp/` for disposable diagnostics.
Do not trust old artifacts without checking their inputs. Run commands explicitly;
do not install, configure or use lifecycle, code or Git hooks. Prefer existing tools
and direct commands; create a helper only for a concrete need they cannot meet.

Keep committed paths portable: repository-relative paths, environment variables or
CLI inputs. Never hardcode private host paths or disclose them unnecessarily.
Already authorized local inputs need no second approval; missing configuration is
an input gap, not permission to invent it.

Answer in the user's language. Keep repository artifacts in English; `.agent/tasks/`
is exempt from language and vocabulary checks. State outcomes and actual evidence
plainly, distinguishing assumptions, hypotheses and unrun work. Detailed language
handling activates for [findings or an explicit audit](.agents/rules/language-policy.md).

Before starting or resuming mouse/keyboard control, announce it. Stop input when
the user takes over. On finishing, including recoverable errors or interruption,
perform the tool's documented completion/release step before unrelated work. Then
explicitly tell the user in their language that control ended and they can work
normally. Do not claim release unless confirmed.

## One Selected Operation

Select the role and atomic `TA-` skill from the actual request. Before dispatch,
apply the [runtime settings](.agents/references/workflow-runtime.md#model-and-reasoning-effort)
and announce the assignment under the [invocation and dual-result contract](.agents/references/workflow-contract.md).
Each invocation returns matching text and JSON, with an exact input identity and
one to four declared rounds (three by default). A missing prerequisite, finding or
limit returns control; it does not start another skill, repair or lifecycle phase.
The receiving main session automatically explains the actual result and presents
applicable options through [next-step guidance](.agents/references/workflow-guidance.md).
Official suggestions use its conditional skill matrix. The user may directly choose
another skill; an empty matrix cell is not a prohibition or a new permission gate.
Each executable choice names its role, skill, exact scope and copyable invocation.
Wait for the user's selection, recheck applicability, then run only that choice.
Interpretation and recommendations do not authorize or dispatch another operation.

Repository roles live in `.codex/agents/`; runtime roles and installed plugin skills
are separate. Use fresh contexts for independent reviews. Do not claim filesystem,
context or budget enforcement beyond the runtime controls actually observed.

For a new repository skill, use [TA-create-skill](.agents/skills/TA-create-skill/SKILL.md).
For a new repository role, use [TA-create-agent](.agents/skills/TA-create-agent/SKILL.md).
Select each operation separately when the request needs both definitions.

## Task-Specific Rules and Navigation

Load only the relevant advance requirements and selected operation. Reviewers also
load their assigned lane criteria; authors do not read all reviewer checklists.
Read-only questions need only the material needed to answer them.

| Work | Read at its trigger |
| --- | --- |
| Rust implementation | [Rust constraints](.agents/rules/rust-best-practices.md); affected specifications through [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) |
| Execution or performance work | [Execution semantics](.agents/rules/performance-and-readability.md) |
| Crates, exports or CPU module layout | [Workspace boundaries](.agents/rules/workspace-structure-and-reexports.md) |
| Chip, DMA, bus, interrupts or peripherals | [Hardware topology](.agents/rules/hardware-bus-topology.md) |
| Test design, execution or assessment | [Testing policy](.agents/rules/unit-testing-policy.md) and the selected skill's criteria |
| Defect investigation or repair | [Original reproduction and causal repair](.agents/references/defect-resolution.md) before mutation |
| GUI | [GUI advance requirements](.agents/rules/egui-best-practices.md) |
| Documentation or specifications | [Documentation value](.agents/rules/docs-maintenance.md) |
| Rules, skills, roles or instruction ownership | [Information hierarchy](.agents/rules/information-hierarchy.md) |
| Preparation, commits or closure | [Readiness policy](.agents/rules/git-commits.md); [diary](.agents/rules/diary-maintenance.md) and [roadmap](.agents/rules/roadmap-maintenance.md) at their triggers |
| Synchronization, publication or cleanup | The relevant part of [integration policy](.agents/rules/git-merge-commits.md) and the separately selected skill |
| PowerShell changes | [TA-write-powershell](.agents/skills/TA-write-powershell/SKILL.md) |
| Reference conversion | [Nested bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable program README |

For cross-file code navigation use the selected Graphify read operation:
[query](.agents/skills/TA-query-graph/SKILL.md),
[path](.agents/skills/TA-find-graph-path/SKILL.md) or
[symbol explanation](.agents/skills/TA-explain-graph-symbol/SKILL.md).
Choose project or reference data explicitly, read source definitions, and use `rg`
when the graph is missing, stale or insufficient. Hardware/manual retrieval follows
[domain provenance](.agents/rules/amiga-rag.md). Manuals live under
`Obsidian/Amiga/Reference/`; imported models and vectors under `ref_src/`.

## Verification and Completion

Run relevant focused checks during development. Canonical batch verification is
`python tools/verification/verify.py commit`; milestone verification is
`python tools/verification/verify.py milestone` at an actual closure trigger.
The [verifier](tools/verification/README.md) owns selection, required gates and
evidence reuse. A failed or missing required gate blocks readiness. Assessment
results do not replace executed verification.

Required batch review uses four fresh independent `TA-code-reviewer` instances,
one for each fixed lane. Schedule within available slots; an author or earlier
audit cannot replace a lane. Preparation, checks, reviews, commits, publication and
cleanup remain separately selected operations; broad existing authorization may
cover their scope without repeated permission requests, while guidance still waits
for selection. Before reporting completion,
review the diff against the requested scope and state checks and remaining limits.
