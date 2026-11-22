# Agent roles

The engineer selects the work. The main session coordinates it and assigns a
specialist to each operation that calls for one. The role TOML files in this
directory give the assigned specialists their role instructions. The skill
explains what the specialist does, what inputs to use and when the assignment
ends. [Skills](../../.agents/skills/README.md)

The root instructions govern delegation and the choice of model and reasoning
effort. [AGENTS.md](../../AGENTS.md#subagent-assignment-and-model-selection)

## Roles and responsibilities

| Role | Purpose |
| --- | --- |
| [TA-planner](TA-planner.toml) | Create or amend a change plan and its test design together; the implementer writes executable tests |
| [TA-implementer](TA-implementer.toml) | Implement assigned code, tests and necessary documentation, including selected repairs and optimizations |
| [TA-code-reviewer](TA-code-reviewer.toml) | Independently assess one selected scope in fresh context; separate instances cover behavior, architecture, tests/coverage or documentation, and a fresh instance challenges reported findings |
| [TA-test-runner](TA-test-runner.toml) | Execute selected tests, checkers and coverage measurements; report actual logs and outcomes without repairs |
| [TA-hardware-researcher](TA-hardware-researcher.toml) | Investigate selected hardware sources while distinguishing manuals, physical measurements and reference-model behavior |

One planner creates or amends the change plan and test design in the same
assignment.
The implementer turns approved scenarios into executable tests. The tests
reviewer checks what the tests assert, which important scenarios are missing,
what coverage the evidence shows, which verification gates apply and whether
the evidence still applies to the candidate. The test runner collects
measurements.
An assignment to correct a finding can add tests for already approved behavior.
New behavior or scope still requires the applicable decision.

Each review starts in a fresh, independent context. The tests reviewer runs
in an instance separate from the author and the behavior and architecture
reviewers. The runtime can run multiple instances of one role independently
or in parallel within its available slots. Parallel test commands need
separate writable outputs. Run coverage measurements one at a time when
they share counters. The workflow guide shows where each assignment fits,
including the review rounds. [Workflow guide](../../.agents/skills/README.md)

## Model and reasoning selection

Before each launch, the main session reads the assignment and selects the
subagent's model and reasoning effort. It makes this selection again for
corrective work. Follow the root policy for precedence, defaults and changes
to hardware behavior. [Root policy](../../AGENTS.md#subagent-assignment-and-model-selection)

The project configuration supplies fallback defaults. The role files leave
model settings unset so the choices for the assignment can take effect.
[Project configuration](../config.toml)

When the coordinator identifies a chipset timing change in an implementation
assignment, it selects the model and reasoning effort required by the hardware
rule. It makes a fresh selection for a later repair and selects settings
separately for reviewers and command runners. The main session uses its
current model throughout the main workflow, including when it loads a skill.

Before each launch, the coordinator announces the role, model, reasoning effort
and reason for the selection. It gives the subagent the selected skill, checkout,
requirements, plan and findings, scope of permitted edits and expected result.
The role instructions and assignment describe the work. Sandbox restrictions
come from the runtime. Check runtime evidence to confirm that an independent
agent started and used the selected model.

The coordinator interprets each agent's assignment to determine which policy
rules apply. The runtime may need a new session to load new role or
configuration metadata. When runtime evidence is available, use it to confirm
the effective settings. State when a launch failed or the runtime provided too
little evidence to confirm the result. If a role or setting is unavailable,
report which assignment could not launch as specified and leave the work for
the selected role and settings.

## Results

Specialists return an explanation and JSON. An assessment can be complete
while the assessed change still has open findings. The engineer's
authorization determines whether repair or another operation follows and can
cover a sequence of assignments.
[Result contract](../../.agents/references/workflow-contract.md)
