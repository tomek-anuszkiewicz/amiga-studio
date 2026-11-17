mod common;

use agnus::dma::{DmaChannel, DmaScheduler};

#[test]
fn short_and_long_lines_reserve_four_refresh_opportunities() {
    // HRM 6 allocates four refresh cycles. The final model slot moves with LOL.
    for line_ccks in [227, 228] {
        let mut dma = DmaScheduler::new();
        let mut slots = Vec::new();
        for hpos in 0..line_ccks {
            if dma.step_cck(
                hpos, 100, false, [false; 4], 0, false, false, true, line_ccks,
            ) == DmaChannel::Refresh
            {
                slots.push(hpos);
            }
        }
        assert_eq!(slots, [1, 3, 5, line_ccks - 1]);
    }
}

#[test]
fn fixed_opportunities_require_current_requests_and_channel_enables() {
    // HRM fixed opportunities in the scheduler's existing horizontal convention.
    for line_ccks in [227, 228] {
        for enabled in [
            0, 0x003f, 0x0200, 0x0201, 0x0202, 0x0204, 0x0208, 0x0210, 0x0220, 0x023f,
        ] {
            for requested in [false, true] {
                let mut dma = DmaScheduler::new();
                dma.write_dmacon(0x8000 | enabled);
                for hpos in 0..line_ccks {
                    let fixed = match hpos {
                        1 | 3 | 5 => DmaChannel::Refresh,
                        h if h == line_ccks - 1 => DmaChannel::Refresh,
                        7 | 9 | 11 if enabled & 0x0210 == 0x0210 && requested => DmaChannel::Disk,
                        13 | 15 | 17 | 19 if enabled & 0x0200 != 0 && requested => {
                            let channel = ((hpos - 13) / 2) as u8;
                            if enabled & (1 << channel) != 0 {
                                DmaChannel::Audio(channel)
                            } else {
                                DmaChannel::Cpu
                            }
                        }
                        21..=51 if hpos % 2 == 1 && enabled & 0x0220 == 0x0220 && requested => {
                            DmaChannel::Sprite(((hpos - 21) / 4) as u8)
                        }
                        _ => DmaChannel::Cpu,
                    };
                    assert_eq!(
                        dma.step_cck(
                            hpos,
                            100,
                            requested,
                            [requested; 4],
                            if requested { 0xff } else { 0 },
                            false,
                            false,
                            true,
                            line_ccks
                        ),
                        fixed,
                        "line={line_ccks}, enable={enabled:x}, requested={requested}, hpos={hpos}"
                    );
                    assert_eq!(dma.chip_ram_blocked, fixed != DmaChannel::Cpu);
                }
            }
        }
    }
}

#[test]
fn a_fixed_sprite_opportunity_keeps_live_display_priority() {
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8320);
    dma.set_bplcon0(0xc000);
    dma.set_ddfstrt(20);
    dma.set_ddfstop(28);
    assert_eq!(
        dma.step_cck(21, 100, false, [false; 4], 0xff, false, false, true, 227),
        DmaChannel::Bitplane(1)
    );
    dma.write_dmacon(0x0100); // Stop bitplane DMA without changing the opportunity.
    assert_eq!(
        dma.step_cck(21, 100, false, [false; 4], 0xff, false, false, true, 227),
        DmaChannel::Sprite(0)
    );
    dma.write_dmacon(0x0020);
    assert_eq!(
        dma.step_cck(21, 100, false, [false; 4], 0xff, false, false, true, 227),
        DmaChannel::Cpu
    );
}

#[test]
fn copper_uses_only_its_memory_phase() {
    // HRM 2 Introduction: alternate memory opportunities, avoiding fixed DMA.
    // In the existing HPOS convention fixed slots are odd and Copper is even.
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8280);
    assert_eq!(
        dma.step_cck(0x40, 100, false, [false; 4], 0, true, false, true, 227),
        DmaChannel::Copper
    );
    assert_eq!(
        dma.step_cck(0x41, 100, false, [false; 4], 0, true, false, true, 227),
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
            dma.step_cck(hpos, 100, false, [false; 4], 0, false, false, true, 227),
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
            dma.step_cck(hpos, 100, false, [false; 4], 0, false, true, true, 227),
            DmaChannel::Cpu
        );
    }
    assert_eq!(
        dma.step_cck(4, 100, false, [false; 4], 0, false, true, true, 227),
        DmaChannel::Cpu
    );
    assert_eq!(dma.cpu_starvation_counter, 0);
}

#[test]
fn absent_cpu_request_cancels_the_consecutive_denial_count() {
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8240);
    dma.step_cck(0x40, 100, false, [false; 4], 0, false, true, true, 227);
    dma.step_cck(0x41, 100, false, [false; 4], 0, false, true, false, 227);
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
fn a_mid_group_stop_requests_one_final_complete_fetch_group() {
    let mut dma = DmaScheduler::new();
    dma.set_ddfstrt(0x38);
    dma.set_ddfstop(0x3C);
    assert!(!dma.is_last_bpl_block(0x3F));
    assert!(dma.is_in_ddf_window(0x40));
    assert!(dma.is_last_bpl_block(0x47));
    assert!(!dma.is_in_ddf_window(0x48));
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

#[test]
fn rejected_disk_ram_read_preserves_the_selected_request() {
    use config::chip_ram::ChipRamAccess;
    let mut ram = common::TestChipRam::default();
    ram.next_cck();
    ram.read_chip_word(0x100);
    let mut agnus = agnus::Agnus::new(agnus::AgnusModel::OcsPal8371);
    agnus.hpos = 6;
    agnus.dskpt = 0x100;
    agnus.disk_dma_requests[0] = true;
    agnus.disk_dma_write[0] = true;
    agnus.dma.write_dmacon(0x8210);
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        agnus.step_cck(&mut ram);
    }));
    assert!(failure.is_err());
    assert!(
        agnus.disk_dma_requests[0],
        "Rejected RAM admission consumed the disk request"
    );
    assert_eq!(agnus.dskpt, 0x100);
    assert!(agnus.pending_dma_write.is_none());
}
