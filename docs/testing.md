# How the Emulator Is Verified

This guide explains what the project's tests check and how to interpret their
results. It gives users and developers an overview of the evidence behind the
emulator and starting points for investigating a failure. Exact commands and
verification routing are documented in [tools/verification/README.md](../tools/verification/README.md)
and the verifier's command-line help.

## Different Tests Answer Different Questions

The emulator has to produce the right state at the right time. A correct value
at the end of an instruction can still conceal a bus access performed in the
wrong clock phase. Verification therefore uses several complementary approaches:

| Evidence | Question it helps answer |
| --- | --- |
| Crate tests | Does an individual component behave as expected? |
| Integration tests | Do the CPU, bus, machine loop and peripherals coordinate correctly? |
| 68000 SingleStep vectors | Does an instruction match generated final state, instruction clocks and the timed bus callback sequence? |
| CPU wait-state crate tests | Does the CPU retain and retry a refused transfer correctly? |
| Whole-machine captures | Does a selected hardware program produce the expected visible result? |
| Performance measurements | How much host time does an emulated workload consume? |

Architecture and quality checks complement these tests by checking project
constraints. They do not establish hardware timing by themselves.

## CPU Accuracy and Bus Waiting

The CPU harness uses Tom Harte's SingleStepTests, a large collection of instruction
cases generated using a software implementation. Every executed vector compares
registers, instruction clocks and timed bus callbacks with its JSON reference.
Sampling limits the number of inputs and never weakens those
comparisons. SingleStep uses the published JSON format;
test names are descriptions and do not select how registers or bus data are
interpreted. Internal delays advance the expected transaction timeline, so correct
total clocks do not conceal an early or late read or write. Existing state masks
and the callback interface's signal limits are documented in the
[CPU verification specification](../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md#12-input-scope-and-complete-verification).

SingleStep uses the [reviewed SingleStep override catalog](../crates/test_runner/tests/overrides/singlestep/README.md)
for approved corrections to exact records, with the originals retained in `ref_src`.

Ordinary CPU tests use PhysicalMemory and a small recording wrapper to refuse
specific transfers zero, once or repeatedly, then release them. They check CCK
phases, stable retry identity, exactly one successful transfer and waiting cost
from independently counted refusals. Long halves, address updates, prefetch,
stack/exception paths, read-modify-write and MOVEM/MOVEP progress have focused
coverage. These tests run with ordinary CPU crate tests. They do not verify
production Agnus arbitration or the complete chipset scheduler; those
relationships need integration tests. Memory-placement and DMA-schedule
permutation tests have been removed.

For source and procedure details, start with [test_runner](../crates/test_runner/)
and the [SingleStep verification skill](../.agents/skills/m68k-singlestep-test/SKILL.md).

## Whole-Machine Behavior and Performance

External hardware programs and reference captures provide another view: the
combined effect of instruction timing, DMA, video and peripheral behavior.
The available vAmigaTS reference captures support investigation, but the previous
capture runner has been removed. There is currently no executable capture
comparison entry point. Whole-machine capture verification remains future work.

Performance also needs separate evidence. A benchmark measures host execution
time, whereas a cycle assertion measures emulated time. Stored benchmark results
provide a comparison reference; fresh measurements run
explicitly. Benchmark regression test targets and the old chipset throughput
runner have been removed. The [benchmark design](../Obsidian/Amiga/Design/Performance/CPU%20Instruction%20Benchmarking.md)
explains the measurement model.

<a id="completion-verification"></a>
## Run Verification and Read the Result

For an authorized commit, run from the repository root:

```powershell
python tools/verification/verify.py commit
```

It runs standard tests and quality checks on the current working tree, including
unchanged files where the check's scope requires them. Staging is not required,
and the command does not create a commit. Ordinary task completion uses relevant
focused checks. Full SingleStep verification runs automatically at
milestone closure or explicitly for diagnosis; changes alone do not select it
in the commit profile. CPU waiting tests run with ordinary crate tests. Use
`python tools/verification/verify.py --help` for the available profiles and
[tools/verification/README.md](../tools/verification/README.md#completion-verification) for execution details.

The report includes code coverage for ordinary crate tests. Coverage shows which
code was executed; it does not tell you whether the assertions were sufficient,
whether all hardware cases were represented, or whether execution was fast.

When assessing a result, check which suites actually ran, whether they used full
or sampled inputs, and whether failures or unavailable assets limited the run.
A pass applies to the executed cases and assertions. For the broader design and
verification strategy, follow the
[testing specification](../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md).
