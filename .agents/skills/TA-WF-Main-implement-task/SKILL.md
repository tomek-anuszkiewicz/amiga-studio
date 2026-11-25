---
name: TA-WF-Main-implement-task
description: Execute an approved plan, confirm live telemetry and run applicable tests with up to two test-driven repairs.
---

# Execute the selected plan

The main session prepares the execution, assigns implementation to
`TA-implementer` and assigns applicable tests to `TA-test-runner`. Use the
[subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection)
for each assignment. Apply the [result contract](../../references/workflow-contract.md).

## Prepare and assign the change

1. Identify the selected plan, task ID, approved revision and agreed scenarios.
   Inspect the current Git checkout, branch and existing worktrees. Establish
   the intended checkout and branch before writing. For a resume, use the
   recorded checkout and branch unless the engineer selected another location.
   Check the approved inputs and identify the last completed step and the next
   unfinished step. Reuse approved scenarios, including attachments of older
   plans. Resolve material ambiguities and missing behavior decisions before
   the work that depends on them.
2. Use [TA-WF-Step-register-execution](../TA-WF-Step-register-execution/SKILL.md)
   against the primary checkout, including when implementation will run
   elsewhere. Keep the plan's task ID, identify the actual initiating VS Code
   thread and retain the execution's receipt and live confirmation. An execution
   ID identifies one recording attempt. Require applicable proof before
   checkout preparation or implementation. Follow the
   [capture guide](../../../tools/workflow/capture/README.md#register-and-read-the-fresh-event)
   for reuse in the same open execution and thread, pending confirmation,
   expiry and retry. Report failed or missing confirmation with the task and
   attempt preserved.
3. For an isolated checkout that needs to be created, use
   [TA-WF-Step-start-plan](../TA-WF-Step-start-plan/SKILL.md) with the selected
   branch and base. The provisioning script copies the local assets. Place
   the selected plan and required inputs at their corresponding relative
   paths. In an authorized existing checkout, including primary `master`,
   use the governing plan in place. Transfer only missing required inputs
   and resolve conflicting copies before writing. Check the working plan
   against the approved inputs before handoff. If preparation fails, report
   the obstacle and preserve any partial checkout.
4. In that checkout, assign [TA-WF-Step-implement-change](../TA-WF-Step-implement-change/SKILL.md)
   to `TA-implementer` with the plan, agreed scenarios, checkout, branch,
   relevant requirements and write scope. The implementer writes the change,
   tests and necessary documentation, formats any Rust edits and inspects the diff.
   Use the same procedure in existing and newly provisioned checkouts.

## Run the applicable tests

1. Once the implementer returns the change and diff inspection, select the
   applicable tests from the approved plan and give their commands to
   `TA-test-runner`. Coordinate with the implementer to run each selected
   suite once on the same inputs. For a selected browser interaction test,
   have the runner use a distribution built on those inputs. Include
   `./tools/web.ps1 build` in the assignment if the runner needs to build it.
   Run the interaction test to establish the browser outcome. For whole
   ordinary Rust packages or a planned workspace run, use
   `python tools/verification/verify.py test coverage crate <name>` or
   `test coverage workspace` as applicable. Use the normal entry point for a
   selected harness, Python suite or expressly filtered test. Record an export
   produced by the selected test command with its location. The runner's
   assignment covers the applicable tests and their same-run instrumentation.
   English checks, separate coverage measurement or assessment, reviewer
   assignments, challenges, aggregate profiles and standalone build or quality
   gates are outside Implement Task.
2. Collect the command, test scope, actual test outcomes and any interruption
   or unrun work. Finish testing as soon as all applicable tests pass. If no
   tests apply, explain why. If a run is blocked or a required outcome is
   unavailable, report what is missing, the cause and any completed results.
   If a build error prevents cases from starting, report the compiler
   diagnostics and the unrun cases. An export failure after a recorded test
   pass is a tooling limitation. Report both the command outcome and any
   recorded test pass.
3. For a failing in-scope test case, give the implementer the failing
   observation, expected result and relevant output. The implementer repairs
   within the approved behavior, test scope and write scope, then formats any
   Rust edits and inspects the diff. Give the failed and affected tests back
   to the runner. Include any rebuild needed for those tests to use the
   repaired inputs, such as refreshing a browser distribution after its inputs
   change. Retain other results only while their inputs still apply.
   Allow at most two test-driven repairs. A new failing case uses the
   remaining allowance. Return the evidence for the engineer's decision if the
   failure persists after the second repair and rerun, falls outside scope or
   needs an unavailable prerequisite.

Return the changed paths, governing plan, checkout and branch, registration and
proof locations, test outcomes, repairs and remaining work.
Leave the changes unstaged. Implementation results describe the checks
on the working files. Delivery and final verification require their own
authorization.
