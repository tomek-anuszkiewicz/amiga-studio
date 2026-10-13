use blitter::{Blitter, LineDrawer};

#[test]
fn area_source_pointer_wraps_within_512k_chip_ram() {
    let mut blitter = Blitter::new();
    let mut chip_ram = vec![0; 512 * 1024];
    chip_ram[0x7FFFE..0x80000].copy_from_slice(&0x1234u16.to_be_bytes());
    chip_ram[0..2].copy_from_slice(&0x5678u16.to_be_bytes());

    blitter.bltcon0 = 0x0800;
    blitter.bltapt = 0x0007_FFFE;
    blitter.set_dma_enabled(true);
    blitter.trigger_blit((1 << 6) | 2);
    for _ in 0..16 {
        blitter.step_cck_ram(&mut chip_ram);
    }

    assert!(!blitter.is_busy);
    assert_eq!(blitter.anew, 0x5678);
    assert_eq!(blitter.bltapt, 0x0000_0002);
}

#[test]
fn line_destination_pointer_wraps_within_512k_chip_ram() {
    let mut drawer = LineDrawer::new();
    let mut bltcon0 = 0;
    let mut bltcon1 = 0;
    let mut bltapt = 0;
    let mut bltcpt = 0x0007_FFFE;

    drawer.step_pixel(
        &mut bltcon0,
        &mut bltcon1,
        &mut bltapt,
        0,
        0,
        2,
        &mut bltcpt,
    );

    assert_eq!(bltcpt, 0);
}
