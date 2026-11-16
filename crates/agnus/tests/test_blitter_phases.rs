mod common;
use agnus::blitter::{Blitter, BlitterPhase};
use common::TestChipRam;

fn phase_code(phase: BlitterPhase) -> u8 {
    match phase {
        BlitterPhase::FetchA => b'A',
        BlitterPhase::FetchB => b'B',
        BlitterPhase::FetchC => b'C',
        BlitterPhase::WriteD => b'D',
        BlitterPhase::BusIdle => b'I',
        BlitterPhase::Idle => b'-',
    }
}

#[test]
fn channel_sequences_preserve_idle_cycles_waits_and_destination_drain() {
    // Current CCK model of the HRM channel sequences, in A/B/C/D enable order.
    // Startup and final pipeline offsets are not independently silicon-calibrated.
    let sequences: [&[u8]; 16] = [
        b"II", b"ID", b"IC", b"ICD", b"IBI", b"IBD", b"IBC", b"IBCD", b"AI", b"AD", b"AC", b"ACD",
        b"ABI", b"ABD", b"ABC", b"ABCD",
    ];
    for (channels, sequence) in sequences.into_iter().enumerate() {
        for fill in [0, 0x08, 0x10, 0x18] {
            let mut blitter = Blitter::new();
            blitter.bltcon0 = (channels as u16) << 8 | 0x96;
            blitter.bltcon1 = fill;
            blitter.set_dma_enabled(true);
            blitter.trigger_blit(0x0042);
            let mut ram = TestChipRam::from(vec![0; 64]);
            let mut word = sequence.to_vec();
            if fill != 0 && channels & 3 == 1 {
                word.push(b'I');
            }
            let mut expected = vec![b'I']; // Startup does not reserve Chip RAM.
            expected.extend(word.iter().map(|&p| if p == b'D' { b'I' } else { p }));
            expected.extend_from_slice(&word);
            if channels & 1 != 0 {
                expected.push(b'D'); // Final pending result drains before IRQ.
            }
            for (cck, phase) in expected.iter().copied().enumerate() {
                assert!(blitter.is_busy);
                assert!(!blitter.blit_irq);
                assert_eq!(
                    phase_code(blitter.current_phase()),
                    phase,
                    "channels={channels:x}, fill={fill:x}, cck={cck}"
                );
                let transfer = phase != b'I';
                assert_eq!(blitter.wants_bus(), transfer);
                blitter.set_dma_enabled(false);
                let paused = blitter.clone();
                ram.next_cck();
                blitter.step_cck(&mut ram, true);
                assert_eq!(blitter, paused);
                blitter.set_dma_enabled(true);
                if transfer {
                    let waiting = blitter.clone();
                    let memory = ram.clone();
                    ram.next_cck();
                    blitter.step_cck(&mut ram, false);
                    assert_eq!(blitter, waiting);
                    assert_eq!(ram, memory);
                    ram.next_cck();
                    blitter.step_cck(&mut ram, false);
                    assert_eq!(blitter, waiting);
                }
                ram.next_cck();
                blitter.step_cck(&mut ram, transfer);
            }
            assert!(!blitter.is_busy);
            assert!(blitter.poll_blit_irq());
            assert!(!blitter.poll_blit_irq());
            assert_eq!(blitter.current_phase(), BlitterPhase::Idle);
            assert!(!blitter.wants_bus());
        }
    }
}

#[test]
fn phase_selection_observes_current_controls_during_an_active_word() {
    let mut blitter = Blitter::new();
    blitter.bltcon0 = 0x0f96;
    blitter.set_dma_enabled(true);
    blitter.trigger_blit(0x0042);
    let mut ram = TestChipRam::from(vec![0; 64]);
    ram.next_cck();
    blitter.step_cck(&mut ram, false);
    ram.next_cck();
    blitter.step_cck(&mut ram, true); // A consumed; next phase is B.
    assert_eq!(blitter.current_phase(), BlitterPhase::FetchB);

    blitter.bltcon0 = 0x0b96; // A/C/D: current index now selects C.
    assert_eq!(blitter.current_phase(), BlitterPhase::FetchC);
    ram.next_cck();
    blitter.step_cck(&mut ram, true);
    blitter.bltcon0 = 0x0996; // A/D: clamp index 2 to the final phase.
    assert_eq!(blitter.current_phase(), BlitterPhase::BusIdle);
    blitter.bltcon1 = 0x08; // Fill extends this word with an internal phase.
    ram.next_cck();
    blitter.step_cck(&mut ram, false);
    assert_eq!((blitter.phase_index, blitter.cur_col), (0, 1));
    assert_eq!(blitter.current_phase(), BlitterPhase::FetchA);
    ram.next_cck();
    blitter.step_cck(&mut ram, true);
    assert_eq!(blitter.current_phase(), BlitterPhase::WriteD);
    ram.next_cck();
    blitter.step_cck(&mut ram, true);
    assert_eq!(blitter.current_phase(), BlitterPhase::BusIdle);
    ram.next_cck();
    blitter.step_cck(&mut ram, false);
    assert_eq!(blitter.current_phase(), BlitterPhase::WriteD);
    ram.next_cck();
    blitter.step_cck(&mut ram, true);
    assert!(!blitter.is_busy);
    assert!(blitter.poll_blit_irq());
}
