#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! DBcc counter expiry timing from MC68000 manual Table 8-9 and first-read
//! address from the WinUAE DBcc prefetch sequence.
//! The supplied DBcc SingleStep vectors have no false-condition case starting with
//! a zero counter, so this regression covers the missing path without changing them.

mod common;

use common::{Access, RecordingBus};
use cpu::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn dbf_counter_expiry_reads_branch_target_then_fallthrough_in_fourteen_clocks() {
    for waits in [0, 1] {
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        memory.write_word_debug(0x0ffe, 0x4afc); // Read speculatively, never executed.
        memory.write_word_debug(0x1000, 0x51c8); // DBF D0, -4
        memory.write_word_debug(0x1002, 0xfffc);
        memory.write_word_debug(0x1004, 0x4e71);
        memory.write_word_debug(0x1006, 0x7001); // Distinct second queued word.

        let mut cpu = Cpu::new();
        cpu.state.set_sr(0x2700);
        cpu.state.set_d_long(0, 0x1234_0000);
        cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);

        let mut bus = RecordingBus::new(
            memory,
            [0x0ffe, 0x1004, 0x1006]
                .map(|address| (Access::ReadWord(address), waits))
                .to_vec(),
        );

        // Each entry describes the bus callback in one CCK (two CPU clocks).
        // A refused read repeats in the following CCK before execution advances.
        let expected_by_cck: &[Option<(Access, bool)>] = if waits == 0 {
            &[
                None,                                   // 1-2: internal work
                Some((Access::ReadWord(0x0ffe), true)), // 3-4: branch target
                None,                                   // 5-6: finish target read
                Some((Access::ReadWord(0x1004), true)), // 7-8: fallthrough opcode
                None,                                   // 9-10: finish opcode read
                Some((Access::ReadWord(0x1006), true)), // 11-12: following word
                None,                                   // 13-14: retire
            ]
        } else {
            &[
                None,
                Some((Access::ReadWord(0x0ffe), false)),
                Some((Access::ReadWord(0x0ffe), true)),
                None,
                Some((Access::ReadWord(0x1004), false)),
                Some((Access::ReadWord(0x1004), true)),
                None,
                Some((Access::ReadWord(0x1006), false)),
                Some((Access::ReadWord(0x1006), true)),
                None,
            ]
        };
        let start_clock = cpu.cycle_counter();
        assert_eq!(cpu.state.ir(), 0x51c8);
        assert_eq!(cpu.state.prefetch(), 0xfffc);

        for (index, expected) in expected_by_cck.iter().enumerate() {
            let attempts_before = bus.attempts.len();
            let irc_before = cpu.state.micro().irc;
            let prefetch_before = cpu.state.prefetch();
            let retired = cpu.step_cck(&mut bus);
            let cck = index + 1;
            let clock = start_clock + 2 * cck as u64;

            assert_eq!(cpu.cycle_counter(), clock, "CCK {cck}");
            let observed = &bus.attempts[attempts_before..];
            assert_eq!(observed.len(), usize::from(expected.is_some()), "CCK {cck}");
            if let Some((access, ready)) = expected {
                assert_eq!(observed[0].access, *access, "CPU clock {clock}");
                assert_eq!(observed[0].ready, *ready, "CPU clock {clock}");
                if *ready {
                    match access {
                        Access::ReadWord(0x0ffe) => {
                            assert_eq!(cpu.state.micro().irc, 0x4afc, "CPU clock {clock}");
                        }
                        Access::ReadWord(0x1004) => {
                            assert_eq!(cpu.state.micro().irc, 0x4e71, "CPU clock {clock}");
                        }
                        Access::ReadWord(0x1006) => {
                            assert_eq!(cpu.state.prefetch(), 0x7001, "CPU clock {clock}");
                        }
                        _ => panic!("unexpected read in CCK {cck}"),
                    }
                } else {
                    assert_eq!(cpu.state.micro().irc, irc_before, "CCK {cck}");
                    assert_eq!(cpu.state.prefetch(), prefetch_before, "CCK {cck}");
                }
            }
            if cck == 3 + waits {
                assert_eq!(cpu.state.micro().ea_addr, 0x1004, "CCK {cck}");
                assert_eq!(cpu.state.micro().irc, 0x4afc, "CCK {cck}");
            }
            assert_eq!(retired, cck == expected_by_cck.len(), "CCK {cck}");
        }
        bus.assert_complete();
        assert_eq!(cpu.cycle_counter() - start_clock, 14 + 6 * waits as u64);

        assert_eq!(cpu.state.d_long(0), 0x1234_ffff);
        assert_eq!(cpu.state.sr(), 0x2700);
        assert_eq!(cpu.state.instruction_pc(), 0x1004);
        assert_eq!(cpu.state.ir(), 0x4e71);
        assert_eq!(cpu.state.prefetch(), 0x7001);
    }
}
