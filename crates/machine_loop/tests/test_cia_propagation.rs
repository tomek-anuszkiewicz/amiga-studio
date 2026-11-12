#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{cia_reg, A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State, AddressBus, BusResult};

mod common;

fn serialized_copy(machine: &A500Machine) -> A500Machine {
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str::<A500State>(&json).unwrap();
    let mut restored = A500Machine::new(machine.config.clone());
    restored.load_state(&state);
    assert!(
        machine.save_state() == restored.save_state(),
        "serialized continuation differs"
    );
    restored
}

#[test]
fn restoration_does_not_consume_unrouted_cia_output() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cpu.state.set_stopped(true);
    common::cia::configure_drive_control_outputs(&mut machine);
    common::cia::write_byte(&mut machine, cia_reg::cia_b::REGISTER_PRB, 0x7f);
    machine.step_cck();
    assert_eq!(machine.floppy.pins.control_input, 0x7f);
    assert!(machine.cia_b.set_prb(0x75));
    machine.cia_b.step_cck();
    while machine.cia_b.eclock_phase != 2 {
        machine.cia_b.step_cck();
    }
    assert!(machine.cia_b.prb_mutated);
    assert!(!machine.floppy.drives[0].motor_on);
    let mut restored = serialized_copy(&machine);
    machine.step_cck();
    restored.step_cck();
    assert!(
        machine.save_state() == restored.save_state(),
        "serialized continuation differs"
    );
    assert!(!restored.floppy.drives[0].motor_on);
    assert!(!restored.cia_b.prb_mutated);
    machine.step_cck();
    restored.step_cck();
    assert!(machine.save_state() == restored.save_state());
    assert!(restored.floppy.drives[0].motor_on);
}

#[test]
fn pending_port_write_and_tod_read_resume_at_the_original_edge() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cpu.state.set_stopped(true);
    machine.cia_a.tod = 0x123456;
    assert_eq!(machine.cia_a.todhi(), 0x12);
    assert!(machine.cia_a.set_ddra(1));
    assert_eq!(machine.cia_a.tod_read_latch, Some(true));
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    let mut restored = serialized_copy(&machine);
    for cck in 1..=7 {
        machine.step_cck();
        restored.step_cck();
        assert!(
            machine.save_state() == restored.save_state(),
            "continuation differs at CCK {cck}"
        );
        assert_eq!(restored.cia_a.tod_latched, cck >= 2);
        assert_eq!(
            restored.physical_memory.is_low_memory_overlay_active(),
            cck < 2
        );
    }
    assert_eq!(machine.cia_a.todlo(), 0x56);
    assert_eq!(machine.cia_a.tod_read_latch, Some(false));
    let mut restored = serialized_copy(&machine);
    for _ in 0..5 {
        machine.step_cck();
        restored.step_cck();
        assert!(
            machine.save_state() == restored.save_state(),
            "serialized continuation differs"
        );
    }
    assert!(!restored.cia_a.tod_latched);
}

