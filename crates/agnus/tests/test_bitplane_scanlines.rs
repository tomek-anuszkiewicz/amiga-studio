mod common;

use agnus::{Agnus, AgnusModel};

fn assert_display_rows(control: u16, start: u16, words: usize, final_slots: [u16; 2]) {
    // Captured register setups: Kickstart's disk prompt is lores; Workbench is hires.
    // HRM 3 requires 40/80 bytes per plane per row. STOP requests a final full
    // eight-CCK group, as in vAmiga's bitplane sequencer.
    for modulos in [[0i16, 0i16], [6, -4]] {
        let mut chip = common::TestChipRam::default();
        let mut a = Agnus::new(AgnusModel::OcsPal8371);
        a.dma.write_dmacon(0x8300);
        a.dma.set_bplcon0(control);
        a.dma.set_ddfstrt(start);
        a.dma.set_ddfstop(0xD0);
        a.dma.set_vstart(100);
        a.dma.set_vstop(102);
        a.bpl1mod = modulos[0];
        a.bpl2mod = modulos[1];
        let bases = [0x1000u32, 0x4000];
        a.bplpt[..2].copy_from_slice(&bases);
        for (plane, base) in bases.into_iter().enumerate() {
            for word in 0..words * 3 {
                let offset = base as usize + word * 2;
                let value = ((plane as u16 + 1) << 12) | word as u16;
                chip[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
            }
        }
        a.hpos = 226;
        a.vpos = 99;
        let mut counts = [[0usize; 2]; 2];
        let mut last_slots = [[0u16; 2]; 2];
        for _ in 0..227 * 3 {
            chip.next_cck();
            a.step_cck(&mut chip);
            if let Some((plane, value)) = a.poll_bpl_dma() {
                assert!((100..102).contains(&a.vpos));
                let row = usize::from(a.vpos - 100);
                let plane = usize::from(plane);
                assert!(plane < 2);
                let stride = (words * 2) as i32 + i32::from(modulos[plane]);
                let address =
                    bases[plane] + (row as i32 * stride) as u32 + counts[row][plane] as u32 * 2;
                let offset = address as usize;
                assert_eq!(value, u16::from_be_bytes([chip[offset], chip[offset + 1]]));
                counts[row][plane] += 1;
                last_slots[row][plane] = a.hpos;
                let modulo = if counts[row][plane] == words {
                    i32::from(modulos[plane])
                } else {
                    0
                };
                assert_eq!(a.bplpt[plane], (address as i32 + 2 + modulo) as u32);
            }
        }
        assert_eq!(counts, [[words; 2]; 2]);
        assert_eq!(last_slots, [final_slots; 2]);
        for plane in 0..2 {
            let stride = (words * 2) as i32 + i32::from(modulos[plane]);
            assert_eq!(a.bplpt[plane], bases[plane] + (2 * stride) as u32);
        }
    }
}

#[test]
fn kickstart_disk_prompt_fetches_complete_forty_byte_rows() {
    assert_display_rows(0x2302, 0x38, 20, [0xD7, 0xD3]);
}

#[test]
fn workbench_fetches_complete_eighty_byte_rows_after_a_mid_group_stop() {
    assert_display_rows(0xA302, 0x3C, 40, [0xDB, 0xD9]);
}
