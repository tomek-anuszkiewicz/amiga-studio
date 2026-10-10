# How the Emulator Is Verified

This guide explains what the project's tests check and how to interpret their
results. It gives users and developers an overview of the evidence behind the
emulator and starting points for investigating a failure. Exact commands and
verification routing are documented in [tests/README.md](../tests/README.md)
and the verifier's command-line help.

## Different Tests Answer Different Questions

The emulator has to produce the right state at the right time. A correct value
at the end of an instruction can still conceal a bus access performed in the
wrong clock phase. Verification therefore uses several complementary approaches:

| Evidence | Question it helps answer |
| --- | --- |
| Crate tests | Does an individual component behave as expected? |
| Integration tests | Do the CPU, bus, machine loop and peripherals coordinate correctly? |
| 68000 silicon vectors | Does an instruction match the captured CPU state and bus behavior? |
| Synthetic DMA contention | Does the CPU wait correctly when bus access is blocked? |
| Whole-machine captures | Does a selected hardware program produce the expected visible result? |
| Performance measurements | How much host time does an emulated workload consume? |

Architecture and quality checks complement these tests by checking project
constraints. They do not establish hardware timing by themselves.

## CPU Accuracy and Bus Waiting

The CPU harness uses Tom Harte's SingleStepTests, a large collection of instruction
cases captured from physical 68000 hardware. These compare registers, memory and
bus execution against external evidence. Sampling is useful during diagnosis;
the exhaustive suite is needed when claiming that all available vectors pass.

A separate contention harness varies memory placement and synthetic schedules of
blocked and available bus slots. It checks that waiting adds the expected cycles
without changing the instruction's result, and that noncontended memory access
remains unaffected. Both harnesses use a synthetic test bus. Their success does
not by itself verify the production memory bus or the complete chipset scheduler;
those relationships need integration tests.

For source and procedure details, start with [test_runner](../crates/test_runner/)
and the [silicon verification skill](../.agents/skills/m68k-singlestep-test/SKILL.md).

## Whole-Machine Behavior and Performance

External hardware programs and reference captures provide another view: the
combined effect of instruction timing, DMA, video and peripheral behavior.
The available vAmigaTS captures support manual investigation and comparison.
They are separate from the standard automatic completion checks; a passing
completion report does not imply that every capture program was executed.

Performance also needs separate evidence. A benchmark measures host execution
time, whereas a cycle assertion measures emulated time. Stored benchmark results
can detect regressions in analysis or fixtures, but they are not a new measurement
of the current build. Fresh throughput measurements and sampling profiles are
run explicitly. The [benchmark design](../Obsidian/Amiga/Design/CPU%20Instruction%20Benchmarking.md)
explains the measurement model.

<a id="completion-verification"></a>
## Run Verification and Read the Result

From the repository root, the normal completion entry point is:

```powershell
python tests/verify.py --commit
```

It runs standard tests and quality checks on the current working tree, including
unchanged files where the check's scope requires them. Staging is not required,
and the command does not create a commit. The expensive silicon and contention
suites are selected by relevant changes or explicit requests. Use
`python tests/verify.py --help` for the available profiles and
[tests/README.md](../tests/README.md#completion-verification) for execution details.

The report includes code coverage for ordinary crate tests. Coverage shows which
code was executed; it does not tell you whether the assertions were sufficient,
whether all hardware cases were represented, or whether execution was fast.

When assessing a result, check which suites actually ran, whether they used full
or sampled inputs, and whether failures or unavailable assets limited the run.
A pass applies to the executed cases and assertions. For the broader design and
verification strategy, follow the
[testing specification](../Obsidian/Amiga/Design/Testing%20Strategy%20and%20Quality%20Assurance.md).
