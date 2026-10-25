# Hand-Written M68000 Benchmark Cases

For new instruction cases, follow the [agent procedure](../../.agents/skills/add-m68k-benchmark/SKILL.md).

[`config.json`](config.json) holds the target instruction count, instruction
limit, series count, and target duration per series shared by the cases. The
limit is 100,000,000 executed guest instructions per complete program run;
it guards against a program that never reaches its end marker, rather than
limiting wall time. Each catalog case has only `program.s` and `case.json`;
the shared [runner](runner.rs) assembles, loads, and measures the cases.

Cases cover selected instruction forms, addressing modes, and execution
paths. Their directory names identify the measured form; `program.s` defines
the target workload and `case.json` defines its initial state and target
primary opcode. The case JSON also states the image origin, required initial
registers, and memory regions. A `random_words` region has a label in the
assembly image; a `fixed_words` region supplies CPU-visible RAM words outside
that image, such as absolute-short operands. A `random_bcd_words` region uses
the same assembly-image placement and validation as `random_words`, but packs
two decimal digits (0-9) into each byte. A `label_address` region names an
exception vector and an assembly handler label; the runner resolves the label
and writes its address into the vector table. A `return_frames` region fills
the reserved SSP stack with one status word and return PC per RTE or RTR
target before timing. For RTE/RTR return frames, the runner requires two-byte
targets and helpers.
A `return_addresses` region fills the reserved SSP stack with the continuation
PC after each contiguous target; an absolute JMP trampoline uses RTS to pop it.
An executable `fixed_words` region can hold a JSR or JMP trampoline outside
the assembly image; execution is allowed only within its declared words.
The case supplies initial register values in `initial_registers`, including
`ssp` or `usp` for an A7 workload, and may supply `initial_sr`. The CPU's
built-in defaults are SR `$2700` (supervisor mode, interrupt mask 7) and
zeroed registers. The runner applies only case-configured initial register
values.

The shared runner checks the program layout and executed instruction counts.
The pinned assembler has no command-line symbol definition, and `rept` does
not accept an `equ` symbol, so the runner substitutes the shared
`{{instruction_count}}` marker before assembly. A case with several targets per
repetition sets `instructions_per_loop`; the runner divides the shared count by
this value. Each assembly program is measured as a separate benchmark. The
configured target count comes from the program's declared loop structure;
the runner observes the total executed instruction count and CPU clocks.

Run from the repository root:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Assembler
python tools/verification/verify.py runner bench case add/add_w_d1_postincrement
```

The bootstrap verifies the published release archive and executable hashes
before installing the assembler and its MIT license under ignored
`tools/m68k-asm/`. The release and platform hashes are pinned in
[`assembler-manifest.json`](../../tools/bootstrap/assembler-manifest.json).
Before assembly, the runner reads that same manifest and rejects a missing
or mismatched executable with an error that points to the bootstrap command.
The assembler does not expose a `--version` option, so its executable hash
identifies the pinned release. The upstream project is
[Attackwave/m68k](https://github.com/Attackwave/m68k); no tool binary is
committed to this repository. The published x86-64 Windows, Linux, and macOS
assets are supported by this bootstrap action.

The runner loads each complete binary image at its configured origin in the
expanded preset's 4 MB Fast RAM. It does not infer instruction size from the
target count or assume that code and data have equal lengths. Execution starts
at the assembler's `entry` symbol and stops when the instruction PC reaches
the `M68K_END_MARKER!` byte sequence. Each source places this marker after the
instruction block. The runner finds its unique occurrence in the assembled
binary and uses its address as `program_end`. Bytes after the marker remain in
the loaded image but are not reached during normal completion.
The benchmark fixture maps Chip RAM into `$000000`-`$07FFFF` instead of the
boot ROM overlay. It loads `fixed_words` and `label_address` regions at their
CPU-visible RAM addresses outside the assembly image and checks that the CPU can
read them before running the program.
Cases with `platform_bus: true` execute through the production `MemoryBus`
router so bank-specific effects such as the TAS write suppression in Chip RAM
are included. The default cases execute through `PhysicalMemory`. Neither path
advances the chipset or measures Chip RAM contention.

The marker bytes are never executed. The cases use `endr` to close `rept`;
with this assembler, `endm` also repeats a following directive. The shared
`instruction_count` controls target instructions per run;
the runner observes total CPU steps and derives the helper count by subtracting
the configured target count. Interrupt acceptance can add a step to that count.
Each Bcc target is followed by `MOVE.W #imm,CCR` to select its taken or
untaken path. DBcc cases reload the counter with `MOVE.L` and then set CCR
with `MOVE.W #imm,CCR`. Scc uses the usual `MOVE.L` helper; its flags select
the `$FF` or `$00` byte written to the destination, without a branch. Other
targets use a `MOVE.L` between data registers to prepare the next target or
perform an inert copy when no restoration is needed. Targets that require a
register or status restoration use that instruction as the sole helper.
For an exception or interrupt target, the handler's RTE can be the sole
executed helper. Absolute JMP trampoline cases use RTS as that helper.

The `random_words` memory fill starts from the runner's fixed seed
`0x12345678` and uses a deterministic 32-bit recurrence:
`state = state * 1664525 + 1013904223` with wrapping arithmetic. The upper
16 bits become the next word. The seed is not configured per case.

For `random_bcd_words`, two successive upper-word samples are each reduced
modulo 100 and encoded as packed decimal bytes in one word. A single region
can supply disjoint source and destination byte arrays for BCD operations.

The first timed program establishes observed total instructions and CPU clocks.
Subsequent timed programs must match those counts; there is no separate guest
inspection pass. The runner derives helper count by subtracting configured
targets. It does not calculate expected instruction effects or compare final
registers, flags, or memory.
Before each timed run, it restores the initial CPU and complete memory bus
outside the timer. With the current shared settings, each of three series
repeats complete runs until their combined
timed CPU execution reaches at least 400 ms; the run count is measured, not
preset. The report also records full wall duration so reset and harness cost
remain visible. On Windows, the runner reuses the existing performance-core
affinity and normal-priority policy. Timed execution includes every instruction
in the complete program, including helpers. The report records the observed
helper count without requiring a configured value. Nanoseconds per target
divides the entire measured duration by the number of target instructions.
This is a whole-workload cost, not isolated 68000 instruction latency or a
silicon correctness proof.

When a case sets `stack_reserve_bytes`, the runner reserves that many bytes at
the end of 4 MB Fast RAM. The case must set the used stack pointer to an address
within that reserve; the reserve does not initialize SSP or USP. Fast RAM does not
participate in Chip RAM contention; operand accesses to Chip RAM use the mapped
bus. This CPU-only runner does not advance Agnus or the machine's CCK loop.

Each case writes `program.assembled.s`, `program.bin`, and `program.sym`
under `.agent/tmp/benchmarks/handwritten/<family>/<case-id>/`. Each invocation
records reports under `benchmarks/results/<run-id>/`. The default command runs
the complete catalog. A selector accepts an exact `family/case-id`, mnemonic,
or four-digit primary opcode and measures all matches. The declared opcode
identifies a case; it does not describe every word in the assembled program.
The approved baseline in `benchmarks/m68k_benchmark_baseline.json` and `.csv`
records the first complete default-parameter run of these workloads. Its
source reports remain under `benchmarks/results/`.
