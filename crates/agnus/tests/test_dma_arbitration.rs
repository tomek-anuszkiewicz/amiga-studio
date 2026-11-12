use agnus::dma::{DmaChannel, DmaScheduler};

#[test]
fn short_and_long_lines_reserve_four_refresh_opportunities() {
    // HRM 6 allocates four refresh cycles. The final model slot moves with LOL.
    for line_ccks in [227, 228] {
        let mut dma = DmaScheduler::new();
        let mut slots = Vec::new();
        for hpos in 0..line_ccks {
            if dma.step_cck(
                hpos, 100, false, [false; 4], [false; 8], false, false, true, line_ccks,
            ) == DmaChannel::Refresh
            {
                slots.push(hpos);
            }
        }
        assert_eq!(slots, [1, 3, 5, line_ccks - 1]);
    }
}

#[test]
fn copper_uses_only_its_memory_phase() {
    // HRM 2 Introduction: alternate memory opportunities, avoiding fixed DMA.
    // In the existing HPOS convention fixed slots are odd and Copper is even.
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8280);
    assert_eq!(
        dma.step_cck(0x40, 100, false, [false; 4], [false; 8], true, false, true, 227),
        DmaChannel::Copper
    );
    assert_eq!(
        dma.step_cck(0x41, 100, false, [false; 4], [false; 8], true, false, true, 227),
        DmaChannel::Cpu
    );
}

#[test]
fn unused_fixed_opportunities_do_not_block_a_requesting_cpu() {
    // HRM 6, Blitter Operations and System DMA: unused slots are available.
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x821F);
    for hpos in [7, 9, 11, 13, 15, 17, 19] {
        assert_eq!(
            dma.step_cck(hpos, 100, false, [false; 4], [false; 8], false, false, true, 227),
            DmaChannel::Cpu
        );
        assert!(!dma.chip_ram_blocked);
    }
}

#[test]
fn normal_blitter_yield_counts_unsatisfied_cpu_requests_across_other_dma() {
    // HRM 6: three unsatisfied CPU requests, not three Blitter grants.
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8240);
    for hpos in [1, 2, 3] {
        assert_ne!(
            dma.step_cck(hpos, 100, false, [false; 4], [false; 8], false, true, true, 227),
            DmaChannel::Cpu
        );
    }
    assert_eq!(
        dma.step_cck(4, 100, false, [false; 4], [false; 8], false, true, true, 227),
        DmaChannel::Cpu
    );
    assert_eq!(dma.cpu_starvation_counter, 0);
}

#[test]
fn absent_cpu_request_cancels_the_consecutive_denial_count() {
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8240);
    dma.step_cck(
        0x40, 100, false, [false; 4], [false; 8], false, true, true, 227,
    );
    dma.step_cck(
        0x41, 100, false, [false; 4], [false; 8], false, true, false, 227,
    );
    assert_eq!(dma.cpu_starvation_counter, 0);
}

#[test]
fn fetch_sequences_match_the_two_book_diagrams() {
    // HRM 6, Figures 6-11 and 6-12. Plane numbers below are one-based.
    for (control, sequence, maximum) in [
        (0x0000, [0, 4, 6, 2, 0, 3, 5, 1], 6),
        (0x8000, [4, 2, 3, 1, 4, 2, 3, 1], 4),
    ] {
        for count in 0..=maximum {
            let mut dma = DmaScheduler::new();
            dma.write_dmacon(0x8300);
            dma.set_bplcon0(control | (count << 12));
            for (offset, plane) in sequence.into_iter().enumerate() {
                let expected = if plane != 0 && plane <= count {
                    Some((plane - 1) as u8)
                } else {
                    None
                };
                assert_eq!(
                    dma.bitplane_channel_for_slot(0x38 + offset as u16, 100),
                    expected
                );
            }
        }
    }
}

#[test]
fn final_fetch_block_contains_the_stop_comparison() {
    let mut dma = DmaScheduler::new();
    dma.set_ddfstrt(0x38);
    dma.set_ddfstop(0x3C);
    assert!(dma.is_last_bpl_block(0x3F));
    assert!(!dma.is_in_ddf_window(0x40));
}

#[test]
fn standard_display_fetches_twenty_lores_or_forty_hires_words_per_plane() {
    // HRM 3, Telling the System How to Fetch and Display Data and Modulo.
    for (control, start, stop, words) in [(0x6000, 0x38, 0xD0, 20), (0xC000, 0x3C, 0xD4, 40)] {
        let mut dma = DmaScheduler::new();
        dma.write_dmacon(0x8300);
        dma.set_bplcon0(control);
        dma.set_ddfstrt(start);
        dma.set_ddfstop(stop);
        let mut counts = [0; 6];
        let mut modulos = [0; 6];
        for hpos in 0..227 {
            if let Some(plane) = dma.bitplane_channel_for_slot(hpos, 100) {
                counts[usize::from(plane)] += 1;
                if dma.is_last_bpl_block(hpos) {
                    modulos[usize::from(plane)] += 1;
                }
            }
        }
        for plane in 0..usize::from(dma.planecount()) {
            assert_eq!(counts[plane], words);
            assert_eq!(modulos[plane], 1);
        }
    }
}
