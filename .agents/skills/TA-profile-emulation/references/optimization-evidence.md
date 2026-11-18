# Optimization evidence and acceptance

An optimization explains why less work is needed and why the remainder is sufficient.
A speedup does not justify obscure control flow, special-case hacks or extra coupling.
Preserve meaningful registers, signals, phases and units. Documented hardware
encodings and local table indices are valid; private cache indices, packed execution
hints and sentinel values must not become chip-to-chip control backchannels.

Local derived state needs simple dependencies and effective update points. Reject
scattered invalidation, hidden ordering and another independently writable device
model. Useful mechanisms include simpler arithmetic over established ranges,
visiting occupied/active entries, stable decoding at effective input changes,
on-demand inspection, small immutable mappings, direct final-buffer writes and
clearing only known-invalid regions. These are inspection prompts, not blanket
permission to apply every technique.

## Baseline and frequency

Bind evidence to actual source including local changes, compiler/features/settings,
executable and matching symbols. A commit ID alone misses dirty inputs. Preserve
baseline artifacts before a rebuild; never reset another actor's source or index.
Disposable old diagnostics require checking their workload, boundaries, identities
and assumptions before reuse.

Choose a specific boundary: core throughput, instruction cost, GUI responsiveness,
presentation or browser execution. Separate emulation, buffer conversion,
upload/drawing and pacing. Capped display FPS is not core throughput. Preserve
normal pixel/audio generation unless explicitly isolating it. Record PAL/NTSC,
machine configuration, ROM/input identity, fixed frames/instructions, warmup and
whether GUI/pacing are included.

Profile warmed optimized execution and resolve inline frames with exact symbols.
Sample weights are not call counts or hardware cycles. Inspect frequent paths even
when sampling cannot resolve small savings; source operation counts are not timings.
Look for repeated register decoding, empty-capacity scans, temporary arrays/copies,
duplicate consumers, unnecessary conversions and repeated dispatch/checks.
Inspect optimized instructions when actual cost is uncertain.

Use bounded counters only to resolve unknown calls, changed inputs, occupied entries,
active channels or writes. Compare diagnostic output/state with the unmodified path;
measure speed uninstrumented. Idle observations do not establish maximum occupancy.
Rank candidates by likely total savings, evidence, complexity and risk without a
minimum percentage. Frequency times estimated cost is prioritization, not proof.

## Timing and correctness

For skipped/cached work identify applied writes, DMA, reset, clone, restore,
inspection and mid-line changes. Price cache construction/invalidation and active
costs. Derive an inactivity condition from existing state when sufficient.
Preserve observable phases, write ordering, bus waits, read side effects, interrupts,
collisions and snapshot continuation. Precomputed output must not apply future
effects early. Stable levels may skip redundant delivery; repeated strobes/writes
can remain distinct events. Host input collection does not authorize reduced guest
sampling. Changing guest timing is a requirement decision.

Keep baseline/candidate compiler, features, workload, warmup and host conditions
comparable. Repeat unprofiled A/B measurements, interleaving versions such as A-B-B-A
to expose drift. Avoid concurrent builds, tests and profilers. Report sample count,
spread and disturbed runs; do not discard inconvenient results.
Include active cases, transitions and frequent invalidation, not only idle wins.
For core changes measure whole-machine execution when available; a small focused
gain may be useful while whole-machine effect remains below noise.
Native evidence does not establish browser performance. Preserve native/Wasm builds
and measure the browser path when that is the claimed outcome.

Report absolute time and relative change with units and scope. Frame-time reduction
and throughput increase use different denominators. A higher sampled share after
other work shrinks is not itself a regression. Differential output/state equality
preserves the starting model, not independent silicon truth. CPU benchmarks do not
replace reference vectors. Keep goldens, timing references, comparison masks and
acceptance criteria intact.

Follow the selected behavioral scenarios and meaningful focused checks.
Refresh profiling only when changed code/workloads invalidate cost ranking or a
specific next decision lacks evidence. Tiny edits do not require broad audits,
reference surveys or all-workload campaigns. Uncertain results require a bounded,
concrete next approach; the operation cannot repeat indefinitely or silently accept
a new tradeoff. Final retained changes compare directly with the original baseline,
including any correctness correction. Do not add independent percentage gains.

## Required candidate report

Create/update `micro-optimizations.md` under the task evidence directory unless the
user explicitly chose another destination or conversation-only delivery. Link it
from the invocation result. Scale detail to the assigned scope, with no candidate quota.

For each stable candidate ID, retain status, priority, file/function, unnecessary
work, observed cost/frequency, proposed formula/guard/data flow, why it is valid,
dependencies/update points, memory/maintenance cost and behavior at risk.
Explain what remains after previous improvements. Include independently expected
correctness checks and a bounded comparison that can decide the proposal.
Algebraic identity proves neither full device behavior nor speed.

Separate current measurements, historical evidence and hypotheses. Keep source/build
identity, workloads, limits, profiles, diagnostics/disassembly and missing evidence.
Record rejected ideas (compiler already removed the work, slower guard/table,
missing evidence or excessive complexity), actual checks, before/after spread,
active/idle results and remaining opportunities. Unmeasured candidates have no
invented percentage. Refresh statuses against final source and do not recommend
already completed work. Plans remain under `.agent/tasks/`; durable lessons belong
in a few supported project examples rather than a copy of every run.

## Call tree format

The primary function-level result preserves calling contexts; a function may occur
on multiple branches. Saved HTML uses expandable rows. A flat ranking can supplement
the tree, not replace candidate inspection. Show together:

| Column | Meaning |
| --- | --- |
| Branch % capture | Function and all descendants relative to the whole captured thread. |
| Self % capture | Function's own work excluding descendants, with the same denominator. |
| Children % capture | All descendant work: branch minus self, without overlapping double counts. |

Before rounding, Branch = Self + Children. Never use percent of parent or normalize
a selected subtree to 100%. Expansion, filtering and root changes preserve all
denominators/percentages. Above the tree show selected root share and share outside
it. Sampled milliseconds are estimated capture-interval time, not measured per-call
duration or exact CPU/GPU time.
