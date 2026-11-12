use agnus::blitter::{apply_fill, eval_minterm, Blitter, BlitterPhase};

fn put(ram: &mut [u8], address: usize, word: u16) {
    ram[address..address + 2].copy_from_slice(&word.to_be_bytes());
}

fn get(ram: &[u8], address: usize) -> u16 {
    u16::from_be_bytes([ram[address], ram[address + 1]])
}

fn configured(control: u16) -> Blitter {
    let mut b = Blitter::new();
    b.bltcon0 = control;
    b.bltafwm = 0xffff;
    b.bltalwm = 0xffff;
    b.set_dma_enabled(true);
    b
}

fn finish(b: &mut Blitter, ram: &mut [u8]) {
    for _ in 0..256 {
        b.step_cck_ram(ram, true);
        if !b.is_busy {
            return;
        }
    }
    assert!(!b.is_busy, "operation did not finish");
}

#[test]
fn all_256_truth_tables_select_the_eight_source_inputs() {
    for table in 0..=255u8 {
        assert_eq!(
            eval_minterm(0xf0f0, 0xcccc, 0xaaaa, table),
            u16::from(table) * 0x0101
        );
    }
}

#[test]
fn all_channel_combinations_use_dma_or_held_source_and_compute_zero_without_d() {
    for channels in 0..16u16 {
        let mut b = configured((channels << 8) | 0x96); // A XOR B XOR C
        let mut ram = [0u8; 256];
        b.bltapt = 0;
        b.bltbpt = 32;
        b.bltcpt = 64;
        b.bltdpt = 128;
        b.bltadat = 0x0f0f;
        b.write_bltbdat(0x3333);
        b.bltcdat = 0x5555;
        let a = [0x1234, 0x5678];
        let b_words = [0x9abc, 0xdef0];
        let c = [0x1010, 0x2020];
        for i in 0..2 {
            put(&mut ram, i * 2, a[i]);
            put(&mut ram, 32 + i * 2, b_words[i]);
            put(&mut ram, 64 + i * 2, c[i]);
        }
        b.trigger_blit(0x0042);
        finish(&mut b, &mut ram);
        for i in 0..2 {
            let result = (if channels & 8 != 0 { a[i] } else { 0x0f0f })
                ^ (if channels & 4 != 0 {
                    b_words[i]
                } else {
                    0x3333
                })
                ^ (if channels & 2 != 0 { c[i] } else { 0x5555 });
            if channels & 1 != 0 {
                assert_eq!(get(&ram, 128 + i * 2), result, "channels {channels:x}");
            }
        }
        assert_eq!(b.bltapt, if channels & 8 != 0 { 4 } else { 0 });
        assert_eq!(b.bltbpt, if channels & 4 != 0 { 36 } else { 32 });
        assert_eq!(b.bltcpt, if channels & 2 != 0 { 68 } else { 64 });
        assert_eq!(b.bltdpt, if channels & 1 != 0 { 132 } else { 128 });
        assert!(!b.is_zero);
        assert!(b.poll_blit_irq());
        assert!(!b.poll_blit_irq());
        b.bltcon0 &= 0xff00; // Constant zero, regardless of enabled sources.
        b.trigger_blit(0x0041);
        finish(&mut b, &mut ram);
        assert!(b.is_zero);
    }
}

#[test]
fn next_sources_are_read_before_previous_destination_in_overlapping_copy() {
    let mut b = configured(0x09f0);
    b.bltdpt = 2;
    let mut ram = [0; 32];
    for (i, word) in [0x1234, 0x5678, 0x9abc, 0xdef0].into_iter().enumerate() {
        put(&mut ram, i * 2, word);
    }
    b.trigger_blit(0x0044);
    finish(&mut b, &mut ram);
    assert_eq!(
        [get(&ram, 2), get(&ram, 4), get(&ram, 6), get(&ram, 8)],
        [0x1234, 0x5678, 0x9abc, 0xdef0]
    );
    assert_eq!(b.bltdpt, 10);
}

#[test]
fn masks_precede_all_shifts_and_source_carry_survives_row_change() {
    for descending in [false, true] {
        for shift in 0..16 {
            let mut b = configured(0x09f0 | (shift << 12));
            b.bltcon1 = if descending { 2 } else { 0 };
            b.bltafwm = 0xfff0;
            b.bltalwm = 0x0fff;
            b.bltapt = 32;
            b.bltdpt = 128;
            b.bltamod = -2; // One-word rows revisit the same masked input.
            let mut ram = [0; 256];
            put(&mut ram, 32, 0xabcd);
            b.trigger_blit(0x0081);
            finish(&mut b, &mut ram);
            let first = if descending {
                0x0bc0u32 << shift
            } else {
                0x0bc0u32 >> shift
            } as u16;
            let second = if descending {
                0x0bc0_0bc0u32 >> (16 - shift)
            } else {
                0x0bc0_0bc0u32 >> shift
            } as u16;
            assert_eq!(
                get(&ram, 128),
                first,
                "descending={descending}, shift={shift}"
            );
            assert_eq!(get(&ram, if descending { 126 } else { 130 }), second);
            assert_eq!(b.bltapt, 32);
            assert_eq!(b.bltdpt, if descending { 124 } else { 132 });
        }
    }
}

