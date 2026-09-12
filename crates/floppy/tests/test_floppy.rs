use floppy::FloppyController;

#[test]
fn test_floppy_geometry_and_stepping() {
    let mut controller = FloppyController::new();
    assert_eq!(controller.dsksyn, 0x4489);

    let df0 = &mut controller.drives[0];
    assert!(df0.is_track0());

    df0.step(true); // Step inward to cylinder 1
    assert_eq!(df0.cylinder, 1);
    assert!(!df0.is_track0());

    df0.step(false); // Step outward to cylinder 0
    assert_eq!(df0.cylinder, 0);
    assert!(df0.is_track0());
}

#[test]
fn test_dsklen_modes() {
    let mut controller = FloppyController::new();
    controller.dsklen = 0xC100; // Bit 15 = DMA enable, Bit 14 = Write mode
    assert!(controller.is_dma_enabled());
    assert!(controller.is_write_mode());
}
