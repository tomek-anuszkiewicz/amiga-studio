#![allow(dead_code)]

#[path = "../benches/bench_instructions/builder.rs"]
mod builder;
#[path = "../benches/bench_instructions/candidate_probe.rs"]
mod candidate_probe;
#[path = "../benches/bench_instructions/catalog.rs"]
mod catalog;
#[path = "../benches/bench_instructions/catalog_data_a.rs"]
mod catalog_data_a;
#[path = "../benches/bench_instructions/catalog_data_b.rs"]
mod catalog_data_b;
#[path = "../benches/bench_instructions/inventory.rs"]
mod inventory;
#[path = "../benches/bench_instructions/prng.rs"]
mod prng;

use builder::BenchmarkProgramBuilder;
use cpu::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn decoder_inventory_separates_legal_encodings_from_decode_exceptions() {
    let (counts, declared) = inventory::decoder_inventory();
    assert_eq!(counts.legal_encodings + counts.decode_exceptions, 65_536);
    assert_eq!(counts.catalog_cases, catalog::all_benchmark_specs().count());
    assert_eq!(
        counts.declared_encodings + counts.undeclared_legal_encodings,
        counts.legal_encodings
    );
    assert!(declared
        .get(&0x4E71)
        .is_some_and(|ids| ids.contains(&"BASE-00")));
    assert!(!std::ptr::eq(
        cpu::OPCODE_DESCRIPTOR_TABLE[0x4E71].steps,
        &cpu::micro::STEPS_DECODE_EXCEPTION[..]
    ));
    assert!(std::ptr::eq(
        cpu::OPCODE_DESCRIPTOR_TABLE[0xFFFF].steps,
        &cpu::micro::STEPS_DECODE_EXCEPTION[..]
    ));
}

#[test]
fn signed_divide_valid_repeats_without_overflow() {
    let spec = *catalog::find_spec_by_id("ARITH-19").expect("DIVS benchmark exists");
    let program = BenchmarkProgramBuilder::new(spec).with_unroll(700).build();
    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();
    program.inject_into(&mut cpu, &mut bus);

    for operation in 0..700 {
        assert_eq!(cpu.state.instruction_pc(), program.entry_pc + 2 * operation);
        assert!(cpu.step_instruction(&mut bus) > 0);
        assert_eq!(
            cpu.state.sr() & 0x0002,
            0,
            "operation {operation} overflowed"
        );
    }
    assert_ne!(cpu.state.d_long(0) & 0xffff, 0);
    assert!(cpu.step_instruction(&mut bus) > 0);
    assert_eq!(cpu.state.instruction_pc(), program.exit_pc);
}

#[test]
fn generated_exception_branch_stack_and_stop_workloads_replay_exactly() {
    let cases = [
        ("88C3", "divide_zero", "supervisor", 700, false),
        ("6200", "untaken", "supervisor", 700, false),
        ("001F", "normal", "user", 700, false),
        ("4E73", "enter_user", "supervisor", 700, false),
        ("4E72", "remain_supervisor", "supervisor", 1, true),
    ];
    for (hex, path, mode, targets, stopped) in cases {
        let opcode = u16::from_str_radix(hex, 16).expect("literal opcode");
        let candidate: candidate_probe::Candidate = serde_json::from_value(serde_json::json!({
            "case_id": format!("contract:{hex}:{path}:{mode}"),
            "representative_opcode": hex,
            "path": path,
            "index_choices": "()",
            "cpu_mode": mode,
        }))
        .expect("literal candidate");
        let words = candidate_probe::instruction_words(opcode, &candidate);
        let mut first = None;
        for _ in 0..2 {
            let mut cpu = Cpu::new();
            let mut bus = PhysicalMemory::new();
            bus.map_chip_ram_to_low_memory();
            let mut fingerprint = String::new();
            let mut prepared = None;
            let outcome = candidate_probe::validate_candidate(
                &mut bus,
                &mut cpu,
                &candidate,
                opcode,
                &words,
                &mut fingerprint,
                Some(&mut prepared),
            );
            assert_eq!(
                outcome.0,
                if stopped {
                    "single_target_stop"
                } else {
                    "sequential_700_reached_exit"
                },
                "{hex} {path}"
            );
            assert_eq!(outcome.1, targets, "{hex} {path}");
            assert!(outcome.2 > 0, "{hex} {path}");
            assert_eq!(
                outcome.3, outcome.4,
                "target clocks drifted in {hex} {path}"
            );
            assert_eq!(cpu.state.is_stopped(), stopped, "{hex} {path}");
            assert!(prepared.is_some(), "{hex} {path}");
            let evidence = (
                outcome,
                fingerprint,
                candidate_probe::final_state_sha256(&cpu, &bus),
            );
            if let Some(previous) = &first {
                assert_eq!(
                    previous, &evidence,
                    "fixture or execution changed on replay: {hex} {path}"
                );
            } else {
                first = Some(evidence);
            }
        }
    }
}
