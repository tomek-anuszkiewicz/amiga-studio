//! Comprehensive Tests for Custom Chip Register Wiring & Cross-Chip Signals
//!
//! Validates live composite read path (DSKBYTR, CLXDAT), open-bus $FFFF enforcement
//! on write-only registers, and proper routing of DSKPT and AUDxLC.

use agnus::Agnus;
use cia::{Cia, CiaId};
use config::{A500Config, RtcModel, VideoStandard};
use denise::{Denise, DeniseModel};
use floppy::FloppyController;
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
    floppy: FloppyController,
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
            floppy: FloppyController::new(),
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
            floppy: &mut self.floppy,
        }
    }
}

#[test]
fn test_dskbytr_composite_assembly_and_clear_on_read() {
    let mut mb = TestMotherboard::new();

    // 1. Initially, no disk DMA and no sync: DSKBYTR should be 0
    assert_eq!(mb.router().read_custom_word_debug(0x01A), 0);

    // 2. Enable Disk DMA in Agnus DMACON: SET (bit 15) | DMAEN (bit 9) | DSKEN (bit 4) = 0x8210
    assert_eq!(
        mb.router().write_word(0xDFF096, 0x8210),
        BusResult::Ready(())
    );
    // Step 2 CCK cycles for DMACON to mature in Agnus and Paula
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    let _ = mb.paula.step_cck();
    let _ = mb.paula.step_cck();

    // 3. Set Paula DSKLEN to write mode: bit 14 (WRITE)
    mb.paula.commit_register_write(0x024, 0x4000);

    // 4. Set Floppy deserializer data byte $A5, WORDEQUAL (bit 12), and DSKBYT (bit 15)
    mb.floppy.dskbytr = 0x90A5; // DSKBYT | WORDEQUAL | byte 0xA5

    // 5. Peek DSKBYTR without clearing bit 15:
    // Should have:
    // - Bit 15: DSKBYT (0x8000)
    // - Bit 14: DMAON (0x4000)
    // - Bit 13: DISKWRITE (0x2000)
    // - Bit 12: WORDEQUAL (0x1000)
    // - Bits 7..0: DATA (0xA5)
    // Expected: 0xF0A5
    let peeked = mb.router().read_custom_word_debug(0x01A);
    assert_eq!(peeked, 0xF0A5);
    // Ensure bit 15 was NOT cleared by peek
    assert_eq!(mb.floppy.dskbytr & 0x8000, 0x8000);

    // 6. Read DSKBYTR with Clear-on-Read side-effects
    let read_val = mb.router().read_custom_word(0x01A);
    assert_eq!(read_val, 0xF0A5);

    // 7. Subsequent peek should show bit 15 cleared (now 0x70A5)
    let peeked_again = mb.router().read_custom_word_debug(0x01A);
    assert_eq!(peeked_again, 0x70A5);
    assert_eq!(mb.floppy.dskbytr & 0x8000, 0);
}

#[test]
fn test_open_bus_on_write_only_custom_registers() {
    let mut mb = TestMotherboard::new();

    // Populate various internal registers with non-zero values
    mb.agnus.commit_register_write(0x000, 0x1234); // BLTDDAT
    mb.agnus.commit_register_write(0x040, 0x09F0); // BLTCON0
    mb.agnus.commit_register_write(0x080, 0x0003); // COP1LCH
    mb.agnus.commit_register_write(0x082, 0x8000); // COP1LCL
    mb.agnus.commit_register_write(0x108, 0x0028); // BPL1MOD
    mb.denise.commit_register_write(0x180, 0x0F00); // COLOR00 (Red)
    mb.paula.commit_register_write(0x0A4, 0x0100); // AUD0LEN
    mb.paula.commit_register_write(0x0A8, 0x0040); // AUD0VOL

    let mut bus = mb.router();

    // Reading write-only registers via read_custom_word must return open bus $FFFF
    assert_eq!(bus.read_custom_word(0x000), 0xFFFF); // BLTDDAT (write-only)
    assert_eq!(bus.read_custom_word(0x040), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x080), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x082), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x108), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x180), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x0A4), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x0A8), 0xFFFF);
    assert_eq!(bus.read_custom_word(0x020), 0xFFFF); // DSKPTH
    assert_eq!(bus.read_custom_word(0x022), 0xFFFF); // DSKPTL

    // Peeking write-only registers via read_custom_word_debug must also return $FFFF
    assert_eq!(bus.read_custom_word_debug(0x000), 0xFFFF);
    assert_eq!(bus.read_custom_word_debug(0x040), 0xFFFF);
    assert_eq!(bus.read_custom_word_debug(0x080), 0xFFFF);
    assert_eq!(bus.read_custom_word_debug(0x180), 0xFFFF);
}

