use agnus::{Agnus, AgnusModel};

fn line(a: &mut Agnus, ram: &mut [u8], vertical: u16) -> Vec<(u16, u16)> {
    a.vpos = vertical;
    a.hpos = 0;
    let mut output = Vec::new();
    for _ in 0..53 {
        a.step_cck_ram(ram);
        if let Some((register, word)) = a.pending_dma_write.take() {
            let sprite = usize::from((register - 0x140) / 8);
            match (register - 0x140) % 8 {
                0 => assert!(a.set_sprpos(sprite, word)),
                2 => assert!(a.set_sprctl(sprite, word)),
                _ => {}
            }
            output.push((register, word));
        }
    }
    output
}

#[test]
fn all_eight_channels_fetch_controls_data_reuse_and_terminator() {
    for (model, initial_line) in [(AgnusModel::OcsPal8371, 25), (AgnusModel::OcsNtsc8370, 19)] {
        let mut a = Agnus::new(model);
        a.dma.write_dmacon(0x8220);
        let mut ram = [0; 1024];
        for sprite in 0..8 {
            a.sprpt[sprite] = sprite as u32 * 64;
            let words = [
                0x6440, 0x6600, 0x1111, 0x2222, 0x3333, 0x4444, 0x6840, 0x6900, 0x5555, 0x6666, 0,
                0,
            ];
            for (index, word) in words.into_iter().enumerate() {
                ram[sprite * 64 + index * 2..sprite * 64 + index * 2 + 2]
                    .copy_from_slice(&u16::to_be_bytes(word));
            }
        }
        assert!(line(&mut a, &mut ram, initial_line - 1).is_empty());
        for (vertical, register_offset, words) in [
            (initial_line, 0, [0x6440, 0x6600]),
            (100, 4, [0x1111, 0x2222]),
            (101, 4, [0x3333, 0x4444]),
            (102, 0, [0x6840, 0x6900]),
            (104, 4, [0x5555, 0x6666]),
            (105, 0, [0, 0]),
        ] {
            if vertical == 100 || vertical == 104 {
                assert!(line(&mut a, &mut ram, vertical - 1).is_empty());
            }
            let received = line(&mut a, &mut ram, vertical);
            let expected: Vec<_> = (0..8)
                .flat_map(|sprite| {
                    [
                        (0x140 + sprite * 8 + register_offset, words[0]),
                        (0x142 + sprite * 8 + register_offset, words[1]),
                    ]
                })
                .collect();
            assert_eq!(received, expected);
        }
        assert!(line(&mut a, &mut ram, 106).is_empty());
        for sprite in 0..8 {
            assert_eq!(a.sprpt[sprite], sprite as u32 * 64 + 24);
        }
    }
}

#[test]
fn nine_bit_positions_disable_and_retained_output_control_memory_advancement() {
    let mut a = Agnus::new(AgnusModel::OcsPal8371);
    a.sprpos[0] = 0x2c40;
    a.sprctl[0] = 0x2e06;
    let mut ram = [0x12; 64];
    assert!(line(&mut a, &mut ram, 299).is_empty());
    assert!(line(&mut a, &mut ram, 300).is_empty()); // DMA disabled, comparator still active.
    assert_eq!(a.sprpt[0], 0);
    a.dma.write_dmacon(0x8220);
    a.vpos = 301;
    a.hpos = 20;
    a.step_cck_ram(&mut ram);
    assert_eq!(a.pending_dma_write, Some((0x144, 0x1212)));
    assert_eq!(a.sprpt[0], 2);
    a.step_cck_ram(&mut ram);
    a.step_cck_ram(&mut ram); // Second opportunity cannot overwrite pending output.
    assert_eq!(a.pending_dma_write, Some((0x144, 0x1212)));
    assert_eq!(a.sprpt[0], 2);
    assert!(!a.chip_ram_blocked);
    a.pending_dma_write = None;
    assert_eq!(
        line(&mut a, &mut ram, 302),
        [(0x140, 0x1212), (0x142, 0x1212)]
    );
    assert_eq!(a.sprpt[0], 6);
}

#[test]
fn overlapping_bitplane_grant_prevents_sprite_fetch_and_pointer_increment() {
    let mut a = Agnus::new(AgnusModel::OcsPal8371);
    a.vpos = 100;
    a.hpos = 20;
    a.sprpos[0] = 0x6440;
    a.sprctl[0] = 0x6600;
    a.dma.write_dmacon(0x8320);
    a.dma.set_bplcon0(0xc000);
    a.dma.set_ddfstrt(20);
    a.dma.set_ddfstop(28);
    let mut ram = [0; 64];
    a.step_cck_ram(&mut ram);
    assert!(a.pending_bpl_dma.is_some());
    assert_eq!(a.pending_dma_write, None);
    assert_eq!(a.sprpt[0], 0);
}
