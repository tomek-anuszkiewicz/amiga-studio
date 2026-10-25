#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! The supplied SingleStep DIVU/DIVS vectors have no normal division with a
//! zero quotient. These cases check the missing Z-flag path and its bus timing.
//! The expected 68000 clocks follow Moira's independent `cyclesDiv` model.
//! Final prefetch placement follows the nonzero-quotient SingleStep traces and
//! the A500 timing investigation in dirkwhoffmann/Moira#38.

mod common;

use common::{Access, RecordingBus};
use cpu::Cpu;
use physical_memory::PhysicalMemory;

fn check_zero_quotient(opcode: u16, clocks: u64) {
    for waits in [0_usize, 1] {
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        memory.write_word_debug(0x1000, opcode);
        memory.write_word_debug(0x1002, 0x4e71); // Next opcode.
        memory.write_word_debug(0x1004, 0x7001); // Its following word.

        let mut cpu = Cpu::new();
        cpu.state.set_sr(0x271b); // X=1, Z=0; division must preserve X and set Z.
        cpu.state.set_d_long(0, 1); // 1 / 5 = 0, remainder 1.
        cpu.state.set_d_long(1, 5);
        cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);

        let mut bus = RecordingBus::new(memory, vec![(Access::ReadWord(0x1004), waits)]);
        let start_clock = cpu.cycle_counter();
        let read_cck = (clocks - 4) / 2 + 1;
        let final_cck = clocks / 2 + waits as u64;

        for cck in 1..=final_cck {
            let attempts_before = bus.attempts.len();
            let prefetch_before = cpu.state.prefetch();
            let retired = cpu.step_cck(&mut bus);
            assert_eq!(cpu.cycle_counter() - start_clock, 2 * cck);

            let observed = &bus.attempts[attempts_before..];
            if cck == read_cck || (waits == 1 && cck == read_cck + 1) {
                assert_eq!(observed.len(), 1, "CCK {cck}");
                assert_eq!(observed[0].access, Access::ReadWord(0x1004));
                assert_eq!(observed[0].ready, cck != read_cck || waits == 0);
                if !observed[0].ready {
                    assert_eq!(cpu.state.prefetch(), prefetch_before, "CCK {cck}");
                }
            } else {
                assert!(observed.is_empty(), "unexpected transfer in CCK {cck}");
            }
            assert_eq!(retired, cck == final_cck, "CCK {cck}");
        }

        bus.assert_complete();
        assert_eq!(cpu.cycle_counter() - start_clock, clocks + 2 * waits as u64);
        assert_eq!(cpu.state.d_long(0), 0x0001_0000);
        assert_eq!(cpu.state.d_long(1), 5);
        assert_eq!(cpu.state.sr(), 0x2714);
        assert_eq!(cpu.state.instruction_pc(), 0x1002);
        assert_eq!(cpu.state.ir(), 0x4e71);
        assert_eq!(cpu.state.prefetch(), 0x7001);
    }
}

#[test]
fn divu_zero_quotient_sets_z_with_final_prefetch() {
    // Moira C68000 DIVU: 38 + 15 * 2 cycle units, each worth two CPU clocks.
    check_zero_quotient(0x80c1, 136);
}

#[test]
fn divs_zero_quotient_sets_z_with_final_prefetch() {
    // Moira C68000 DIVS: 6 + 55 - 1 + 15 cycle units, each worth two CPU clocks.
    check_zero_quotient(0x81c1, 150);
}
