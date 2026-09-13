use config::{A500Config, VideoStandard};
use machine_loop::memory_bus::BusResult;
use machine_loop::A500Machine;

#[test]
fn test_cross_chip_bplcon0_broadcast_latencies() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Write BPLCON0 ($DFF100) = 0x8200 (HIRES mode) via memory bus
    assert_eq!(
        machine.memory_bus.write_word(0xDFF100, 0x8200),
        BusResult::Ready(())
    );

    // Cycle T: write is queued in memory_bus pending writes
    assert_eq!(machine.denise.bplcon0, 0);
    assert_eq!(machine.agnus.bplcon0, 0);

    // Step 1 CCK (Cycle T+1):
    // - memory_bus drains and dispatches write: Denise gets 1 CCK delay, Agnus gets 4 CCK delay
    // - Denise ticks: mutation matures and commits!
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
        machine.memory_bus.write_word(0xDFF096, 0x8211),
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
    machine.memory_bus.map_kickstart_to_low_memory();
    assert!(machine.memory_bus.is_low_memory_overlay_active());

    // Write CIA-A Port A ($BFE001) bit 0 = 1 (Chip RAM engaged)
    machine.cia_a.write_register(0x0, 0x01);

    // Step CCK: machine loop observes CIA-A ovl_transition and disengages overlay
    machine.step_cck();
    assert!(!machine.memory_bus.is_low_memory_overlay_active());

    // Write bit 0 = 0 (Kickstart ROM overlay engaged)
    machine.cia_a.write_register(0x0, 0x00);
    machine.step_cck();
    assert!(machine.memory_bus.is_low_memory_overlay_active());
}

#[test]
fn test_paula_interrupt_cascade_to_cpu_ipl() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    assert_eq!(machine.cpu.state.ipl, 0);

    // Write INTENA ($DFF09A) = 0xC020 (SET master INTEN + Level 3 VBlank)
    assert_eq!(
        machine.memory_bus.write_word(0xDFF09A, 0xC020),
        BusResult::Ready(())
    );
    // Write INTREQ ($DFF09C) = 0x8020 (SET Level 3 VBlank request)
    assert_eq!(
        machine.memory_bus.write_word(0xDFF09C, 0x8020),
        BusResult::Ready(())
    );

    // Step 1 CCK: writes mature and commit in Paula, immediately updating CPU IPL!
    machine.step_cck();
    assert_eq!(machine.paula.intena, 0x4020);
    assert_eq!(machine.paula.intreq, 0x0020);
    assert_eq!(machine.cpu.state.ipl, 3);
}

#[test]
fn test_bus_custom_registers_snapshot_sync_and_open_bus() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Direct write to DMACON
    machine.agnus.commit_register_write(0x096, 0x8200);

    // Step CCK syncs snapshot
    machine.step_cck();

    // Reading DMACONR ($DFF002) from bus returns 0x0200
    let dmaconr = machine.memory_bus.read_word(0xDFF002).ok().unwrap();
    assert_eq!(dmaconr & 0x0200, 0x0200);

    // Reading write-only register (e.g. BPLCON0 at $DFF100) returns 0xFFFF (open bus)
    let bplcon0_read = machine.memory_bus.read_word(0xDFF100).ok().unwrap();
    assert_eq!(bplcon0_read, 0xFFFF);
}
