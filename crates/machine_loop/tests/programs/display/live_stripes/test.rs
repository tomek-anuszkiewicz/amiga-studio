//! GUI smoke fixture: HRM chapter 3, low-resolution data fetch and modulo.
//! Load at $1000 with overlay disabled and a supervisor stack in Chip RAM.
//! Observe two completed fields within four PAL/NTSC field budgets. The guest
//! owns every display register and source word; host setup only loads its bytes.

use crate::common::MachineHarness;
use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;

const PROGRAM: &[u8] = include_bytes!("program.bin");
const ENTRY: u32 = 0x1000;
const MAX_CCK: u64 = 4 * 313 * 228;
const BLUE: u32 = 0xff00_0020;
const RED: u32 = 0xfff0_0000;
const GREEN: u32 = 0xff00_f000;

#[test]
fn guest_stripes_update_across_completed_fields() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut harness = MachineHarness {
            machine: A500Machine::new(A500Config::standard_1mb(standard)),
        };
        let machine = &mut harness.machine;
        machine.physical_memory.map_chip_ram_to_low_memory();
        machine.cpu.state.set_sr(0x2700);
        machine.cpu.state.set_ssp(0x70000);
        for (index, byte) in PROGRAM.iter().enumerate() {
            machine
                .physical_memory
                .write_byte_debug(ENTRY + index as u32, *byte);
        }
        machine.set_pc_and_prime_prefetch(ENTRY);
        let mut fields = Vec::new();
        for _ in 0..MAX_CCK {
            machine.step_cck();
            let frame = &machine.denise.frame_builder;
            if frame.frame_number == 2 + fields.len() as u64 {
                let pair = [frame.get_pixel(300, 100), frame.get_pixel(308, 100)];
                assert!(pair.contains(&BLUE), "missing background: {pair:x?}");
                assert!(
                    pair.contains(&RED) || pair.contains(&GREEN),
                    "missing stripes: {pair:x?}"
                );
                assert_eq!(pair, [frame.get_pixel(300, 120), frame.get_pixel(308, 120)]);
                fields.push(pair);
                if fields.len() == 2 {
                    break;
                }
            }
        }
        assert_eq!(
            fields.len(),
            2,
            "guest exceeded field budget at PC {:x}",
            machine.cpu.state.instruction_pc()
        );
        assert_ne!(
            fields[0], fields[1],
            "guest palette update did not reach video"
        );
    }
}
