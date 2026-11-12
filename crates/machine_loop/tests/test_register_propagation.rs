#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use config::cia_reg;
use config::custom_reg;
use config::{A500Config, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_cross_chip_bplcon0_broadcast_latencies() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Write BPLCON0 ($DFF100) = 0x8200 (HIRES mode) via memory bus router
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::denise::REGISTER_BPLCON0),
            0x8200
        ),
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
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACON),
            0x8211
        ),
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
fn test_cia_a_overlay_follows_committed_port_level() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine
        .physical_memory
        .write_bytes_debug(0xf80000, &[0x12, 0x34]);
    for (addr, value, expected_overlay) in [
        (cia_reg::cia_a::REGISTER_DDRA, 1, false),
        (cia_reg::cia_a::REGISTER_PRA, 1, true),
        (cia_reg::cia_a::REGISTER_PRA, 0, false),
        (cia_reg::cia_a::REGISTER_DDRA, 0, true),
    ] {
        let before = machine.physical_memory.is_low_memory_overlay_active();
        assert_eq!(
            machine.memory_bus().write_byte(addr, value),
            BusResult::WaitState
        );
        assert_eq!(
            machine.physical_memory.is_low_memory_overlay_active(),
            before
        );
        assert!(machine.cia_a.mutations.iter().all(Option::is_none));
        assert_eq!(
            common::cia::accept_write(&mut machine, addr, value),
            BusResult::Ready(())
        );
        assert_eq!(machine.cia_a.eclock_phase, 1);
        assert_eq!(machine.cia_a.mutations.iter().flatten().count(), 1);
        assert_eq!(
            machine.physical_memory.is_low_memory_overlay_active(),
            before
        );
        machine.step_cck();
        assert_eq!(machine.cia_a.eclock_phase, 2);
        assert_eq!(
            machine.physical_memory.is_low_memory_overlay_active(),
            expected_overlay
        );
        let expected_word = if expected_overlay { 0x1234 } else { 0 };
        assert_eq!(
            machine.memory_bus().read_word(0),
            BusResult::Ready(expected_word)
        );
        machine.step_cycles(5);
        assert_eq!(
            machine.physical_memory.is_low_memory_overlay_active(),
            expected_overlay
        );
    }
    // CIA-B has no OVL connection.
    common::cia::write_byte(&mut machine, cia_reg::cia_b::REGISTER_DDRA, 1);
    common::cia::write_byte(&mut machine, cia_reg::cia_b::REGISTER_PRA, 0);
    assert!(machine.physical_memory.is_low_memory_overlay_active());
}

#[test]
fn test_paula_interrupt_cascade_to_cpu_ipl() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    assert_eq!(machine.cpu.state.ipl(), 0);

    // Write INTENA ($DFF09A) = 0xC020 (SET master INTEN + Level 3 VBlank)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_INTENA),
            0xC020
        ),
        BusResult::Ready(())
    );
    // Write INTREQ ($DFF09C) = 0x8020 (SET Level 3 VBlank request)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_INTREQ),
            0x8020
        ),
        BusResult::Ready(())
    );

    // Step 1 CCK: writes mature and commit in Paula, immediately updating CPU IPL!
    machine.step_cck();
    assert_eq!(machine.paula.interrupts.intena, 0x4020);
    assert_eq!(machine.paula.interrupts.intreq, 0x0020);
    assert_eq!(machine.cpu.state.ipl(), 3);
}

#[test]
fn test_bus_custom_registers_snapshot_sync_and_open_bus() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Direct write to DMACON
    machine.agnus.set_dmacon(0x8200);

    // Two CCKs commit the queued DMA control.
    machine.step_cycles(2);

    // Reading DMACONR ($DFF002) from bus returns 0x0200
    let BusResult::Ready(dmaconr) = machine
        .memory_bus()
        .read_word(0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACONR))
    else {
        panic!("Expected a completed bus read");
    };
    assert_eq!(dmaconr & 0x0200, 0x0200);

    // Reading write-only register (e.g. BPLCON0 at $DFF100) returns 0xFFFF (open bus)
    let BusResult::Ready(bplcon0_read) = machine
        .memory_bus()
        .read_word(0xDFF000 + u32::from(custom_reg::denise::REGISTER_BPLCON0))
    else {
        panic!("Expected a completed bus read");
    };
    assert_eq!(bplcon0_read, 0xFFFF);
}

#[test]
fn test_write_custom_word_and_byte_methods() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Test write_custom_word directly on MemoryBus
    machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_COLOR00, 0x0F00);
    assert_eq!(machine.denise.color[0], 0);
    machine.step_cck();
    assert_eq!(machine.denise.color[0], 0x0F00);

    // Test write_custom_byte with byte duplication
    machine.memory_bus().write_custom_byte(
        0xDFF000 + u32::from(custom_reg::denise::REGISTER_COLOR01),
        0x33,
    );
    assert_eq!(machine.denise.color[1], 0);
    machine.step_cck();
    assert_eq!(machine.denise.color[1], 0x0333);
}

#[test]
fn test_dmacon_write_propagation_delay() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Initially DMACON is 0
    assert_eq!(machine.agnus.dmacon, 0);

    // Write DMACON = SET DMAEN (bit 9) + SPREN (bit 5) + BPLEN (bit 8) -> 0x8320
    machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8320);

    // After 1 CCK: in-flight in mutation pipeline, not yet committed in Agnus (2 CCKs delay)
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon, 0);

    // After 2 CCKs: matured and committed in Agnus
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon, 0x0320);

    // Clear SPREN (bit 5)
    machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x0020);
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon, 0x0320);

    machine.step_cck();
    assert_eq!(machine.agnus.dmacon, 0x0300);
}
