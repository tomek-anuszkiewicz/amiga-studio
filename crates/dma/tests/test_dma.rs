use dma::{DmaChannel, DmaScheduler};

#[test]
fn test_dmacon_set_and_clear() {
    let mut dma = DmaScheduler::new();
    assert_eq!(dma.dmacon, 0);

    // SET master DMA + Audio 0 + Copper: $8000 | 0x0200 | 0x0001 | 0x0080 = $8281
    dma.write_dmacon(0x8281);
    assert_eq!(dma.dmacon, 0x0281);
    assert!(dma.is_dma_enabled());
    assert!(dma.is_channel_enabled(DmaChannel::Audio(0)));
    assert!(dma.is_channel_enabled(DmaChannel::Copper));
    assert!(!dma.is_channel_enabled(DmaChannel::Disk));

    // CLR Copper: bit 15 = 0, bit 7 = 1
    dma.write_dmacon(0x0080);
    assert_eq!(dma.dmacon, 0x0201);
    assert!(!dma.is_channel_enabled(DmaChannel::Copper));
}

#[test]
fn test_chip_ram_contention() {
    let mut dma = DmaScheduler::new();
    // Enable DMA + Disk
    dma.write_dmacon(0x8210);

    // HPOS 4 is Disk slot
    assert!(dma.is_chip_ram_blocked(4, false));
    // HPOS 10 is free slot
    assert!(!dma.is_chip_ram_blocked(10, false));

    // Blitter Nasty test
    dma.write_dmacon(0x8440); // SET BLTPRI (bit 10) + BLTEN (bit 6)
    assert!(dma.is_chip_ram_blocked(10, true));
}
