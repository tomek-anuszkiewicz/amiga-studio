#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for Amiga 500 Save State Serialization and Restoration

use config::custom_reg;
use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::{A500Machine, A500State};

mod common;
use common::continuation::{
    retire, rom_machine, BOOT_CODE, BOOT_PC, BOOT_SSP, COLOR_MARKER, RAM_MARKER,
};

#[test]
fn snapshot_between_field_strobe_and_hblank_finishes_the_previous_display_row() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::standard_1mb(standard));
        machine.denise.color[0] = 0x0f00;
        machine
            .memory_bus()
            .write_custom_word(custom_reg::denise::REGISTER_BPLCON0, 4);
        let mut boundary = false;
        for _ in 0..(3 * 313 * 228) {
            machine.step_subsystems_cck();
            if machine.denise.sync.field_odd
                && machine.denise.sync.vpos == 0
                && machine.denise.sync.hpos == 9
            {
                boundary = true;
                break;
            }
        }
        assert!(
            boundary,
            "the machine did not reach the pending field boundary"
        );
        assert_eq!(machine.denise.frame_builder.frame_number, 0);
        assert!(!machine.denise.frame_builder.field_odd);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state = serde_json::from_str::<A500State>(&json).unwrap();
        let mut restored = A500Machine::new(A500Config::standard_1mb(standard));
        restored.load_state(&state);
        for _ in 0..10 {
            machine.step_subsystems_cck();
            restored.step_subsystems_cck();
            assert_eq!(restored.denise.frame_builder, machine.denise.frame_builder);
        }
        assert_eq!(machine.denise.frame_builder.frame_number, 1);
        assert!(machine.denise.frame_builder.field_odd);
        let frame = &machine.denise.frame_builder;
        let blank_lines = if standard == VideoStandard::Pal {
            26
        } else {
            21
        };
        let complete_lines = usize::from(machine.denise.sync.field_lines) - blank_lines - 1;
        assert_eq!(frame.height as usize, complete_lines * 2 - 1);
        let row = complete_lines * 2 - 2;
        // The retained two-sample Denise pipeline follows the blanking edge.
        assert_eq!(frame.get_pixel(0, row), 0xff000000);
        assert_eq!(frame.get_pixel(2, row), 0xffff0000);
        assert_eq!(frame.get_pixel(835, row), 0xffff0000);
        assert_eq!(
            frame.frame_buffer().len(),
            frame.width as usize * frame.height as usize
        );
    }
}

#[test]
fn snapshots_resume_reset_stop_and_a_refused_second_operand_half() {
    for boundary in 0..3 {
        let prefix: &[u16] = match boundary {
            0 => &[0x4E70],         // RESET output asserted
            1 => &[0x4E72, 0x2000], // STOP, interrupts enabled
            _ => &[0x2418],         // MOVE.L (A0)+,D2
        };
        let mut code = prefix.to_vec();
        code.extend_from_slice(BOOT_CODE);
        let mut machine = rom_machine(&code);
        let boot_pc = BOOT_PC + prefix.len() as u32 * 2;
        match boundary {
            0 => {
                for _ in 0..3 {
                    assert!(!machine.step_cck());
                }
                assert!(machine.cpu.state.is_reset_line_asserted());
                // The pulse edge has already reset peripherals. A restore in the
                // asserted interval must not manufacture a second assertion edge.
                machine.denise.color[0] = 0x0F00;
            }
            1 => {
                retire(&mut machine, boot_pc);
                assert!(machine.cpu.state.is_stopped());
                // Vector 27 lives in the ROM overlay; handler clears Paula's source.
                machine.physical_memory.write_word_debug(0xF8006C, 0x00F8);
                machine.physical_memory.write_word_debug(0xF8006E, 0x2000);
                for (index, word) in [0x5282, 0x33FC, 0x0040, 0x00DF, 0xF09C, 0x4E73]
                    .into_iter()
                    .enumerate()
                {
                    machine
                        .physical_memory
                        .write_word_debug(0xF82000 + index as u32 * 2, word);
                }
            }
            _ => {
                machine.cpu.state.set_a_long(0, 0xC41000);
                machine.physical_memory.write_word_debug(0xC41000, 0x1234);
                machine.physical_memory.write_word_debug(0xC41002, 0x5678);
                // Complete the high half in an idle slot, then refuse the low
                // half at a production DRAM refresh slot (HPOS 1).
                machine.agnus.hpos = 49;
                assert!(!machine.step_cck());
                assert!(!machine.step_cck());
                let step = machine.cpu.state.micro().micro_step;
                machine.agnus.hpos = 0;
                assert!(!machine.step_cck());
                assert!(machine.physical_memory.chip_ram_blocked);
                assert_eq!(machine.cpu.state.micro().micro_step, step);
                assert_eq!(machine.cpu.state.d_long(2), 0);
                assert_eq!(machine.cpu.state.micro().source, 0x1234_0000);
            }
        }
        // Actual serialization drops the runtime-only static micro-step slice.
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let snapshot = serde_json::from_str::<A500State>(&json).unwrap();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&snapshot);
        if boundary != 1 {
            assert!(!restored.cpu.state.micro().current_steps.is_empty());
        }
        if boundary == 1 {
            for target in [&mut machine, &mut restored] {
                target
                    .memory_bus()
                    .write_custom_word(custom_reg::paula::REGISTER_INTENA, 0xC040);
                target
                    .memory_bus()
                    .write_custom_word(custom_reg::paula::REGISTER_INTREQ, 0x8040);
            }
        }
        let mut finished = false;
        for tick in 0..4096 {
            let original_done = machine.step_cck();
            let restored_done = restored.step_cck();
            assert_eq!(
                original_done, restored_done,
                "retirement diverged at tick {tick}"
            );
            assert_eq!(
                machine.cpu.state, restored.cpu.state,
                "CPU diverged at tick {tick}"
            );
            assert_eq!(machine.denise.color[0], restored.denise.color[0]);
            if boundary == 0 && tick == 0 {
                assert_eq!(restored.denise.color[0], 0x0F00);
            }
            if original_done && machine.cpu.state.instruction_pc() == boot_pc + 22 {
                finished = true;
                break;
            }
        }
        assert!(finished, "restored program did not reach its final loop");
        assert_eq!(machine.cpu.state.d_long(0), 43);
        assert_eq!(machine.cpu.state.d_long(1), 53);
        assert_eq!(
            machine.cpu.state.d_long(2),
            match boundary {
                1 => 1,
                2 => 0x12345678,
                _ => 0,
            }
        );
        if boundary == 2 {
            assert_eq!(machine.cpu.state.a_long(0), 0xC41004);
        }
        assert_eq!(machine.cpu.state.ssp(), BOOT_SSP);
        assert_eq!(machine.denise.color[0], COLOR_MARKER);
        assert_eq!(
            &machine.physical_memory.slow_ram.as_ref().unwrap()[RAM_MARKER..RAM_MARKER + 2],
            &[0, 42]
        );
        assert!(!machine.cpu.state.is_reset_line_asserted() && !machine.cpu.state.is_stopped());
        for _ in 0..4 {
            retire(&mut machine, boot_pc + 22);
            retire(&mut restored, boot_pc + 22);
            assert_eq!(restored.denise.color[0], COLOR_MARKER);
        }
        assert_eq!(machine.save_state(), restored.save_state());
    }
}

