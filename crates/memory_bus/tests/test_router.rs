//! Integration & Unit Tests for Motherboard MemoryBus Router
//!
//! Validates 24-bit physical address routing across PhysicalMemory,
//! Custom Chips ($DFFxxx), CIAs ($BFDxxx/$BFExxx), and RTC ($DCxxxx).

use agnus::Agnus;
use cia::{Cia, CiaId};
use config::{A500Config, RtcModel, VideoStandard};
use denise::{Denise, DeniseModel};
use memory_bus::MemoryBus;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, PhysicalMemory};
use rtc::RtcMsm6242b;

struct TestMotherboard {
    mem: PhysicalMemory,
    agnus: Agnus,
    denise: Denise,
    paula: Paula,
    cia_a: Cia,
    cia_b: Cia,
    rtc: RtcMsm6242b,
}

impl TestMotherboard {
    fn new() -> Self {
        let mut mem = PhysicalMemory::new();
        mem.map_chip_ram_to_low_memory();
        Self {
            mem,
            agnus: Agnus::new(A500Config::standard_1mb(VideoStandard::Pal).agnus_model()),
            denise: Denise::new(DeniseModel::Ocs8362),
            paula: Paula::new(),
            cia_a: Cia::new(CiaId::A),
            cia_b: Cia::new(CiaId::B),
            rtc: RtcMsm6242b::new(RtcModel::Msm6242b),
        }
    }

    fn router(&mut self) -> MemoryBus<'_> {
        MemoryBus {
            mem: &mut self.mem,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
        }
    }
}

#[test]
fn test_physical_memory_passthrough() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();

    // Chip RAM write and read
    let _ = bus.write_word(0x001000, 0x1234);
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0x1234));
    assert_eq!(bus.read_byte(0x001000), BusResult::Ready(0x12));
    assert_eq!(bus.read_byte(0x001001), BusResult::Ready(0x34));

    // Open bus read returns $FF / $FFFF
    assert_eq!(bus.read_byte(0x100000), BusResult::Ready(0xFF));
    assert_eq!(bus.read_word(0x100000), BusResult::Ready(0xFFFF));
}

#[test]
fn test_custom_register_broadcast_and_routing() {
    let mut mb = TestMotherboard::new();
    {
        let mut bus = mb.router();
        // Write DMACON ($DFF096): set master DMA enable and Copper DMA enable ($8280)
        let _ = bus.write_word(0xDFF096, 0x8280);
    }
    // Check that Agnus staged DMACON mutation with physical propagation delay
    let agnus_staged = mb
        .agnus
        .mutations
        .iter()
        .flatten()
        .any(|m| m.reg_offset == 0x096 && m.value == 0x8280);
    assert!(
        agnus_staged,
        "Agnus must stage DMACON write in mutation pipeline"
    );

    // Step 2 CCKs: Agnus mutation matures and enables Copper DMA
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    assert!(mb.agnus.copper.dma_enabled, "Copper DMA must be enabled");
}

#[test]
fn test_cia_address_decoding() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();

    // CIA-A is at odd byte addresses ($BFE001..$BFEF01)
    // Write CRA (Control Register A) at offset $E -> $BFE001 + ($E << 8) = $BFEE01
    let _ = bus.write_byte(0xBFEE01, 0x55);
    assert_eq!(bus.read_byte(0xBFEE01), BusResult::Ready(0x55));

    // Even byte in CIA-A range returns open bus $FF
    assert_eq!(bus.read_byte(0xBFEE00), BusResult::Ready(0xFF));

    // CIA-B is at even byte addresses ($BFD000..$BFDF00)
    // Write CRB at offset $F -> $BFD000 + ($F << 8) = $BFDF00
    let _ = bus.write_byte(0xBFDF00, 0xAA);
    assert_eq!(bus.read_byte(0xBFDF00), BusResult::Ready(0xAA));

    // Odd byte in CIA-B range returns open bus $FF
    assert_eq!(bus.read_byte(0xBFDF01), BusResult::Ready(0xFF));
}

#[test]
fn test_ciaa_port_a_overlay_toggle() {
    let mut mb = TestMotherboard::new();
    mb.mem
        .write_bytes_debug(0xF80000, &[0x11, 0x22, 0x33, 0x44]);
    mb.mem.map_kickstart_to_low_memory();

    let mut bus = mb.router();
    // With overlay active, $000000 reads from ROM
    assert_eq!(bus.read_word(0x000000), BusResult::Ready(0x1122));

    // Writing 1 to bit 0 of CIA-A PRA ($BFE001) disables overlay
    let _ = bus.write_byte(0xBFE001, 0x01);
    // Overlay is now off, $000000 reads Chip RAM (which is 0)
    assert_eq!(bus.read_word(0x000000), BusResult::Ready(0x0000));
}

#[test]
fn test_rtc_odd_byte_routing() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();

    // RTC registers reside on odd byte addresses in $DC0000..$DC003F
    let _ = bus.write_byte(0xDC0001, 0x07);
    assert_eq!(bus.read_byte(0xDC0001), BusResult::Ready(0x07));

    // Non-RTC register address in bank $DC returns floating open bus
    assert_eq!(bus.read_byte(0xDC0040), BusResult::Ready(0xFF));
}

#[test]
fn test_direct_custom_and_cia_register_writes() {
    let mut mb = TestMotherboard::new();
    {
        let mut bus = mb.router();
        // Denise: COLOR00 ($DFF180)
        let _ = bus.write_word(0xDFF180, 0x0F00);
        // Paula: INTENA ($DFF09A) - set bit 15 (SET) | bit 14 (INTEN) | bit 0 (TBE) = 0xC001
        let _ = bus.write_word(0xDFF09A, 0xC001);
        // Agnus: BLTCON0 ($DFF040)
        let _ = bus.write_word(0xDFF040, 0x09F0);
        // Shared: BPLCON0 ($DFF100)
        let _ = bus.write_word(0xDFF100, 0x1200);
        // CIA-A: CRA ($BFEE01)
        let _ = bus.write_byte(0xBFEE01, 0x55);
        // CIA-B: CRB ($BFDF00)
        let _ = bus.write_byte(0xBFDF00, 0xAA);
    }

    assert_eq!(mb.denise.color[0], 0x0F00);
    // Step Paula 1 CCK so staged INTENA write matures
    let _ = mb.paula.step_cck();
    assert_eq!(mb.paula.interrupts.intena, 0x4001);
    // Step Agnus 2 CCKs so staged BLTCON0 write matures
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    assert_eq!(mb.agnus.blitter.bltcon0, 0x09F0);
    assert_eq!(mb.cia_a.cra, 0x55);
    assert_eq!(mb.cia_b.crb, 0xAA);
}

#[test]
fn test_memory_bus_open_bus_read_byte() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();
    let res = bus.read_byte(0x200000);
    assert_eq!(res, BusResult::Ready(0xFF));
}