#[test]
fn cpu_b_data_write_captures_shift_before_blit_and_disabled_b_reuses_it() {
    let mut b = configured(0x01cc);
    b.bltcon1 = 4 << 12;
    b.write_bltbdat(0x1234);
    b.bltcon1 = 0;
    let mut ram = [0; 16];
    b.trigger_blit(0x0042);
    finish(&mut b, &mut ram);
    assert_eq!([get(&ram, 0), get(&ram, 2)], [0x0123; 2]);
}

#[test]
fn fill_matches_manual_boundaries_and_reloads_carry_per_row() {
    for (exclusive, initial, expected) in [
        (false, false, 0x3c18),
        (true, false, 0x1c08),
        (false, true, 0xe7ff),
        (true, true, 0xe3f7),
    ] {
        let mut carry = initial;
        assert_eq!(apply_fill(0x2418, &mut carry, exclusive), expected);
    }
    for (mode, expected) in [(0x0008, 0xffff), (0x0010, 0x7fff)] {
        let mut b = configured(0x09f0);
        b.bltcon1 = mode | 2;
        b.bltapt = 6;
        b.bltdpt = 22;
        let mut ram = [0u8; 32];
        // Descending order: one boundary starts carry; next row resets it.
        put(&mut ram, 6, 0x8000);
        put(&mut ram, 2, 0x8000);
        b.trigger_blit(0x0082);
        finish(&mut b, &mut ram);
        assert_eq!(get(&ram, 20), 0xffff);
        assert_eq!(get(&ram, 16), 0xffff);
        // Carry is clear at the beginning of each row.
        assert_eq!(get(&ram, 22), 0x8000);
        assert_eq!(get(&ram, 18), 0x8000);
        b.bltcon1 = mode | 2 | 4;
        b.bltapt = 6;
        b.bltdpt = 22;
        b.trigger_blit(0x0041);
        finish(&mut b, &mut ram);
        assert_eq!(get(&ram, 22), expected);
    }
}

#[test]
fn denied_grants_and_dma_disable_preserve_the_pending_operation() {
    let mut b = configured(0x09f0);
    b.bltapt = 0;
    b.bltdpt = 8;
    let mut ram = [0; 16];
    put(&mut ram, 0, 0x1234);
    b.trigger_blit(0x0041);
    b.step_cck_ram(&mut ram, false); // Internal startup.
    assert!(b.wants_bus());
    let waiting = b.clone();
    b.step_cck_ram(&mut ram, false);
    assert_eq!(b, waiting);
    for _ in 0..8 {
        if b.current_phase() == BlitterPhase::WriteD {
            break;
        }
        b.step_cck_ram(&mut ram, true);
    }
    assert_eq!(b.current_phase(), BlitterPhase::WriteD);
    assert_eq!(get(&ram, 8), 0);
    assert!(b.is_busy);
    assert!(!b.poll_blit_irq());
    b.set_dma_enabled(false);
    let paused = b.clone();
    b.step_cck_ram(&mut ram, true);
    assert_eq!(b, paused);
    b.set_dma_enabled(true);
    b.step_cck_ram(&mut ram, false);
    assert_eq!(get(&ram, 8), 0);
    b.step_cck_ram(&mut ram, true);
    assert_eq!(get(&ram, 8), 0x1234);
    assert!(!b.is_busy);
    assert!(b.poll_blit_irq());
    b.step_cck_ram(&mut ram, true);
    assert!(!b.poll_blit_irq());
    b.trigger_blit(0);
    assert_eq!((b.total_cols, b.total_rows), (64, 1024));
    b.reset();
    assert!(!b.is_busy);
    assert!(!b.wants_bus());
    assert!(!b.poll_blit_irq());
}

#[test]
fn zero_encoded_area_size_completes_all_65536_destination_words() {
    let mut blitter = configured(0x01f0);
    blitter.bltadat = 0xa55a;
    blitter.trigger_blit(0);
    let mut ram = vec![0; 0x20000];
    let mut transfers = 0;
    for _ in 0..300_000 {
        let writes = blitter.current_phase() == BlitterPhase::WriteD;
        blitter.step_cck_ram(&mut ram, true);
        transfers += usize::from(writes);
        if !blitter.is_busy {
            break;
        }
    }
    assert!(!blitter.is_busy);
    assert_eq!(transfers, 65536);
    assert_eq!(blitter.bltdpt, 0x20000);
    assert!(ram.chunks_exact(2).all(|word| word == [0xa5, 0x5a]));
    assert!(blitter.poll_blit_irq());
    assert!(!blitter.poll_blit_irq());
}
