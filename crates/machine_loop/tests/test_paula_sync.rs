#![allow(clippy::unwrap_used)]

use config::{A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn idle(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    machine
}

fn restored(machine: &A500Machine, standard: VideoStandard) -> A500Machine {
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str::<A500State>(&json).unwrap();
    let mut copy = idle(standard);
    copy.load_state(&state);
    copy
}

#[test]
fn every_sync_address_reaches_paula_after_capture_and_survives_a_snapshot() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for (line, source_hpos, address) in [
            (0, 1, 0x038),
            (12, 1, 0x03a),
            (100, 1, 0x03c),
            (100, 3, 0x03e),
        ] {
            if standard == VideoStandard::Pal && address == 0x03e {
                continue; // OCS PAL has no alternating long-line strobe.
            }
            let mut machine = idle(standard);
            machine.agnus.vpos = line;
            machine.agnus.hpos = source_hpos - 1;
            machine.agnus.lol = address == 0x03e;
            machine.paula.sync.cck_since_horizontal = 20;
            assert!(machine.paula.set_intena(0xc020));
            machine.step_cck();
            assert_eq!(machine.agnus.pins.rga_strobe, Some(address));
            assert_eq!(
                machine.paula.pins.incoming.unwrap().rga_strobe,
                Some(address)
            );
            assert_eq!(machine.paula.sync.cck_since_horizontal, 21);
            assert_eq!(machine.paula.intreqr() & 0x0020, 0);
            assert_eq!(machine.cpu.state.ipl(), 0);

            let mut copy = restored(&machine, standard);
            for target in [&mut machine, &mut copy] {
                // The retained receiver event cannot be replaced by a changed source.
                target.agnus.pins.rga_strobe = None;
                target.step_cck();
                assert_eq!(
                    target.paula.sync.cck_since_horizontal,
                    if address == 0x03e { 22 } else { 0 }
                );
                assert_eq!(target.paula.sync.equalization, address == 0x038);
                assert_eq!(
                    target.paula.intreqr() & 0x0020,
                    if address == 0x038 { 0x0020 } else { 0 }
                );
                assert_eq!(target.cpu.state.ipl(), if address == 0x038 { 3 } else { 0 });
                assert!(target.paula.pins.rga_strobe.is_none());
                assert!(target.paula.set_intreq(0x0020));
                target.step_cck();
            }
            assert_eq!(machine.save_state(), copy.save_state());

            if address == 0x038 {
                // Restoring the active equalization phase must not invent another edge.
                let mut copy = restored(&machine, standard);
                for _ in 0..500 {
                    machine.step_cck();
                    copy.step_cck();
                    assert_eq!(copy.paula.intreqr() & 0x0020, 0);
                }
                assert_eq!(machine.save_state(), copy.save_state());
            }
        }
    }
}

#[test]
fn vertb_is_requested_once_per_field_with_and_without_interlace() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for interlaced in [false, true] {
            let mut machine = idle(standard);
            if interlaced {
                machine.agnus.dma.bplcon0 = 0x0004;
            }
            assert!(machine.paula.set_intena(0xc020));
            let mut requests = 0;
            let mut previous_request = 0;
            let short_field_ccks = match standard {
                VideoStandard::Pal => 312 * 227,
                VideoStandard::Ntsc => 131 * (227 + 228),
            };
            for _ in 0..3 * 313 * 228 {
                machine.step_cck();
                if machine.paula.intreqr() & 0x0020 == 0 {
                    continue;
                }
                // Source HPOS 1 is captured; Paula consumes the strobe at HPOS 2.
                assert_eq!((machine.agnus.vpos, machine.agnus.hpos), (0, 2));
                assert_eq!(machine.cpu.state.ipl(), 3);
                if requests == 0 {
                    assert_eq!(machine.cck, 2);
                } else {
                    let extra_line = u64::from(interlaced && requests == 2) * 227;
                    assert_eq!(
                        machine.cck - previous_request,
                        short_field_ccks + extra_line
                    );
                }
                previous_request = machine.cck;
                requests += 1;
                assert!(machine.paula.set_intreq(0x0020));
                if requests == 3 {
                    break;
                }
            }
            assert_eq!(requests, 3);
        }
    }
}
