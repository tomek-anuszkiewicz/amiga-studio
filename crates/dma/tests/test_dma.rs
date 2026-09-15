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
fn test_fixed_slots_schedule_and_dynamic_release() {
    let mut dma = DmaScheduler::new();
    // DMA disabled: Refresh must STILL be active and block Chip RAM (unconditional)
    for hpos in 0..=3 {
        let owner = dma.arbitrate(hpos, 100, false, [false; 4], false, false, true);
        assert_eq!(owner, DmaChannel::Refresh);
        assert!(dma.chip_ram_blocked);
    }

    // Enable DMA + Disk + Audio + Sprites
    dma.write_dmacon(0x8000 | 0x0200 | 0x0010 | 0x000F | 0x0020);

    // Slot 4: Disk DMA when active
    let owner = dma.arbitrate(4, 100, true, [false; 4], false, false, true);
    assert_eq!(owner, DmaChannel::Disk);
    assert!(dma.chip_ram_blocked);

    // Dynamic slot release: Disk DMA idle at slot 4 -> released to CPU
    let owner = dma.arbitrate(4, 100, false, [false; 4], false, false, true);
    assert_eq!(owner, DmaChannel::Cpu);
    assert!(!dma.chip_ram_blocked);

    // Slots 5..8: Audio channels 0..3
    for ch in 0..4 {
        let mut audio_active = [false; 4];
        audio_active[ch] = true;
        let owner = dma.arbitrate(5 + ch as u16, 100, false, audio_active, false, false, true);
        assert_eq!(owner, DmaChannel::Audio(ch as u8));
        assert!(dma.chip_ram_blocked);
    }

    // Slots 12..27: Sprites 0..7
    for sprite in 0..8 {
        let hpos_a = 12 + sprite * 2;
        let hpos_b = hpos_a + 1;
        assert_eq!(
            dma.arbitrate(hpos_a, 100, false, [false; 4], false, false, true),
            DmaChannel::Sprite(sprite as u8)
        );
        assert_eq!(
            dma.arbitrate(hpos_b, 100, false, [false; 4], false, false, true),
            DmaChannel::Sprite(sprite as u8)
        );
    }

    // Dynamic release: Sprites disabled in DMACON -> slot 12 released to CPU
    dma.write_dmacon(0x0020); // CLR SPREN
    let owner = dma.arbitrate(12, 100, false, [false; 4], false, false, true);
    assert_eq!(owner, DmaChannel::Cpu);
    assert!(!dma.chip_ram_blocked);
}

#[test]
fn test_bitplane_lores_allocation_and_cycle_stealing() {
    let mut dma = DmaScheduler::new();
    // Enable DMA + Bitplane DMA: $8000 | 0x0200 | 0x0100 = $8300
    dma.write_dmacon(0x8300);
    dma.set_ddfstrt(0x0038);
    dma.set_ddfstop(0x00D0);

    // Test 4 planes in LoRes (BPLCON0 = 0x4000)
    dma.set_bplcon0(0x4000);
    assert_eq!(dma.planecount(), 4);
    assert!(!dma.is_hires());

    // Within active scanline 100 and DDF window (0x38..=0xD0)
    // Even slots (phases 0, 2, 4, 6) must be claimed by planes 0..3
    let base = 0x0038;
    assert_eq!(dma.bitplane_channel_for_slot(base + 0, 100), Some(0));
    assert_eq!(dma.bitplane_channel_for_slot(base + 2, 100), Some(1));
    assert_eq!(dma.bitplane_channel_for_slot(base + 4, 100), Some(2));
    assert_eq!(dma.bitplane_channel_for_slot(base + 6, 100), Some(3));

    // Odd slots (phases 1, 3, 5, 7) MUST remain completely free for CPU
    assert_eq!(dma.bitplane_channel_for_slot(base + 1, 100), None);
    assert_eq!(dma.bitplane_channel_for_slot(base + 3, 100), None);
    assert_eq!(dma.bitplane_channel_for_slot(base + 5, 100), None);
    assert_eq!(dma.bitplane_channel_for_slot(base + 7, 100), None);

    // Arbitrate odd slot with 4 planes: awards to CPU with zero wait states
    let owner = dma.arbitrate(base + 1, 100, false, [false; 4], false, false, true);
    assert_eq!(owner, DmaChannel::Cpu);
    assert!(!dma.chip_ram_blocked);

    // Test 5 planes in LoRes (BPLCON0 = 0x5000): Plane 5 steals 25% of odd cycles (phase 1)
    dma.set_bplcon0(0x5000);
    assert_eq!(dma.planecount(), 5);
    assert_eq!(dma.bitplane_channel_for_slot(base + 1, 100), Some(4)); // Stolen!
    assert_eq!(dma.bitplane_channel_for_slot(base + 3, 100), None); // Free

    // Test 6 planes in LoRes (BPLCON0 = 0x6000): Planes 5 & 6 steal 50% of odd cycles (phases 1 & 3)
    dma.set_bplcon0(0x6000);
    assert_eq!(dma.planecount(), 6);
    assert_eq!(dma.bitplane_channel_for_slot(base + 1, 100), Some(4)); // Stolen!
    assert_eq!(dma.bitplane_channel_for_slot(base + 3, 100), Some(5)); // Stolen!
    assert_eq!(dma.bitplane_channel_for_slot(base + 5, 100), None); // Free
    assert_eq!(dma.bitplane_channel_for_slot(base + 7, 100), None); // Free
}

