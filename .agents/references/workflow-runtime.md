# Loading TA components

Start a deliberate new Codex session in the accepted checkout. The standalone
`.codex/agents/TA-*.toml` files register the exact role names through their `name`
fields; matching filenames make ownership clear. The project MCP configuration
remains in `.codex/config.toml`. No separate role registry, orchestration engine,
API account or compatibility role aliases are needed.

The project must be trusted for its local configuration to load. An observed
untrusted trial disabled project configuration and did not expose the custom roles.
Report that prerequisite before dispatch; selecting project trust requires the
caller's authorization and does not change role permissions.

## Model and reasoning effort

Resolve the role and atomic skill from the user's request before looking up
settings. The request can name the operation in plain language, name the skill,
or specify the role and skill together. A model or reasoning-effort choice applies
to the selected invocation only.

The following tables own the overridable model and reasoning-effort defaults.
Other instructions link here; model names in usage examples describe an explicit
user choice and do not define another set of defaults.
The [matrix renderer](../../tools/workflow/README.md#receiving-session-guidance)
reads the role and skill tables to display reasoning effort.

### Role defaults

| Role | Default model | Default reasoning effort |
| --- | --- | --- |
| `TA-planner` | `gpt-6.2-astra` | `high` |
| `TA-test-designer` | `gpt-6.2-astra` | `high` |
| `TA-implementer` | `gpt-6.2-sol` | `medium` |
| `TA-test-auditor` | `gpt-6.2-sol` | `medium` |
| `TA-code-reviewer` | `gpt-6.2-sol` | `high` |
| `TA-hardware-researcher` | `gpt-6.2-sol` | `high` |
| `TA-cpu-verifier` | `gpt-6.2-sol` | `high` |

### Skill overrides

Resolve the assigned executor through the existing skill contract. For a named
role, a skill setting overrides its default. For `main-session`, the effort
shown is a suggestion and the run uses the session's settings.

| Skill | Default model | Default reasoning effort |
| --- | --- | --- |
| `TA-implement-change` | Inherit role default | `high` |
| `TA-optimize-emulation` | `gpt-6.2-astra` | `high` |
| `TA-audit-tests` | Inherit role default | `high` |
| `TA-review-behavior` | `gpt-6.2-astra` | `high` |
| `TA-review-architecture` | `gpt-6.2-astra` | `high` |
| `TA-review-semantic-parity` | `gpt-6.2-astra` | `high` |
| `TA-create-skill` | Session setting | `medium` |
| `TA-create-agent` | Session setting | `medium` |

Skills without a specific setting use their role or session defaults, including
`TA-fix-findings` and `TA-measure-coverage`. Use `medium` for bounded work with
clear inputs, `high` for deeper analysis of behavior, tests or sources, and
`xhigh` on explicit request.

### Resolve one invocation

1. Resolve each field independently. Use the explicit user choice first, the
   selected skill's default when present, then the named role's default. A
   model-only or effort-only choice keeps the other resolved field.
2. Before dispatch, check the pair against the current runtime's model catalog
   and launch controls. In Codex CLI, `codex debug models` lists model slugs and
   their supported reasoning levels. Also check any restrictions exposed by
   the current spawn tool, including fixed role settings. Missing catalog or
   launch support is a prerequisite gap. Report the exact pair and the mismatch
   or missing control, and wait for the user's choice before using an alternative.
3. Announce the selected skill, named role, resolved model and effort, and the
   source of each field alongside the [bounded assignment](workflow-contract.md).
   Pass both resolved fields explicitly as `model` and `reasoning_effort` to the
   named role's launch tool. With Codex `spawn_agent`, use `fork_turns: "none"`
   when passing these fields. Include the necessary requirements, candidate,
   scope, evidence and round budget in the assignment itself.

Role TOML files own identity and instructions. A native `model` or
`model_reasoning_effort` pin takes precedence over launch parameters and belongs
only on a setting explicitly fixed for every invocation of that role.

Main-session skills use the session's own model and effort. To use a different
pair, select a session configured with that pair or explicitly assign an
eligible named role. Report the limitation before performing the operation when
the current session cannot apply the requested pair.

## Dispatch and evidence

Start each independent reviewer with fresh context. Keep each lane's identity
and result when consolidating the required four reviews. Schedule in waves if
the runtime has fewer available child slots.

Keep the user's request, the announced resolution, and the observed settings
distinguishable. The launch arguments record the assigned pair. For Codex CLI,
match the child session to the parent launch by thread linkage and check the
child's `turn_context` model and effort for the operation. When available, reuse
the result's `observations.model` and `observations.configuration` for the observed
model and configuration, including effort and runtime evidence. Preserve a
consumed worker result and keep any receiver observations with the runtime
evidence. If the client does not expose the effective pair, report what could be
observed. A caller announcement or worker self-report does not verify it.

When receiving a result, apply [next-step guidance](workflow-guidance.md) without
dispatching. The user's current selection binds the next executor and skill. A
schema mapping or a rendered instruction alone is not a runtime dispatch receipt.

Custom roles inherit permissions from the parent in the observed runtime.
Putting `sandbox_mode` in a role file does not establish separate protection.
Assessment roles prohibit source edits by instruction; use an appropriately
restricted parent context when available. Inherited full access is an explicit
runtime limit, not a claim of technical isolation.

For a bounded loading diagnostic, `codex debug prompt-input` can expose the main
session's model-visible input. Inspect only the relevant role names and instruction
sources, and keep any local diagnostic under `.agent/tmp/`. A real invocation must
also confirm dispatch and consume actual text/JSON; a listed role alone proves
neither execution nor isolation. Fresh context still includes system, developer,
repository and other runtime-supplied guidance. Report what was observable.

During a harness migration keep the leading session on its recorded source harness.
Candidate instruction files are content under review until a separately identified
trial loads them. A linked worktree isolates files and build artifacts, not session
instructions or real Git references. Publication trials need an independent local
repository. No trial may publish to the user's primary branch without authorization.
