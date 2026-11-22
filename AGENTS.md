# Amiga 500 Emulator: Agent Instructions

## Shared Contract

Work within the outcome the user requested and make the technical changes it
requires. Investigate when the user asks for inspection. When they ask for
implementation, edit within the agreed scope. Propose optional behavior before
adding it. Preserve unrelated files, staged versions, user data and other work
in progress. This is a private project before release. Choose the simplest
implementation that meets current requirements and carry internal migrations
through to completion.

Read the relevant requirements before designing or editing. Use nearby code for
examples. Base decisions about behavior on the specifications and hardware
evidence. The standards for your work apply even if you have not opened the
detailed review checklists. Fix issues within your assignment. Keep corrections
prompted by findings within the authorized scope.

Leave ordinary changes unstaged. When the user asks you to prepare a batch,
stage only the selected changes. Commit, publish, clean up or push only with
authorization for that action. If you report or commit incomplete work, explain
what remains blocked. State whether the commit works and whether verification
establishes readiness. Follow the approval rules for architecture checkers
and their policy data, weakening verification gates, and protected references.
[Explicit approval boundaries](.agents/references/protected-verification.md#protected-verification)

Store active, paused and blocked plans in `.agent/tasks/`. Put temporary
diagnostics in `.agent/tmp/`. Keep the task ID when you copy a plan or resume a
task. When resuming, verify the versions of the approved inputs and record where
the earlier work ended and where you are picking it up.

Check the inputs to an older artifact before you rely on it. Run commands
explicitly. Do not install, configure or use lifecycle, code or Git hooks.
Prefer existing tools and direct commands. Write a helper only to meet a
specific need those tools cannot handle.

Keep committed paths portable by using repository-relative paths, environment
variables or command-line inputs. Keep private host paths out of hardcoded
values and disclose them only as needed. Reuse local inputs the user has
already authorized. Report missing configuration as a missing input and use a
value only when the user or an authorized source provides it.

### Specification authority

Approved Design specifications govern behavior, timing, memory semantics and
architecture. If a requirement, expected result or reference model conflicts
with a specification, stop the conflicting change. Present the requirement,
the evidence of the conflict and the concrete options. Obtain and record the
user's decision before changing or waiving an approved requirement or adding
behavior beyond the agreed scope. This requirement applies even if a review
finding calls for the change or the code already agrees with it.

### Language

Reply in the user's language. Write repository artifacts, including saved plans
and agent result files, in English regardless of the conversation language.
Express the user's concepts in idiomatic English in those files. State the
outcome and the evidence plainly. Identify assumptions, hypotheses and work
you have yet to run.

Apply vocabulary-only maintenance in ordinary work and in selected workflows.
You may add justified words or literal exceptions for exact files without
a separate approval, scan, review or verification loop.
[Vocabulary-only maintenance](.agents/references/workflow-contract.md#vocabulary-only-maintenance)

### Communication

Use these rules for answers, progress updates, plans, test results, review
reports, documentation, code comments, new names in code and other explanations,
including summaries of work by other agents. Write so the user can understand
and maintain the project. The user commissioned the system and knows its goals
and overall workflow. Explain relevant code, scripts and internal names so the
user can follow without opening source files or piecing together past messages.

Give the direct answer first. When reporting an action, say what you did, why
and what happened. Choose familiar, concrete words. Use technical terms where
they make the text more precise, and explain them on first use. Before coining
a name, check whether a familiar term already means the same thing. Use the
official names of skills, agent roles, programs, services and components. When
one first matters to the explanation, say what kind of thing it is, what it
does, what data it uses and what it produces.

To explain behavior, trace a typical user action through the components it
involves. Say who starts it, what each component does and passes on, and what
result the user receives. Include what state a component reads or writes when
that helps explain the result. When describing a defect, point out the step
that differs from what should happen and say what the user can observe as a
result. Put the explanation before function names, source locations and other
technical evidence.

Explain what a delegated result means for the user before presenting the
technical report. Say what the evidence establishes, what you infer and what
conditions any probe introduced. Describe proposed behavior as future work.
Put identifiers and links after the explanation they support.

Before sending or saving text, simplify sentences that rely on unexplained
concepts. Check that the text itself explains the terms the reader needs, who
acts, why they act and what follows. Match the detail to the question and keep
simple replies short. Choose a format that fits the answer. Introduce the
workflow when the reader needs it to understand the update.

After rewriting, check claims about behavior and evidence. Say what a setting
specifies, what someone actually did and what they observed. Before attributing
an effect to a setting, check whether it takes an action to produce that effect
or an observation to confirm it. Do not make a claim sound more certain when
simplifying it. Preserve its conditions, exceptions and limits of scope. When a
claim depends on a source, check the claim against that source and report any
uncertainty you cannot resolve.

Tell the user before you start or resume controlling the mouse or keyboard.
Stop sending input when the user takes over. When you finish, including after
an interruption or recoverable error, follow the tool's documented step for
ending the session or releasing control before starting unrelated work. Wait
for confirmation before telling the user, in their language, that control has
ended and they can work normally.

### Choose the task checkout

For new implementation work, use Git to identify the current checkout and
branch before you start. To resume a saved plan, use the checkout and branch in
the plan unless the user selects another location. For other work, use the
location the user selected if there is one. Otherwise, establish whether to use
the current checkout or a new one. While that choice is pending, continue with
read-only discovery. Report missing local assets and rely on authorized sources
to provide them.

## User-Selected Operations

The user chooses the outcome and scope in ordinary language or by naming a
skill. The main session uses the request and the active task context to choose
the relevant repository skill, reads it and coordinates the steps within the
user's authorization. Use `TA-WF-Main-implement-task` to implement an existing
task. Use the repair mode of `TA-WF-Step-implement-change` to fix identified
errors in an active task. Use `TA-WF-Main-plan-change` to create or amend a plan.
A request for an explanation or assessment authorizes inspection. For other
operations, match the user's intent to the skill descriptions, allowing for
wording that differs from these examples. Resolve any missing scope before
starting work that depends on it.

Follow the authorization the user gave when you apply a skill. Treat findings,
recommendations and workflow completion as results. Start another operation or
stage only when the user's authorization covers it. Commit, publish and clean
up only within the scope authorized for each action.

If the user redirects work beyond its approved scope, behavior or acceptance
criteria, identify which requirement changed and update any active plan to
reflect the user's decision.

The repository defines agent roles in `.codex/agents/`. Runtime roles and
installed plugin skills are separate. Give each independent review a fresh
context. Report limits on filesystem access, context or budget only as far as
you observe the runtime enforce them.

### Subagent assignment and model selection

The main session coordinates `TA-WF-Main-` workflows using its current model
and reasoning effort. Loading a skill leaves those settings as they are.
Assign planning and test design to `TA-planner`, implementation of code and
tests and repairs to `TA-implementer`, independent assessments to
`TA-code-reviewer`, hardware source research to `TA-hardware-researcher`, and
selected command runs and coverage measurements to `TA-test-runner`. For other
skills, use the executor specified in the skill. If the operation needs several
assignments, choose the settings for each one separately.

Choose the model and reasoning effort separately. For each setting, take the
first applicable value in this order.

1. The user's explicit choice for that assignment.
2. The hardware behavior rule below.
3. The default given by the selected skill.
4. The role default in the table.
5. The project `[agents]` default in `.codex/config.toml`.

If the user specifies only one setting, use this order for the other setting.

| Role | Default model | Default reasoning effort |
| --- | --- | --- |
| `TA-planner` | `gpt-6.2-astra` | `xhigh` |
| `TA-implementer` | `gpt-6.2-sol` | `high` |
| `TA-code-reviewer` | `gpt-6.2-sol` | `high` |
| `TA-hardware-researcher` | `gpt-6.2-sol` | `high` |
| `TA-test-runner` | `gpt-6.2-sol` | `medium` |

For implementation, repair or optimization that changes Amiga chipset or
peripheral hardware behavior, use `gpt-6.2-astra` with `xhigh`. Examples include
changes to custom-chip timing, DMA, bus arbitration, register semantics,
interrupts and device behavior. To decide if the rule applies, look at the
requested behavior, the plan, the interactions affected and the authorized
edits. Use paths and hardware names to locate the work, then check what behavior
the assignment would change. Skip this rule for assignments limited to reading
chipset code or changing prose, logs or build tooling. Reviewers keep their
role defaults when reviewing chipset work.

Before every launch, including those for corrections or resumed work, announce
the role, model and reasoning effort, with a short reason for the choice. Give
the agent the selected skill, checkout, requirements, current plan and findings,
scope of permitted edits and the result to return. Start with a fresh context
(`fork_turns = "none"` where supported) and pass `model` and `reasoning_effort`
explicitly to the spawn tool. The assigned planner completes the included test
design within its own assignment and uses no second planner.

When choosing requirements for an assignment, check the review decisions
register for entries that apply. Give the agent only the relevant entries and
their source pointers.
[Review decisions register](.agents/references/review-decisions.md)

Reconsider both settings if the assignment changes scope. If the new
assignment needs different settings, launch a fresh agent with the current
context it needs. A follow-up message leaves the existing agent's settings in
place. Omit `model` and `model_reasoning_effort` from role TOML files so their
precedence cannot override the choices for an assignment.

If you cannot launch the assigned role or use the selected settings, say what
was unavailable. Leave the work for the selected role and settings. Report
which settings you requested and which the runtime records confirm the agent
used. An agent's description of its own model is unverified. Changes to role or
skill configuration may become discoverable only in a new session.

## Task-Specific Rules and Navigation

Read only the technical requirements relevant to the work. If the user asks
for a review, read the criteria assigned to it. When authoring changes, limit
checklist reading to what is relevant to the assignment. For read-only
questions, read only what you need to answer.

| Work | Read at its trigger |
| --- | --- |
| Crates, exports or CPU module layout | [Workspace layout](Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout) |
| Chip, DMA, bus, interrupts or peripherals | [Hardware topology](Obsidian/Amiga/Design/General%20Architecture.md#11-physical-bus-topology--inter-chip-isolation-invariant) |
| Reference conversion | [Nested bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable program README |

When using a graph to find your way around, identify the project or reference
collection it covers and check the relevant definitions in the source. Use `rg`
when it gives you the needed context or when the graph is missing, stale or
insufficient. Say whether hardware evidence comes from manuals, measurements or
reference-model behavior. Manuals are in `Obsidian/Amiga/Reference/`. Imported
models and vectors are in `ref_src/`.

For planning, test design, implementation and review, consult the shared
knowledge tools guide when graph navigation, RAG retrieval, hardware research
or a GUI screenshot would help. The guide explains which operations read data
and which update indexes or refresh graphs.
[Knowledge tools guide](.agents/references/knowledge-tools.md)

## Verification and Completion

Report which checks ran, what inputs they used and what results they returned.
Identify evidence from checks run for this work, evidence retained from an
earlier run and conclusions from an assessment. Readiness requires valid
results from the required checks. A failed check, missing evidence, evidence
that no longer applies or an assessment alone leaves readiness unverified.

Prepare changes, review, commit, publish and clean up only within the scope the
user selected for each action. Choosing a workflow leaves that scope as given.
Before reporting completion, review the diff against the user's request and
state which checks ran and what limits still apply.
