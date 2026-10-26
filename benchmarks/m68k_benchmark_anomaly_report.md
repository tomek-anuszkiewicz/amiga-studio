# M68000 Benchmark Anomaly Report

Run: `20261025T203204689106Z`

## Findings

**0 baseline regressions, 0 baseline coverage issues, 6 instruction/mode outliers, 6 A7 outliers, 31 measurement issues.**

Baseline: `20261025T203204689106Z`; current: `20261025T203204689106Z`. 2257 cases are comparable.

### Instruction and addressing-mode outliers

Times cover complete measured programs. A mode signal means the instruction gap differs from the usual gap across matching modes. Negative deviation means the first workload is faster than expected; positive means slower.

| Case | Compared with | Times ns/target | Deviation | Evidence |
| --- | --- | ---: | ---: | --- |
| `adda/adda_l_a2_d5` | `suba/suba_l_a2_d5` | 26.64 / 36.23 | -30.5% | direct match; inspect program details before attributing the gap |
| `add/add_b_postincrement` | `sub/sub_b_postincrement` | 29.18 / 39.49 | -28.0% | gap vs median of 9 modes; usual gap -0.71 ns |
| `adda/adda_w_immediate` | `suba/suba_w_immediate` | 45.03 / 35.44 | +24.3% | gap vs median of 8 modes; usual gap -0.18 ns |
| `and/and_w_displacement` | `or/or_w_displacement` | 43.33 / 38.10 | +10.3% | gap vs median of 8 modes; usual gap +1.04 ns |
| `add/add_l_d1_d0` | `sub/sub_l_d1_d0` | 25.35 / 26.94 | -6.1% | direct match; inspect program details before attributing the gap |
| `add/add_l_a7_postincrement_d0` | `sub/sub_l_a7_postincrement_d0` | 39.10 / 41.40 | -5.7% | direct match; inspect program details before attributing the gap |

### A7 compared with A0-A6

The pairs have the same instruction, program shape, target count and CPU clocks after normalizing register numbers. Byte stack steps can access different addresses.

| A7 case | A0-A6 comparison | A7 ns/target | A0-A6 ns/target | Difference |
| --- | --- | ---: | ---: | ---: |
| `sub/sub_b_a7_postincrement_d0` | `sub/sub_b_postincrement` | 29.56 | 39.49 | -25.1% |
| `suba/suba_l_a2_d5_a7_destination` | `suba/suba_l_a2_d5` | 27.15 | 36.23 | -25.1% |
| `adda/adda_w_immediate_a7_destination` | `adda/adda_w_immediate` | 35.09 | 45.03 | -22.1% |
| `negx/negx_b_a7_postincrement` | `negx/negx_b_postincrement` | 33.90 | 41.05 | -17.4% |
| `movea/movea_l_a7_predecrement_d0` | `movea/movea_l_predecrement` | 44.17 | 38.97 | +13.4% |
| `tst/tst_l_a7_predecrement` | `tst/tst_l_predecrement` | 43.73 | 40.51 | +7.9% |

<details><summary>31 cases with at least 10% series spread</summary>

| Case | Spread |
| --- | ---: |
| `eor/eor_b_d1_postincrement` | 53.3% |
| `scc/scc_displacement` | 36.8% |
| `bcc/bcc_w_taken` | 30.4% |
| `move/move_l_absolute_long_to_address_predecrement` | 29.7% |
| `move/move_w_pc_indexed_to_address_indexed` | 26.6% |
| `add/add_w_d2_a7_predecrement` | 25.0% |
| `and/and_b_destination_predecrement` | 22.7% |
| `move/move_b_immediate_to_data_register` | 22.5% |
| `clr/clr_b_indexed` | 22.1% |
| `shift/asl_w_absolute_short` | 21.7% |
| `tst/tst_b_indexed` | 21.2% |
| `shift/lsr_l_register_large_count` | 20.2% |
| `cmp/cmp_w_a7_postincrement_d0` | 19.8% |
| `move/move_l_pc_displacement_to_stack_predecrement` | 17.5% |
| `move/move_b_absolute_long_to_stack_predecrement` | 17.4% |
| `scc/smi_a7_postincrement` | 17.2% |
| `bit/bchg_data_register_absolute_short` | 14.1% |
| `not/not_b_indirect` | 13.7% |
| `div/divu_w_immediate_dense` | 13.2% |
| `sub/sub_l_d2_a7_predecrement` | 13.2% |
| `scc/smi_a7_predecrement` | 12.7% |
| `movea/movea_w_postincrement_a5` | 12.5% |
| `rol/rol_w_postincrement` | 12.3% |
| `div/divu_w_indexed_dense` | 12.0% |
| `scc/sgt_absolute_short` | 11.7% |
| `neg/neg_l_indexed` | 11.6% |
| `roxl/roxl_l_immediate_d4` | 11.4% |
| `eori/eori_b_a7_postincrement` | 11.4% |
| `dbcc/dble_branch` | 11.0% |
| `clr/clr_b_a7_postincrement` | 10.4% |
| `movep/movep_w_displacement_to_register` | 10.4% |

</details>

Within-run timing deviations of at least 5% are listed when they exceed the observed series spread. Baseline regressions use the same percentage limit. They are review signals, not CPU correctness results. Whole-program times include helper instructions.
Details: `m68k_benchmark_baseline_comparison.json`, `m68k_benchmark_cross_case_anomalies.json`, `m68k_benchmark_a7_comparison.json`, and `m68k_benchmark_anomalies.json`.
A7 coverage: 340/365 cases found a matching A0-A6 workload.
