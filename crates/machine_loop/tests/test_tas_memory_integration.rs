#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TAS through CPU, machine coordination and the production MemoryBus.
//! Expected write suppression follows MemoryBus.md's TAS erratum and
//! CPU Motorola M68000.md Section 7.1. This is not an electrical pin capture.

mod common;

use common::MachineHarness;
use config::A500Preset;
use machine_loop::{AddressBus, BusResult};

fn check_tas_memory_region(base: u32, suppress_write: bool) {
    let mut failures = Vec::new();
    for offset in [0, 1] {
        for original in [0x00u8, 0x35, 0x80] {
            for extend in [0u16, 0x10] {
                let mut harness = MachineHarness::new_with_preset(A500Preset::ExpandedPowerUser);
                let addr = base + offset;
                let neighbor = addr ^ 1;
                // Prove ordinary writes reach installed RAM; disable the boot overlay
                // through the existing harness and leave DMA/interrupt sources disabled.
                assert_eq!(
                    harness.machine.memory_bus().write_byte(addr, original),
                    BusResult::Ready(())
                );
                assert_eq!(
                    harness.machine.memory_bus().write_byte(neighbor, 0x5a),
                    BusResult::Ready(())
                );
                assert_eq!(
                    harness.machine.physical_memory.read_byte_debug(addr),
                    original
                );
                harness.machine.cpu.state.set_a_long(0, addr);
                // Start with N/Z/V/C set so the instruction must replace all four.
                harness.machine.cpu.state.set_sr(0x200f | extend);
                harness.load_cpu_code(0x1000, &[0x4ad0, 0x4e71, 0x4e71]); // TAS (A0); NOP
                assert!(harness.step_until_pc(0x1002, 128), "TAS did not retire");

                let expected_ccr = extend
                    | if original == 0 { 0x04 } else { 0 }
                    | if original & 0x80 != 0 { 0x08 } else { 0 };
                assert_eq!(harness.machine.cpu.state.sr(), 0x2000 | expected_ccr);
                assert_eq!(harness.machine.cpu.state.a_long(0), addr);
                assert_eq!(
                    harness.machine.physical_memory.read_byte_debug(neighbor),
                    0x5a
                );
                let expected = if suppress_write {
                    original
                } else {
                    original | 0x80
                };
                let actual = harness.machine.physical_memory.read_byte_debug(addr);
                if actual != expected {
                    failures.push(format!(
                        "TAS at ${addr:06X}, original ${original:02X}, X=${extend:02X}: expected ${expected:02X}, got ${actual:02X}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_tas_chip_ram_discards_write_and_updates_flags() {
    check_tas_memory_region(0x002000, true);
}

#[test]
fn test_tas_slow_ram_discards_write_and_updates_flags() {
    check_tas_memory_region(0xc02000, true);
}

#[test]
fn test_tas_fast_ram_sets_bit_seven_and_updates_flags() {
    check_tas_memory_region(0x202000, false);
}
