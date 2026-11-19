# Test design and evidence criteria

Use these standards for the selected test design, test audit or tests/evidence
review. The [testing policy](../rules/unit-testing-policy.md) owns protection,
domain obligations and gate triggers. A report-only assessor never adds tests or
starts missing measurements automatically.

## Scenario design

Map approved requirements and chip/module interactions to existing evidence before
proposing tests. Identify a small set of meaningful scenarios, suitable levels,
independent expected results, observation methods and concrete gaps. Prefer reused
behavioral/integration fixtures to a new framework. Do not impose assertion-count,
per-function or exhaustive catalogue quotas. Thin declarations and forwarders may
be protected through parent integration coverage.

For each proposed test, name the current project use, observed regression or
approved requirement it protects and the effect a failure would have. Apply
this to tests of verification tools as well. A fixture may force a rare failure
to make it repeatable. Explain what can cause that failure in supported use.
Add coverage for a future use when the change introducing it is in scope.

Expected results come from a contract, specification, external reference or explicit
invariant. An implementation-generated baseline can establish stability or a
relative property such as invariance under waiting; it cannot independently prove
absolute correctness. Preserve independent literal encodings when sharing a
production constant would hide an incorrect definition.

Use existing host APIs or snapshots for observations. A new runtime API solely for
a test needs a demonstrated observation gap and an intentional interface. Test-only
code may be an intentional Host I/O or debugger control; do not delete it merely
because production callers are absent. Shared setup belongs in local test helpers.

## Behavior and domain boundaries

Assert expected state, side effects, failures and unchanged state where relevant.
Merely executing a line or changing a test file is not behavioral protection.
Maintain production integration coverage when chips gate execution, transfer Chip
RAM, signal peers or compete for bus slots. A synthetic CPU bus does not establish
production bus or chipset scheduler correctness.

The four verification tiers are isolated crate behavior, machine/debugger/GUI
integration, CPU reference vectors, and whole-machine hardware captures. Capture
execution is currently unavailable. Every executed SingleStep vector compares
final state, instruction clocks and timed bus transactions with its JSON reference;
sampling only changes the number of inputs. Report harness limits explicitly.

For a CPU execution path absent from supplied vectors, design a handwritten
regression in `crates/cpu/tests/` with independent expectations. Check resulting
state and side effects, total CPU clocks and each observable bus access's CCK,
address, direction and value. Include wait-state retries when bus behavior changes.
Report missing reference-vector coverage; neighboring passes do not verify that path.

GUI tests distinguish behavior/integration assertions from visual confirmation.
Apply the [GUI criteria](gui-criteria.md) and reproduce the original interaction
before repair. Converter work instead follows its
[nested testing scope](../../tools/bootstrap/AGENTS.md#converter-testing).

## Coverage and adequacy assessment

For affected emulator production code, inspect canonical coverage's unexecuted
functions and source spans after implementation stabilizes. Earlier measurement
needs an explicit request or diagnostic reason. Coverage is execution evidence,
not correctness; assess assertions with the final measured source.

Check measured suites, targets, conditional compilation, instrumentation and stale
object mappings before identifying a gap. Evidence from another target may explain
a missing span but must be supplied, not assumed. Full executable-code coverage is
the goal, not a global percentage gate. Uncovered reachable changed behavior or
affected interactions require meaningful protection before readiness. Report
pre-existing out-of-scope gaps separately; never hide them with exclusions,
allowlists, reduced measurement scope or meaningless assertions.

The Implementer may adjust fixtures and strengthen assertions within approved
scenarios. Newly found scenarios outside that plan are proposed to the caller;
required gaps remain blockers while undecided. The Test Auditor reports adequacy
and measurement limits; coverage collection is a separate selected operation.

## Evidence review

Separate behavioral tests, static architecture checks, performance measurements and
diagnostic traces. Source-inspection checks belong in verification tooling;
measurements in benchmark targets. Generated diagnostics, including the per-attempt
timing report, stay ignored. Keep the report locator and digest for archival
provenance. The report stays outside reusable gate artifacts and supplies no pass evidence.

Match actual commands, exit statuses, input identities, acceptance criteria and
intact artifacts to the selected candidate. Distinguish newly executed checks,
retained evidence and inspected reports. Failed, missing, interrupted or invalidated
groups are not passes. Changed global fingerprints do not by themselves invalidate
an unchanged scoped review, but the comparison and original identity must be retained.

Batch acceptance needs the canonical verdict for the complete candidate; milestone
acceptance additionally needs its triggered gates. Combined integration needs
combined evidence, not source-branch passes. Do not claim intermediate commits,
hardware, platforms or performance were checked beyond the supplied evidence.
Canonical verification owns deterministic selection and reuse; an agent report
does not replace it or start it without assignment.
