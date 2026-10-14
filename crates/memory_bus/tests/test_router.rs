#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration & Unit Tests for Motherboard MemoryBus Router
//!
//! Validates 24-bit physical address routing across PhysicalMemory,
//! Custom Chips ($DFFxxx), CIAs ($BFDxxx/$BFExxx), and RTC ($DCxxxx).

use agnus::Agnus;
use cia::{Cia, CiaId};
use config::{cia_reg, custom_reg};
use config::{A500Config, RtcModel, VideoStandard};
use denise::{Denise, DeniseModel};
use memory_bus::MemoryBus;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
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
        let _ = bus.write_word(0xDFF000 + u32::from(custom_reg::agnus::DMACON), 0x8280);
    }
    // Check that Agnus staged DMACON mutation with physical propagation delay
    let agnus_staged = mb
        .agnus
        .mutations
        .iter()
        .flatten()
        .any(|m| m.reg_offset == custom_reg::agnus::DMACON && m.value == 0x8280);
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
fn test_all_cia_addresses_select_their_registers() {
    let mut mb = TestMotherboard::new();
    for (cia, base) in [(&mut mb.cia_a, 0x20u8), (&mut mb.cia_b, 0x40u8)] {
        cia.pra = base;
        cia.prb = base + 1;
        cia.ddra = base + 2;
        cia.ddrb = base + 3;
        cia.ta_counter = u16::from_be_bytes([base + 5, base + 4]);
        cia.tb_counter = u16::from_be_bytes([base + 7, base + 6]);
        cia.tod = u32::from_be_bytes([0, base + 10, base + 9, base + 8]);
        cia.sdr = base + 12;
        cia.icr_data = base + 13;
        cia.cra = base + 14;
        cia.crb = base + 15;
    }

    let bus = mb.router();
    for (addr, expected) in [
        (cia_reg::cia_a::PRA, 0x20),
        (cia_reg::cia_a::PRB, 0x21),
        (cia_reg::cia_a::DDRA, 0x22),
        (cia_reg::cia_a::DDRB, 0x23),
        (cia_reg::cia_a::TALO, 0x24),
        (cia_reg::cia_a::TAHI, 0x25),
        (cia_reg::cia_a::TBLO, 0x26),
        (cia_reg::cia_a::TBHI, 0x27),
        (cia_reg::cia_a::TODLO, 0x28),
        (cia_reg::cia_a::TODMID, 0x29),
        (cia_reg::cia_a::TODHI, 0x2A),
        (cia_reg::cia_a::SDR, 0x2C),
        (cia_reg::cia_a::ICR, 0x2D),
        (cia_reg::cia_a::CRA, 0x2E),
        (cia_reg::cia_a::CRB, 0x2F),
        (cia_reg::cia_b::PRA, 0x40),
        (cia_reg::cia_b::PRB, 0x41),
        (cia_reg::cia_b::DDRA, 0x42),
        (cia_reg::cia_b::DDRB, 0x43),
        (cia_reg::cia_b::TALO, 0x44),
        (cia_reg::cia_b::TAHI, 0x45),
        (cia_reg::cia_b::TBLO, 0x46),
        (cia_reg::cia_b::TBHI, 0x47),
        (cia_reg::cia_b::TODLO, 0x48),
        (cia_reg::cia_b::TODMID, 0x49),
        (cia_reg::cia_b::TODHI, 0x4A),
        (cia_reg::cia_b::SDR, 0x4C),
        (cia_reg::cia_b::ICR, 0x4D),
        (cia_reg::cia_b::CRA, 0x4E),
        (cia_reg::cia_b::CRB, 0x4F),
    ] {
        assert_eq!(
            bus.read_cia_byte_debug(addr),
            expected,
            "address {addr:06X}"
        );
    }
}

