# M68000 Benchmark Anomaly Report

Base full run: `20261026T004156918649Z`
Current snapshot also contains 86 refreshed cases from run(s) `20261026T085530110000Z`, `20261026T091457888920Z`; per-case sources are in the current JSON snapshot.

## Findings

**0 baseline regressions, 0 baseline coverage issues, 3 instruction/mode outliers, 1 A7 outliers, 18 measurement issues.**

Baseline: `20261026T004156918649Z`; current: `20261026T004156918649Z`. 2257 cases are comparable.

### Instruction and addressing-mode outliers

Times cover complete measured programs. A mode signal means the instruction gap differs from the usual gap across matching modes. Negative deviation means the first workload is faster than expected; positive means slower.

| Case | Compared with | Times ns/target | Deviation | Evidence |
| --- | --- | ---: | ---: | --- |
| `adda/adda_w_immediate` | `suba/suba_w_immediate` | 42.89 / 34.60 | +19.8% | gap vs median of 8 modes; usual gap +0.61 ns |
| `addi/addi_l_indexed` | `subi/subi_l_indexed` | 84.30 / 76.31 | +9.8% | gap vs median of 7 modes; usual gap +0.14 ns |
| `neg/neg_w_absolute_long` | `not/not_w_absolute_long` | 59.77 / 54.49 | +8.3% | gap vs median of 7 modes; usual gap +0.54 ns |

### A7 compared with A0-A6

The pairs have the same instruction, program shape, target count and CPU clocks after normalizing register numbers. Byte stack steps can access different addresses.

| A7 case | A0-A6 comparison | A7 ns/target | A0-A6 ns/target | Difference |
| --- | --- | ---: | ---: | ---: |
| `adda/adda_w_immediate_a7_destination` | `adda/adda_w_immediate` | 34.67 | 42.89 | -19.2% |

<details><summary>18 cases with at least 10% series spread</summary>

| Case | Spread |
| --- | ---: |
| `addi/addi_w_predecrement` | 14.8% |
| `dbcc/dbpl_condition_true` | 12.0% |
| `move/move_w_absolute_short_to_stack_postincrement` | 11.9% |
| `subq/subq_b_displacement` | 11.7% |
| `move/move_l_pc_indexed_to_stack_predecrement` | 11.6% |
| `scc/shi_indexed` | 11.5% |
| `move/move_w_address_postincrement_to_sr` | 11.5% |
| `adda/adda_w_immediate_a7_destination` | 11.4% |
| `bcc/bvs_s_untaken` | 10.8% |
| `ori/ori_w_postincrement` | 10.7% |
| `sub/sub_w_d2_a7_postincrement` | 10.6% |
| `move/move_w_pc_indexed_to_absolute_long` | 10.5% |
| `scc/scc_a7_postincrement` | 10.5% |
| `sub/sub_w_a7_predecrement_d0` | 10.5% |
| `scc/sge_a7_postincrement` | 10.4% |
| `move/move_b_pc_displacement_to_address_predecrement` | 10.3% |
| `move/move_l_pc_displacement_to_address_predecrement` | 10.3% |
| `dbcc/dbge_counter_expired` | 10.1% |

</details>

Within-run timing deviations of at least 5% are listed when they exceed the observed series spread. Baseline regressions use the same percentage limit. They are review signals, not CPU correctness results. Whole-program times include helper instructions.
Details: `m68k_benchmark_baseline_comparison.json`, `m68k_benchmark_cross_case_anomalies.json`, `m68k_benchmark_a7_comparison.json`, and `m68k_benchmark_anomalies.json`.
A7 coverage: 340/365 cases found a matching A0-A6 workload.