#[test]
fn test_bitplane_hires_allocation_and_lockout() {
    let mut dma = DmaScheduler::new();
    dma.write_dmacon(0x8300); // Enable DMA + Bitplane
    dma.set_ddfstrt(0x0038);
    dma.set_ddfstop(0x00D0);

    // HiRes 4 planes: BPLCON0 = 0x8000 (HIRES) | 0x4000 (4 planes) = 0xC000
    dma.set_bplcon0(0xC000);
    assert!(dma.is_hires());
    assert_eq!(dma.planecount(), 4);

    let base = 0x0038;
    // HiRes 4 planes consumes 100% of memory cycles across all 8 phases
    for phase in 0..8 {
        assert!(
            dma.bitplane_channel_for_slot(base + phase, 100).is_some(),
            "Phase {} in HiRes 4 planes should be claimed",
            phase
        );
        let owner = dma.arbitrate(base + phase, 100, false, [false; 4], false, false, true);
        assert!(matches!(owner, DmaChannel::Bitplane(_)));
        assert!(dma.chip_ram_blocked);
    }
}

#[test]
fn test_blitter_nasty_vs_normal_and_cpu_starvation() {
    let mut dma = DmaScheduler::new();
    // Enable DMA + Blitter: $8000 | 0x0200 | 0x0040 = $8240
    dma.write_dmacon(0x8240);

    let idle_hpos = 50; // Outside fixed slots, assuming no bitplane

    // 1. Normal Blitter Mode (BLTPRI == 0) with CPU requesting bus
    // Cycles 1, 2, 3: Blitter takes the bus, starvation counter increments
    for i in 1..=3 {
        let owner = dma.arbitrate(idle_hpos, 100, false, [false; 4], false, true, true);
        assert_eq!(owner, DmaChannel::Blitter);
        assert_eq!(dma.cpu_starvation_counter, i);
        assert!(dma.chip_ram_blocked);
    }

    // Cycle 4: CPU Starvation Yield! Arbiter forces Blitter to yield 1 cycle to CPU!
    let owner = dma.arbitrate(idle_hpos, 100, false, [false; 4], false, true, true);
    assert_eq!(owner, DmaChannel::Cpu);
    assert_eq!(dma.cpu_starvation_counter, 0);
    assert!(!dma.chip_ram_blocked);

    // 2. Blitter Nasty Mode (BLTPRI == 1, bit 10): SET $8400
    dma.write_dmacon(0x8400);
    assert!(dma.is_blitter_nasty());

    // In Blitter Nasty, Blitter NEVER yields, even after 10 cycles!
    for _ in 0..10 {
        let owner = dma.arbitrate(idle_hpos, 100, false, [false; 4], false, true, true);
        assert_eq!(owner, DmaChannel::Blitter);
        assert!(dma.chip_ram_blocked);
    }
}

#[test]
fn test_copper_bus_participation_over_blitter_and_cpu() {
    let mut dma = DmaScheduler::new();
    // Enable DMA + Copper + Blitter: $8000 | 0x0200 | 0x0080 | 0x0040 = $82C0
    dma.write_dmacon(0x82C0);

    let idle_hpos = 50;

    // Both Copper and Blitter want bus: Copper has higher priority than Blitter!
    let owner = dma.arbitrate(idle_hpos, 100, false, [false; 4], true, true, true);
    assert_eq!(owner, DmaChannel::Copper);
    assert!(dma.chip_ram_blocked);

    // Copper finished instruction fetch: Blitter gets the bus
    let owner = dma.arbitrate(idle_hpos, 100, false, [false; 4], false, true, true);
    assert_eq!(owner, DmaChannel::Blitter);
    assert!(dma.chip_ram_blocked);
}

#[test]
fn test_dma_is_in_ddf_window_block_boundary() {
    let mut dma = DmaScheduler::new();
    dma.ddfstrt = 0x68; // 104
    dma.ddfstop = 0x70; // 112

    // Low-res: 8-CCK blocks
    assert!(!dma.is_in_ddf_window(103));
    // Block 1 (104..111)
    assert!(dma.is_in_ddf_window(104));
    assert!(dma.is_in_ddf_window(111));
    // Block 2 (112..119): block_start = 112 <= ddfstop (112), so all 8 slots are inside
    assert!(dma.is_in_ddf_window(112));
    assert!(dma.is_in_ddf_window(119));
    // Block 3 (120..127): block_start = 120 > ddfstop (112), so outside
    assert!(!dma.is_in_ddf_window(120));
}
