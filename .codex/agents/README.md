# Agent roles

The main session coordinates the engineer's selected work and assigns specialist
roles to its component operations. A role describes who performs an assignment;
a [skill](../../.agents/skills/README.md) describes the operation, inputs and
stopping boundary. [AGENTS.md](../../AGENTS.md#subagent-assignment-and-model-selection)
owns delegation and model/effort policy. The role TOML files in this directory
define the specialist instructions.

## Roles and responsibilities

| Role | Purpose |
| --- | --- |
| [TA-planner](TA-planner.toml) | Create or amend a change plan and its test design together; the implementer writes executable tests |
| [TA-implementer](TA-implementer.toml) | Implement assigned code, tests and necessary documentation, including selected repairs and optimizations |
| [TA-code-reviewer](TA-code-reviewer.toml) | Independently assess one selected scope in fresh context; separate instances cover behavior, architecture, tests/coverage or documentation, and a fresh instance challenges reported findings |
| [TA-test-runner](TA-test-runner.toml) | Execute selected tests, checkers and coverage measurements; report actual logs and outcomes without repairs |
| [TA-hardware-researcher](TA-hardware-researcher.toml) | Investigate selected hardware sources while distinguishing manuals, physical measurements and reference-model behavior |

One planner handles both planning and test design in the same assignment. The
implementer turns approved scenarios into executable tests. The tests reviewer
assesses assertions, meaningful missing scenarios, supplied coverage, required
verification gates and evidence freshness; the runner collects measurements.
Selected corrective work can add tests for already approved behavior. New behavior
or scope retains its applicable decision boundary.

Review assignments use fresh independent contexts. The tests reviewer is a separate
instance from the author and behavior/architecture reviewers. Multiple instances
of one role can run independently or in parallel within available runtime slots.
Parallel test commands need isolated writable outputs; coverage measurements
sharing counters run serially. The [workflow guide](../../.agents/skills/README.md)
shows where these assignments participate, including the review rounds.

## Model and reasoning selection

The main session reads the actual assignment and resolves each subagent's model
and reasoning effort before launch, including corrective work. Use the precedence,
defaults and hardware-behavior rule in the
[root policy](../../AGENTS.md#subagent-assignment-and-model-selection).
The project [configuration](../config.toml) provides fallback defaults; role files
leave model settings unset so assignment-specific choices can apply.

For example, the coordinator identifies a chipset timing change in an implementation
assignment and selects the policy's contextual model/effort pair. A later repair
assignment resolves its settings again. Reviewers and command runners receive
their own settings. Main workflows run on the main session's current model;
loading a skill does not switch that model.

Before each launch, the coordinator announces the role, model, effort and reason,
then supplies the selected skill, checkout, requirements, plan/findings, write scope
and result boundary. Role instructions and assignment scope do not establish a
runtime sandbox. A loaded definition does not prove the runtime created an
independent agent or applied a particular model.

The policy is an instruction to the coordinator, not a built-in semantic
classifier. New role/configuration metadata may require a new session to load.
Confirm effective settings from runtime evidence when available and report launch
or observation gaps explicitly. An unavailable role or setting remains an actual
launch gap rather than permission to substitute another model silently.

## Results

Specialist assignments return an explanation and JSON under the shared
[result contract](../../.agents/references/workflow-contract.md). A completed
assessment can contain findings; it does not mean the assessed change passed.
Findings do not start repairs or another operation automatically. The engineer's
authorization can cover a combined sequence of assignments.
