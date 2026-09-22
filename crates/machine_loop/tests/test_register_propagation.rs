#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_cross_chip_bplcon0_broadcast_latencies() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Write BPLCON0 ($DFF100) = 0x8200 (HIRES mode) via memory bus router
    assert_eq!(
        machine.memory_bus().write_word(0xDFF100, 0x8200),
        BusResult::Ready(())
    );

    // Cycle T: write is dispatched directly into Denise & Agnus mutation pipelines
    assert_eq!(machine.denise.bplcon0, 0);
    assert_eq!(machine.agnus.bplcon0, 0);

    // Step 1 CCK (Cycle T+1):
    // - Denise mutation matures and commits!
    // - Agnus ticks: 3 CCKs remaining
    machine.step_cck();
    assert_eq!(machine.denise.bplcon0, 0x8200);
    assert_eq!(machine.agnus.bplcon0, 0);

    // Step 2 CCKs (Cycle T+2, T+3): Agnus still in flight
    machine.step_cck();
    assert_eq!(machine.agnus.bplcon0, 0);
    machine.step_cck();
    assert_eq!(machine.agnus.bplcon0, 0);

    // Step 4th CCK (Cycle T+4): Agnus mutation matures and commits!
    machine.step_cck();
    assert_eq!(machine.agnus.bplcon0, 0x8200);
}

#[test]
fn test_cross_chip_dmacon_broadcast_to_agnus_and_paula() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Write DMACON ($DFF096) = SET DMAEN (bit 9) + DSKEN (bit 4) + AUD0EN (bit 0) = 0x8211
    assert_eq!(
        machine.memory_bus().write_word(0xDFF096, 0x8211),
        BusResult::Ready(())
    );

    // Step 1 CCK: in flight (remaining = 1)
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon & 0x0200, 0);
    assert_eq!(machine.paula.dma_enables & 0x0011, 0);

    // Step 2 CCKs: commits to both Agnus and Paula!
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon & 0x0200, 0x0200);
    assert_eq!(machine.paula.dma_enables & 0x0011, 0x0011);
}

#[test]
fn test_cia_a_ovl_pin_cascade_to_memory_bus() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Initially, low memory overlay is disengaged for bare_512k test RAM (or re-engaged on cold reset)
    machine.physical_memory.map_kickstart_to_low_memory();
    assert!(machine.physical_memory.is_low_memory_overlay_active());

    // Write CIA-A Port A ($BFE001) bit 0 = 1 (Chip RAM engaged)
    machine.cia_a.write_register(0x0, 0x01);

    // Step CCK: machine loop observes CIA-A ovl_transition and disengages overlay
    machine.step_cck();
    assert!(!machine.physical_memory.is_low_memory_overlay_active());

    // Write bit 0 = 0 (Kickstart ROM overlay engaged)
    machine.cia_a.write_register(0x0, 0x00);
    machine.step_cck();
    assert!(machine.physical_memory.is_low_memory_overlay_active());
}

#[test]
fn test_paula_interrupt_cascade_to_cpu_ipl() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    assert_eq!(machine.cpu.state.ipl, 0);

    // Write INTENA ($DFF09A) = 0xC020 (SET master INTEN + Level 3 VBlank)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF09A, 0xC020),
        BusResult::Ready(())
    );
    // Write INTREQ ($DFF09C) = 0x8020 (SET Level 3 VBlank request)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF09C, 0x8020),
        BusResult::Ready(())
    );

    // Step 1 CCK: writes mature and commit in Paula, immediately updating CPU IPL!
    machine.step_cck();
    assert_eq!(machine.paula.interrupts.intena, 0x4020);
    assert_eq!(machine.paula.interrupts.intreq, 0x0020);
    assert_eq!(machine.cpu.state.ipl, 3);
}

#[test]
fn test_bus_custom_registers_snapshot_sync_and_open_bus() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Direct write to DMACON
    machine.agnus.commit_register_write(0x096, 0x8200);

    // Step CCK syncs state
    machine.step_cck();

    // Reading DMACONR ($DFF002) from bus returns 0x0200
    let dmaconr = machine.memory_bus().read_word(0xDFF002).ok().unwrap();
    assert_eq!(dmaconr & 0x0200, 0x0200);

    // Reading write-only register (e.g. BPLCON0 at $DFF100) returns 0xFFFF (open bus)
    let bplcon0_read = machine.memory_bus().read_word(0xDFF100).ok().unwrap();
    assert_eq!(bplcon0_read, 0xFFFF);
}

#[test]
fn test_write_custom_word_and_byte_methods() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Test write_custom_word directly on A500
    machine.write_custom_word(0x180, 0x0F00);
    assert_eq!(machine.denise.color[0], 0x0F00);

    // Test write_custom_byte with byte duplication
    machine.write_custom_byte(0xDFF182, 0x33);
    assert_eq!(machine.denise.color[1], 0x0333);
}

#[test]
fn test_dmacon_sync_to_denise_sprites_and_frame_builder() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Initially both sprite and frame builder DMA are disabled
    assert!(!machine.denise.sprites.dma_enabled);
    assert!(!machine.denise.frame_builder.dma_enabled);

    // Write DMACON = SET DMAEN (bit 9) + SPREN (bit 5) + BPLEN (bit 8) -> 0x8320
    machine.write_custom_word(0x096, 0x8320);

    // Step 2 CCKs for DMACON write to mature in Agnus and sync to Denise
    machine.step_cck();
    machine.step_cck();

    assert!(machine.denise.sprites.dma_enabled);
    assert!(machine.denise.frame_builder.dma_enabled);

    // Clear SPREN (bit 5)
    machine.write_custom_word(0x096, 0x0020);
    machine.step_cck();
    machine.step_cck();

    assert!(!machine.denise.sprites.dma_enabled);
    assert!(machine.denise.frame_builder.dma_enabled);

    // Direct sync_dmacon() method verification on machine loop
    machine.agnus.dmacon = 0;
    machine.sync_dmacon();
    assert!(!machine.denise.sprites.dma_enabled);
    assert!(!machine.denise.frame_builder.dma_enabled);
}