#[test]
fn guest_cpu_cia_write_retries_and_serializes_before_acceptance_and_commitment() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.physical_memory.map_chip_ram_to_low_memory();
        // MOVE.B #$5A,$BFEC01; STOP #$2700. No firmware or OS service runs.
        let code = [0x13fc_u16, 0x005a, 0x00bf, 0xec01, 0x4e72, 0x2700];
        for (index, word) in code.into_iter().enumerate() {
            machine
                .physical_memory
                .write_word_debug(0x1000 + index as u32 * 2, word);
        }
        machine.set_pc_and_prime_prefetch(0x1000);
        machine.cpu.state.set_sr(0x2700);
        for _ in 0..30 {
            machine.step_cck();
            if machine.cia_a.bus_wait_cck.is_some() {
                break;
            }
        }
        assert!(machine.cia_a.bus_wait_cck.is_some());
        assert!(machine.cia_a.mutations.iter().all(Option::is_none));
        let mut restored = serialized_copy(&machine);
        let mut accepted = false;
        for _ in 0..12 {
            let micro_step = machine.cpu.state.micro().micro_step;
            let waiting = machine.cia_a.bus_wait_cck.unwrap_or(0);
            machine.step_cck();
            restored.step_cck();
            assert!(
                machine.save_state() == restored.save_state(),
                "serialized continuation differs"
            );
            if machine.cia_a.mutations.iter().flatten().count() == 1 {
                assert_eq!(machine.cia_a.eclock_phase, 1);
                assert_eq!(machine.cia_a.sdr(), 0);
                assert_eq!(machine.cia_a.bus_wait_cck, None);
                accepted = true;
                break;
            }
            if waiting > 1 {
                assert_eq!(machine.cpu.state.micro().micro_step, micro_step);
            }
        }
        assert!(accepted);
        let mut restored = serialized_copy(&machine);
        for cck in 1..=10 {
            machine.step_cck();
            restored.step_cck();
            assert!(
                machine.save_state() == restored.save_state(),
                "continuation differs at CCK {cck}"
            );
            assert_eq!(machine.cia_a.sdr(), 0x5a);
            assert!(machine.cia_a.mutations.iter().all(Option::is_none));
        }
    }
}

#[test]
fn reset_cancels_pending_capture_and_bus_synchronization() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cpu.state.set_stopped(true);
    assert!(machine.cia_a.set_ddra(1));
    assert_eq!(
        machine
            .memory_bus()
            .write_byte(cia_reg::cia_b::REGISTER_PRA, 0),
        BusResult::WaitState
    );
    assert!(machine.cia_b.bus_wait_cck.is_some());
    machine.reset_warm();
    assert!(machine.cia_a.mutations.iter().all(Option::is_none));
    assert_eq!(machine.cia_b.bus_wait_cck, None);
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    machine.step_cycles(10);
    assert!(machine.physical_memory.is_low_memory_overlay_active());
}

#[test]
fn input_port_b_releases_drive_control_without_selecting_or_starting_drives() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cpu.state.set_stopped(true);
    // HRM Appendix F: DDRB zero selects input; passive pull-ups keep pins high.
    // The reset output latch is zero, but it does not drive an input pin.
    assert_eq!(machine.cia_b.prb, 0);
    assert_eq!(machine.cia_b.ddrb(), 0);
    common::cia::write_byte(&mut machine, cia_reg::cia_b::REGISTER_DDRB, 0);
    for (index, drive) in machine.floppy.drives.iter().enumerate() {
        assert!(!drive.selected, "DF{index} selected by an input pin");
        assert!(!drive.motor_on, "DF{index} motor started by an input pin");
        assert_eq!(drive.cylinder, 0);
    }
}

#[test]
fn guest_cpu_live_icr_read_acknowledges_only_after_synchronization() {
    let mut harness = common::MachineHarness::new();
    // MOVE.B $BFED01,D0; STOP #$2700.
    harness.load_cpu_code(0x1000, &[0x1039, 0x00bf, 0xed01, 0x4e72, 0x2700]);
    harness.machine.cpu.state.set_sr(0x2700);
    harness.machine.cia_a.icr_data = 0x81;
    for _ in 0..30 {
        harness.machine.step_cck();
        if harness.machine.cia_a.bus_wait_cck.is_some() {
            break;
        }
    }
    assert!(harness.machine.cia_a.bus_wait_cck.is_some());
    let mut restored = serialized_copy(&harness.machine);
    let mut accepted = false;
    for _ in 0..12 {
        assert_eq!(harness.machine.cia_a.icr_debug(), 0x81);
        harness.machine.step_cck();
        restored.step_cck();
        assert!(harness.machine.save_state() == restored.save_state());
        if harness.machine.cia_a.bus_wait_cck.is_none() {
            assert_eq!(harness.machine.cia_a.icr_debug(), 0);
            accepted = true;
            break;
        }
    }
    assert!(accepted);
    harness.machine.step_cycles(8);
    assert_eq!(harness.machine.cpu.state.d_byte(0), 0x81);
}
