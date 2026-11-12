//! Deterministic raw pulses and RAM words; no drive media or firmware execution.
#![allow(clippy::unwrap_used)]

use config::{custom_reg, A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn active(standard: VideoStandard, length: u16, control: u16) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    machine.agnus.hpos = 40;
    machine.agnus.dskpt = 0x2000;
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8210));
    assert!(machine.paula.set_adkcon(0x8000 | control));
    assert!(machine.paula.set_dsklen(length));
    machine.step_cycles(3);
    assert!(machine.paula.set_dsklen(length));
    machine.step_cck();
    machine
}

fn bits(machine: &mut A500Machine, value: u16, count: u8) {
    for bit in (0..count).rev() {
        machine.floppy.pins.read_data_high = value & (1 << bit) == 0;
        machine.step_cck();
        machine.floppy.pins.read_data_high = true;
        machine.step_cycles(6);
    }
}

#[test]
fn read_dma_resumes_partial_input_and_transfers_distinct_fifo_words_once() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = active(standard, 0x8003, 0x0500);
        bits(&mut machine, 0x4489, 16);
        bits(&mut machine, 0x5a42 >> 11, 5);
        assert_eq!(machine.agnus.dskpt, 0x2000);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state: A500State = serde_json::from_str(&json).unwrap();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&state);
        for target in [&mut machine, &mut restored] {
            bits(target, 0x5a42, 11);
            bits(target, 0xa55a, 16);
            bits(target, 0xcc33, 16);
            target.step_cycles(500);
            assert_eq!(target.agnus.dskpt, 0x2006);
            for (address, expected) in [
                (0x2000, 0x5a42),
                (0x2002, 0xa55a),
                (0x2004, 0xcc33),
                (0x2006, 0),
            ] {
                assert_eq!(target.physical_memory.read_chip_word(address), expected);
            }
            assert_eq!(target.paula.dsklen & 0x3fff, 0);
            assert!(!target.paula.dma_active);
            assert_eq!(target.paula.disk.buffered_words(), 0);
            assert_eq!(target.paula.interrupts.intreq & 0x1002, 0x1002);
        }
        assert_eq!(machine.save_state(), restored.save_state());
    }
}

#[test]
fn write_dma_fetches_ram_into_paula_and_serializes_all_buffered_words() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = active(standard, 0xc003, 0x0100);
        let words = [0x8001u16, 0x5a42, 0xffff];
        for (index, word) in words.into_iter().enumerate() {
            machine
                .physical_memory
                .write_chip_word(0x2000 + index as u32 * 2, word);
        }
        let mut serial = Vec::new();
        let mut bit_phase = None;
        let mut grants = Vec::new();
        let mut previous_pointer = machine.agnus.dskpt;
        for _ in 0..1000 {
            machine.step_cck();
            if machine.agnus.dskpt != previous_pointer {
                grants.push(machine.agnus.hpos);
                previous_pointer = machine.agnus.dskpt;
            }
            if machine.paula.pins.disk_write_enable {
                let phase = bit_phase.get_or_insert(0usize);
                if *phase % 7 == 0 {
                    serial.push(!machine.paula.pins.disk_write_high);
                }
                *phase += 1;
            }
        }
        let expected: Vec<bool> = words
            .into_iter()
            .flat_map(|word| (0..16).rev().map(move |bit| word & (1 << bit) != 0))
            .collect();
        assert_eq!(serial, expected);
        assert_eq!(grants, [7, 9, 11]);
        assert_eq!(machine.agnus.dskpt, 0x2006);
        assert_eq!(machine.paula.interrupts.intreq & 2, 2);
        assert!(!machine.paula.pins.disk_write_enable);
        assert!(machine.floppy.pins.write_enable_high);
        assert!(machine
            .floppy
            .drives
            .iter()
            .all(|drive| !drive.disk_inserted));
    }
}