#[test]
fn test_save_state_deterministic_stepping_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));

    let code = [0x4E, 0x71, 0x60, 0xFE]; // NOP; BRA *-0
    machine.physical_memory.map_chip_ram_to_low_memory();
    machine
        .physical_memory
        .write_bytes_debug(0x001000, &code)
        .unwrap();
    machine
        .cpu
        .set_pc_and_prime_prefetch(0x001000, &mut machine.physical_memory);

    // 1. Advance machine by 300 CCKs
    machine.step_cycles(300);

    // 2. Capture baseline save state
    let snapshot = machine.save_state();

    // 3. Step forward another 600 CCKs to produce branch A
    machine.step_cycles(600);
    let state_a = machine.save_state();

    // 4. Restore the baseline snapshot into machine
    machine.load_state(&snapshot);

    // 4b. Verify that Cpu::restore_state re-hydrated the static micro-step slice
    assert!(
        !machine.cpu.state.micro().current_steps.is_empty(),
        "Micro-step slice must be re-hydrated by restore_state on load_state"
    );

    // 5. Step forward the exact same 600 CCKs to produce branch B
    machine.step_cycles(600);
    let state_b = machine.save_state();

    // 6. Assert bit-for-bit determinism across branches A and B
    assert_eq!(
        state_a.cck, state_b.cck,
        "Color Clock cycle count must match exactly"
    );
    assert_eq!(
        state_a.cpu, state_b.cpu,
        "CPU state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.agnus, state_b.agnus,
        "Agnus state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.denise, state_b.denise,
        "Denise state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.paula, state_b.paula,
        "Paula state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.cia_a, state_b.cia_a,
        "CIA-A state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.cia_b, state_b.cia_b,
        "CIA-B state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.physical_memory, state_b.physical_memory,
        "PhysicalMemory buffers must match"
    );
    assert_eq!(state_a, state_b);
}

#[test]
fn test_save_state_restores_kickstart_rom_unconditionally() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    // Synthetic Kickstart ROM
    let dummy_rom = vec![0x42; 256 * 1024];
    machine
        .physical_memory
        .write_bytes_debug(0xF80000, &dummy_rom)
        .unwrap();

    let state = machine.save_state();
    assert_eq!(state.physical_memory.kickstart_rom().len(), 256 * 1024);

    let mut target_machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    assert!(target_machine.physical_memory.kickstart_rom()[8..]
        .iter()
        .all(|&b| b == 0xFF));

    target_machine.load_state(&state);
    assert_eq!(target_machine.physical_memory.kickstart_rom(), dummy_rom);
}

#[test]
fn test_save_state_adopts_configuration_and_memory() {
    let machine_1mb = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));
    let state_1mb = machine_1mb.save_state();
    assert!(state_1mb.physical_memory.slow_ram.is_some());

    let mut machine_512k = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    assert!(machine_512k.physical_memory.slow_ram.is_none());

    machine_512k.load_state(&state_1mb);

    assert_eq!(machine_512k.config, state_1mb.config);
    assert!(machine_512k.physical_memory.slow_ram.is_some());
}
