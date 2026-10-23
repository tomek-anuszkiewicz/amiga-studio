#![allow(dead_code)]

#[path = "../benches/bench_instructions/candidate_measure.rs"]
mod candidate_measure;
#[path = "../benches/bench_instructions/candidate_probe.rs"]
mod candidate_probe;
#[path = "../benches/bench_instructions/inventory.rs"]
mod inventory;
#[path = "../benches/bench_instructions/platform.rs"]
mod platform;

use cpu::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn retry_confirmation_requires_agreement_between_measurements() {
    assert!(candidate_measure::retry_agrees(12.0, 12.2, 3.0));
    assert!(!candidate_measure::retry_agrees(10.0, 12.0, 3.0));
    assert!(!candidate_measure::retry_agrees(12.0, f64::NAN, 3.0));
}

#[test]
fn decoder_inventory_separates_legal_encodings_from_decode_exceptions() {
    let counts = inventory::decoder_inventory();
    assert_eq!(counts.legal_encodings + counts.decode_exceptions, 65_536);
    assert!(counts.legal_encodings > 0);
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

#[test]
fn jump_workloads_execute_700_jumps_without_guest_helpers() {
    let variants = [
        ("4ED3", "()"),
        ("4EEB", "()"),
        ("4EF3", "(0,)"),
        ("4EF3", "(1,)"),
        ("4EF3", "(2,)"),
        ("4EF3", "(3,)"),
        ("4EF8", "()"),
        ("4EF9", "()"),
        ("4EFA", "()"),
        ("4EFB", "(0,)"),
        ("4EFB", "(1,)"),
        ("4EFB", "(2,)"),
        ("4EFB", "(3,)"),
    ];
    for (hex, index_choices) in variants {
        let opcode = u16::from_str_radix(hex, 16).expect("literal opcode");
        let candidate: candidate_probe::Candidate = serde_json::from_value(serde_json::json!({
            "case_id": format!("jump:{hex}:{index_choices}"),
            "representative_opcode": hex,
            "path": "normal",
            "index_choices": index_choices,
            "cpu_mode": "supervisor",
        }))
        .expect("literal candidate");
        let words = candidate_probe::instruction_words(opcode, &candidate);
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
        assert_eq!(outcome.0, "counted_700_jumps", "{hex} {index_choices}");
        assert_eq!(outcome.1, 700, "{hex} {index_choices}");
        let (mut replay_cpu, mut replay_bus) = prepared.expect("validated fixture");
        for step in 0..700 {
            let pc = replay_cpu.state.instruction_pc() as usize;
            let executed =
                u16::from_be_bytes([replay_bus.chip_ram[pc], replay_bus.chip_ram[pc + 1]]);
            assert_eq!(executed, opcode, "{hex} {index_choices} step {step}");
            assert!(replay_cpu.step_instruction(&mut replay_bus) > 0);
        }
        assert_eq!(replay_cpu, cpu, "{hex} {index_choices}");
        assert_eq!(replay_bus.chip_ram, bus.chip_ram, "{hex} {index_choices}");
    }
}