#[test]
fn test_dskpt_routed_to_agnus() {
    let mut mb = TestMotherboard::new();

    // Write DSKPTH ($DFF020) = 0x0004 and DSKPTL ($DFF022) = 0x2000
    assert_eq!(
        mb.router().write_word(0xDFF020, 0x0004),
        BusResult::Ready(())
    );
    assert_eq!(
        mb.router().write_word(0xDFF022, 0x2000),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles for writes to mature in Agnus
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();

    // Verify Agnus holds pointer $00042000
    assert_eq!(mb.agnus.dskpt, 0x0004_2000);
}

#[test]
fn test_aud0lch_aud0lcl_routed_to_agnus() {
    let mut mb = TestMotherboard::new();

    // Write AUD0LCH ($DFF0A0) = 0x0002 and AUD0LCL ($DFF0A2) = 0x8000
    assert_eq!(
        mb.router().write_word(0xDFF0A0, 0x0002),
        BusResult::Ready(())
    );
    assert_eq!(
        mb.router().write_word(0xDFF0A2, 0x8000),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles for writes to mature in Agnus
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();

    // Verify Agnus audlc[0] and audpt[0] hold $00028000
    assert_eq!(mb.agnus.audlc[0], 0x0002_8000);
    assert_eq!(mb.agnus.audpt[0], 0x0002_8000);
}

#[test]
fn test_dmacon_routing_to_agnus_and_paula() {
    let mut mb = TestMotherboard::new();

    // Write DMACON ($DFF096): SET (bit 15) | Master Enable (bit 9) | DSKEN (bit 4) | AUD0EN (bit 0) -> $8211
    assert_eq!(
        mb.router().write_word(0xDFF096, 0x8211),
        BusResult::Ready(())
    );

    // Step 2 CCKs for Agnus and Paula mutations to mature
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    let _ = mb.paula.step_cck();
    let _ = mb.paula.step_cck();

    // Verify Agnus received and committed DMACON bits
    assert_eq!(mb.agnus.dmacon & 0x0211, 0x0211);
    // Verify Paula received and committed DMACON bits
    assert_eq!(mb.paula.dma_enables & 0x0011, 0x0011);
    assert!(mb.paula.dma_master);
}

#[test]
fn test_memory_bus_canonical_constants() {
    assert_eq!(memory_bus::BANK_CUSTOM, 0xDF);
    assert_eq!(memory_bus::BANK_CIA, 0xBF);
    assert_eq!(memory_bus::BANK_RTC, 0xDC);
    assert_eq!(memory_bus::CIA_A_START, 0xBFE001);
    assert_eq!(memory_bus::CIA_A_END, 0xBFEF01);
    assert_eq!(memory_bus::CIA_B_START, 0xBFD000);
    assert_eq!(memory_bus::CIA_B_END, 0xBFDF00);
    assert_eq!(memory_bus::RTC_START, 0xDC0000);
    assert_eq!(memory_bus::RTC_END, 0xDC003F);
    assert_eq!(memory_bus::CUSTOM_REG_OFFSET_MASK, 0x01FE);
    assert_eq!(paula::DSKBYTR_DMAON, 0x4000);
    assert_eq!(paula::DSKBYTR_DISKWRITE, 0x2000);
    assert_eq!(paula::DSKBYTR_DATA_MASK, 0x90FF);
    assert_eq!(paula::DSKLEN_WRITE_FLAG, 0x4000);
}

#[test]
fn test_copjmp1_and_copjmp2_strobe_on_read() {
    let mut mb = TestMotherboard::new();

    // Set COP1LC and COP2LC
    mb.agnus.copper.cop1lc = 0x0001_0000;
    mb.agnus.copper.cop2lc = 0x0002_0000;

    // Initially Copper is not running or at address 0
    assert_eq!(mb.agnus.copper.cop_pc, 0);

    // Reading COPJMP1 ($DFF088) must strobe jump1 and return open-bus 0xFFFF
    let res1 = mb.router().read_word(0xDFF088);
    assert_eq!(res1, BusResult::Ready(0xFFFF));
    assert_eq!(mb.agnus.copper.cop_pc, 0x0001_0000);

    // Reading COPJMP2 ($DFF08A) must strobe jump2 and return open-bus 0xFFFF
    let res2 = mb.router().read_word(0xDFF08A);
    assert_eq!(res2, BusResult::Ready(0xFFFF));
    assert_eq!(mb.agnus.copper.cop_pc, 0x0002_0000);
}

#[test]
fn test_custom_byte_write_duplicates_byte_lanes() {
    let mut mb = TestMotherboard::new();

    // Writing a byte 0x42 to even address $DFF180 (COLOR00) must duplicate to 0x4242
    assert_eq!(mb.router().write_byte(0xDFF180, 0x42), BusResult::Ready(()));
    // Denise color 0 should now be 0x4242 & 0x0FFF = 0x0242
    assert_eq!(mb.denise.read_color(0), 0x0242);

    // Writing a byte 0x55 to odd address $DFF181 (COLOR00) must also duplicate to 0x5555
    assert_eq!(mb.router().write_byte(0xDFF181, 0x55), BusResult::Ready(()));
    // Denise color 0 should now be 0x5555 & 0x0FFF = 0x0555
    assert_eq!(mb.denise.read_color(0), 0x0555);
}

#[test]
fn test_debug_read_and_register_method_conventions() {
    let mut mb = TestMotherboard::new();

    // 1. JOY0DAT / JOY1DAT via Denise setters & read_word_debug
    mb.denise.set_joy0dat(0x1234);
    mb.denise.set_joy1dat(0x5678);
    assert_eq!(mb.router().read_word_debug(0xDFF00A), 0x1234);
    assert_eq!(mb.router().read_word_debug(0xDFF00C), 0x5678);

    // 2. CLXDAT debug read does NOT clear collision bits
    mb.denise.clxdat = 0x00A5;
    assert_eq!(mb.router().read_word_debug(0xDFF00E), 0x00A5);
    assert_eq!(mb.denise.clxdat_debug(), 0x00A5);
    // Live read clears it
    assert_eq!(mb.router().read_word(0xDFF00E), BusResult::Ready(0x00A5));
    assert_eq!(mb.denise.clxdat_debug(), 0x0000);

    // 3. COPJMP1 strobe directly via strobe_copjmp1()
    mb.agnus.copper.cop1lc = 0x0003_4000;
    mb.agnus.strobe_copjmp1();
    assert_eq!(mb.agnus.copper.cop_pc, 0x0003_4000);

    // 4. CIA debug byte read does not clear or alter state
    mb.cia_a.pra = 0xAA;
    let debug_cia = mb.router().read_cia_byte_debug(0xBFE001);
    assert_eq!(debug_cia, mb.cia_a.read_register_debug(0));
    assert_eq!(debug_cia, 0xAA);
}