#[test]
fn test_cia_register_addresses_mirror_on_unused_low_bits() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();

    let _ = bus.write_byte(cia_reg::cia_a::CRA, 0x55);
    let _ = bus.write_byte(cia_reg::cia_b::CRA, 0x2A);

    // A7..A1 do not participate in CIA register selection.
    assert_eq!(
        bus.read_byte(cia_reg::cia_a::CRA + 2),
        BusResult::Ready(0x55)
    );
    assert_eq!(
        bus.read_byte(cia_reg::cia_b::CRA + 2),
        BusResult::Ready(0x2A)
    );
    assert_eq!(bus.read_cia_byte_debug(cia_reg::cia_a::CRA + 2), 0x55);
    assert_eq!(bus.read_cia_byte_debug(cia_reg::cia_b::CRA + 2), 0x2A);

    let _ = bus.write_byte(cia_reg::cia_a::CRA + 2, 0x11);
    let _ = bus.write_byte(cia_reg::cia_b::CRA + 2, 0x12);
    assert_eq!(bus.read_byte(cia_reg::cia_a::CRA), BusResult::Ready(0x11));
    assert_eq!(bus.read_byte(cia_reg::cia_b::CRA), BusResult::Ready(0x12));

    // Selector $B is unconnected on both chips.
    assert_eq!(bus.read_byte(0xBFEB01), BusResult::Ready(0xFF));
    assert_eq!(bus.read_byte(0xBFDB00), BusResult::Ready(0xFF));
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
        let _ = bus.write_word(0xDFF000 + u32::from(custom_reg::denise::COLOR00), 0x0F00);
        // Paula: INTENA ($DFF09A) - set bit 15 (SET) | bit 14 (INTEN) | bit 0 (TBE) = 0xC001
        let _ = bus.write_word(0xDFF000 + u32::from(custom_reg::paula::INTENA), 0xC001);
        // Agnus: BLTCON0 ($DFF040)
        let _ = bus.write_word(0xDFF000 + u32::from(custom_reg::agnus::BLTCON0), 0x09F0);
        // Shared: BPLCON0 ($DFF100)
        let _ = bus.write_word(0xDFF000 + u32::from(custom_reg::denise::BPLCON0), 0x1200);
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
fn test_open_bus_across_unmapped_banks_and_device_lanes() {
    let mut mb = TestMotherboard::new();
    // These banks have no installed storage in the standard A500 preset.
    let unmapped = [
        0x080000, 0x100000, 0x180000, 0x200000, 0x600000, 0xC80000, 0xDD0000, 0xE00000,
    ];
    for addr in unmapped {
        assert_eq!(
            mb.mem.bank_map[(addr >> 16) as usize].bank,
            MemoryBank::OpenBus
        );
    }

    let mut bus = mb.router();
    for addr in unmapped {
        assert_eq!(bus.read_byte(addr), BusResult::Ready(0xFF), "{addr:06X}");
        assert_eq!(bus.read_word(addr), BusResult::Ready(0xFFFF), "{addr:06X}");
    }

    // CIA-A uses the odd lane; CIA-B uses the even lane.
    assert_eq!(bus.read_byte(0xBFEE00), BusResult::Ready(0xFF));
    assert_eq!(bus.read_byte(0xBFDF01), BusResult::Ready(0xFF));
    assert_eq!(bus.read_byte(0xBFEB01), BusResult::Ready(0xFF));
    assert_eq!(bus.read_byte(0xBFDB00), BusResult::Ready(0xFF));
    assert_eq!(bus.write_byte(0xBFEE01, 0x55), BusResult::Ready(()));
    assert_eq!(bus.read_word(0xBFEE00), BusResult::Ready(0xFF55));

    // The RTC drives odd addresses only; the rest of its bank is unmapped.
    assert_eq!(bus.write_byte(0xDC0001, 0x07), BusResult::Ready(()));
    assert_eq!(bus.read_byte(0xDC0000), BusResult::Ready(0xFF));
    assert_eq!(bus.read_word(0xDC0000), BusResult::Ready(0xFF07));
    assert_eq!(bus.read_word(0xDC0040), BusResult::Ready(0xFFFF));

    // Early DMA addresses and write-only registers do not return their stored data.
    for offset in [
        custom_reg::agnus::BLTDDAT,
        custom_reg::paula::DSKDATR,
        custom_reg::agnus::BLTCON0,
        custom_reg::denise::COLOR00,
        custom_reg::paula::AUD0LEN,
    ] {
        let addr = 0xDFF000 + u32::from(offset);
        assert_eq!(bus.read_word(addr), BusResult::Ready(0xFFFF), "{addr:06X}");
        assert_eq!(bus.read_byte(addr), BusResult::Ready(0xFF), "{addr:06X}");
    }
    assert_eq!(
        bus.read_word(0xDFF000 + u32::from(custom_reg::agnus::COPJMP1)),
        BusResult::Ready(0xFFFF)
    );
    assert_eq!(
        bus.read_word(0xDFF000 + u32::from(custom_reg::agnus::COPJMP2)),
        BusResult::Ready(0xFFFF)
    );

    // A populated bank and an active register still return driven data.
    assert_eq!(bus.write_word(0x001000, 0x1234), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0x1234));
    assert_eq!(bus.read_byte(0xBFEE01), BusResult::Ready(0x55));
    assert_eq!(bus.read_byte(0xDC0001), BusResult::Ready(0x07));

    mb.rtc = RtcMsm6242b::new(RtcModel::None);
    assert_eq!(mb.router().read_byte(0xDC0001), BusResult::Ready(0xFF));
}

#[test]
fn test_memory_bus_debug_derive() {
    let mut mb = TestMotherboard::new();
    let bus = mb.router();
    let debug_str = format!("{:?}", bus);
    assert!(debug_str.contains("MemoryBus"));
}
