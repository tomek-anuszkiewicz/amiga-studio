#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Gary decode through the production scheduler, without firmware or disk media.

use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::{A500Machine, AddressBus, BusResult};

mod common;

#[test]
fn guest_dual_cia_word_transfer_keeps_one_pending_event_per_receiver() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for preset in [
            A500Preset::Bare512k,
            A500Preset::Standard1Mb,
            A500Preset::ExpandedPowerUser,
        ] {
            let mut machine = A500Machine::new(A500Config::from_preset(preset, standard));
            machine.physical_memory.map_chip_ram_to_low_memory();
            // MOVE.W #$8765,$A04CFE; MOVE.W $BFCC00,D0; STOP #$2700.
            // Both accesses select SDR on both CIAs, through different mirrors.
            let code = [
                0x33fc, 0x8765, 0x00a0, 0x4cfe, 0x3039, 0x00bf, 0xcc00, 0x4e72, 0x2700,
            ];
            for (index, word) in code.into_iter().enumerate() {
                machine
                    .physical_memory
                    .write_word_debug(0x1000 + index as u32 * 2, word);
            }
            machine.set_pc_and_prime_prefetch(0x1000);
            machine.cpu.state.set_sr(0x2700);
            let mut saw_wait = false;
            let mut saw_pending = false;
            for _ in 0..150 {
                let before = machine.cpu.state.micro();
                let stable_payload = (
                    before.addr1,
                    before.addr2,
                    before.source,
                    before.destination,
                );
                let micro_step = before.micro_step;
                let remaining = machine.cia_a.bus_wait_cck;
                machine.step_cck();
                if machine.cia_a.bus_wait_cck.is_some() {
                    saw_wait = true;
                    assert_eq!(machine.cia_a.bus_wait_cck, machine.cia_b.bus_wait_cck);
                    if remaining.is_some_and(|cck| cck > 1) {
                        assert_eq!(machine.cpu.state.micro().micro_step, micro_step);
                        let after = machine.cpu.state.micro();
                        assert_eq!(
                            (after.addr1, after.addr2, after.source, after.destination),
                            stable_payload
                        );
                    }
                }
                if machine.cia_a.mutations.iter().flatten().count() == 1 {
                    assert!(!saw_pending, "a retry inserted another write");
                    saw_pending = true;
                    assert_eq!(machine.cia_a.eclock_phase, 1);
                    assert_eq!((machine.cia_a.sdr, machine.cia_b.sdr), (0, 0));
                    for (chip, value) in [(&machine.cia_a, 0x65), (&machine.cia_b, 0x87)] {
                        let pending: Vec<_> = chip.mutations.iter().flatten().collect();
                        assert_eq!(pending.len(), 1);
                        assert_eq!(pending[0].reg_offset, 0x0c);
                        assert_eq!(pending[0].value, value);
                        assert_eq!(pending[0].remaining_cck, 1);
                    }
                }
                if machine.cpu.state.is_stopped() {
                    break;
                }
            }
            assert!(saw_wait && saw_pending);
            assert!(machine.cpu.state.is_stopped());
            assert_eq!(machine.cpu.state.d_word(0), 0x8765);
            assert_eq!((machine.cia_a.sdr, machine.cia_b.sdr), (0x65, 0x87));
            assert!(machine.cia_a.mutations.iter().all(Option::is_none));
            assert!(machine.cia_b.mutations.iter().all(Option::is_none));
        }
    }
}

#[test]
fn every_cia_a_bank_alias_routes_only_committed_pa0_to_overlay() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for preset in [
            A500Preset::Bare512k,
            A500Preset::Standard1Mb,
            A500Preset::ExpandedPowerUser,
        ] {
            let mut machine = A500Machine::new(A500Config::from_preset(preset, standard));
            machine.cpu.state.set_stopped(true);
            machine
                .physical_memory
                .write_bytes_debug(0xf80000, &[0x12, 0x34]);
            machine.physical_memory.write_chip_word(0, 0x5678);
            for bank in 0xa0..=0xbf {
                let base = bank << 16;
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x1234);
                assert_eq!(machine.physical_memory.read_chip_word(0), 0x5678);
                // Dual-select even DDRA write: the byte reaches CIA-A as well.
                assert_eq!(
                    common::cia::accept_write(&mut machine, base | 0x42fe, 1),
                    BusResult::Ready(())
                );
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x1234);
                assert_eq!(machine.cia_a.ddra, 0);
                machine.step_cck();
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x5678);
                // CIA-B-only PRA must not change OVL, even on an odd write.
                assert_eq!(
                    common::cia::write_byte(&mut machine, base | 0x90ff, 1),
                    BusResult::Ready(())
                );
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x5678);
                // CIA-A-only PRA: retain RAM while pending, then select ROM.
                assert_eq!(
                    common::cia::accept_write(&mut machine, base | 0xe0fe, 1),
                    BusResult::Ready(())
                );
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x5678);
                machine.step_cck();
                assert_eq!(machine.memory_bus().read_word_debug(0), 0x1234);
                // Prepare zero PRA and input PA0 for the next alias.
                common::cia::write_byte(&mut machine, base | 0xe2fe, 0);
                common::cia::write_byte(&mut machine, base | 0xe0fe, 0);
            }
        }
    }
}
