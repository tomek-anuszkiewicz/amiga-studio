#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Comprehensive Tests for Custom Chip Register Wiring & Cross-Chip Signals
//!
//! Validates live composite read path (DSKBYTR, CLXDAT), open-bus $FFFF enforcement
//! on write-only registers, and proper routing of DSKPT and AUDxLC.

use agnus::Agnus;
use cia::Cia;
use config::custom_reg;
use config::{A500Config, DelayedMutation, MutationMode, RtcModel, VideoStandard};
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

// Receiver order: Agnus, Denise, Paula. The OCS address families come from the
// ownership matrix; delays and queue modes are the provisional transport contract.
fn delivery_contract(offset: u16) -> [(u8, MutationMode); 3] {
    use MutationMode::{OverwritePending as Replace, Pipeline};
    let mut receivers = [(0, Replace); 3];
    match offset {
        0x020
        | 0x022
        | 0x028..=0x02e
        | 0x040..=0x056
        | 0x060..=0x066
        | 0x080..=0x086
        | 0x08c
        | 0x108
        | 0x10a
        | 0x120..=0x13e => {
            receivers[0] = (2, Replace);
        }
        0x058 | 0x088 | 0x08a => receivers[0] = (1, Replace),
        0x070..=0x074 => receivers[0] = (1, Pipeline),
        0x08e | 0x090 | 0x100 => {
            receivers[0] = (4, Replace);
            receivers[1] = (1, Replace);
        }
        0x092 | 0x094 => receivers[0] = (4, Replace),
        0x096 => {
            receivers[0] = (2, Replace);
            receivers[2] = (2, Replace);
        }
        0x036 | 0x038 | 0x03a | 0x03e | 0x098 | 0x102 | 0x104 | 0x180..=0x1be => {
            receivers[1] = (1, Replace);
        }
        0x03c => {
            receivers[1] = (1, Replace);
            receivers[2] = (1, Replace);
        }
        0x110..=0x11a => receivers[1] = (1, Pipeline),
        0x024 | 0x026 | 0x032 | 0x034 | 0x07e | 0x09e => receivers[2] = (2, Replace),
        0x030 => receivers[2] = (1, Pipeline),
        0x09a | 0x09c => receivers[2] = (1, Replace),
        0x0a0..=0x0da => match offset & 0xf {
            0 | 2 => receivers[0] = (2, Replace),
            4 | 6 | 8 => receivers[2] = (2, Replace),
            0xa => receivers[2] = (1, Pipeline),
            _ => {}
        },
        0x0e0..=0x0f6 => receivers[0] = (2, Replace),
        0x140..=0x17e => {
            receivers[1] = (1, Replace);
            if offset & 7 < 4 {
                receivers[0] = (2, Replace);
            }
        }
        _ => {}
    }
    receivers
}

#[test]
fn every_ocs_write_reaches_only_its_owners_and_is_delivered_once_when_due() {
    for video in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut writable = 0;
        for offset in (0..=0x1fe).step_by(2) {
            let contract = delivery_contract(offset);
            writable += usize::from(contract.iter().any(|&(delay, _)| delay != 0));
            // All aligned slots, including reserved/dummy addresses, and both byte lanes.
            for lane in 0..3 {
                let mut mb = TestMotherboard::new();
                mb.agnus = Agnus::new(A500Config::bare_512k(video).agnus_model());
                let address = 0xdff000 + u32::from(offset);
                let result = match lane {
                    0 => mb.router().write_word(address, 0xa55a),
                    1 => mb.router().write_byte(address, 0x5a),
                    _ => mb.router().write_byte(address + 1, 0x5a),
                };
                assert_eq!(result, BusResult::Ready(()), "{video:?} {offset:03x}");
                let value = match offset {
                    0x038..=0x03e | 0x088 | 0x08a => 0, // Address strobes ignore data.
                    _ if lane == 0 => 0xa55a,
                    _ => 0x5a5a,
                };
                for (queue, &(delay, mode)) in [
                    mb.agnus.mutations.as_slice(),
                    mb.denise.mutations.as_slice(),
                    mb.paula.mutations.as_slice(),
                ]
                .into_iter()
                .zip(&contract)
                {
                    let expected: Vec<_> = if delay == 0 {
                        vec![]
                    } else {
                        vec![DelayedMutation {
                            reg_offset: offset,
                            value,
                            remaining_cck: delay,
                            mode,
                        }]
                    };
                    assert_eq!(
                        queue.iter().flatten().copied().collect::<Vec<_>>(),
                        expected,
                        "{video:?} {offset:03x} lane {lane}"
                    );
                }
                for elapsed in 1..=5 {
                    let due = [
                        mb.agnus
                            .step_cck()
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>(),
                        mb.denise
                            .step_cck(mb.agnus.beam())
                            .into_iter()
                            .flatten()
                            .collect(),
                        mb.paula.step_cck().into_iter().flatten().collect(),
                    ];
                    for (events, &(delay, _)) in due.iter().zip(&contract) {
                        let expected = if delay != 0 && elapsed == delay {
                            vec![(offset, value)]
                        } else {
                            vec![]
                        };
                        assert_eq!(
                            *events, expected,
                            "{video:?} {offset:03x} lane {lane} elapsed {elapsed}"
                        );
                    }
                }
                assert!(mb.agnus.mutations.iter().all(Option::is_none));
                assert!(mb.denise.mutations.iter().all(Option::is_none));
                assert!(mb.paula.mutations.iter().all(Option::is_none));
            }
        }
        assert_eq!(writable, 180);
    }
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
            cia_a: Cia::new(),
            cia_b: Cia::new(),
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
fn test_dskbytr_composite_assembly_and_clear_on_read() {
    let mut mb = TestMotherboard::new();

    // 1. Initially, no disk DMA and no sync: DSKBYTR should be 0
    assert_eq!(
        mb.router()
            .read_custom_word_debug(custom_reg::paula::DSKBYTR),
        0
    );

    // 2. Enable Disk DMA in Agnus DMACON: SET (bit 15) | DMAEN (bit 9) | DSKEN (bit 4) = 0x8210
    assert_eq!(
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::DMACON), 0x8210),
        BusResult::Ready(())
    );
    // Step 2 CCK cycles for DMACON to mature in Agnus and Paula
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    let _ = mb.paula.step_cck();
    let _ = mb.paula.step_cck();

    // 3. Set Paula DSKLEN to write mode: bit 14 (WRITE)
    mb.router()
        .write_custom_word(custom_reg::paula::DSKLEN, 0x4000);
    mb.paula.step_cck();
    mb.paula.step_cck();

    // 4. Set Paula DSKBYTR with data byte $A5, WORDEQUAL (bit 12), and DSKBYT (bit 15)
    mb.paula.dskbytr = 0x90A5; // DSKBYT | WORDEQUAL | byte 0xA5

    // 5. Peek DSKBYTR without clearing bit 15:
    // Should have:
    // - Bit 15: DSKBYT (0x8000)
    // - Bit 14: DMAON (0x4000)
    // - Bit 13: DISKWRITE (0x2000)
    // - Bit 12: WORDEQUAL (0x1000)
    // - Bits 7..0: DATA (0xA5)
    // Expected: 0xF0A5
    let peeked = mb
        .router()
        .read_custom_word_debug(custom_reg::paula::DSKBYTR);
    assert_eq!(peeked, 0xF0A5);
    // Ensure bit 15 was NOT cleared by peek
    assert_eq!(mb.paula.dskbytr & 0x8000, 0x8000);

    // 6. Read DSKBYTR with Clear-on-Read side-effects
    let read_val = mb.router().read_custom_word(custom_reg::paula::DSKBYTR);
    assert_eq!(read_val, 0xF0A5);

    // 7. Subsequent peek should show bit 15 cleared (now 0x70A5)
    let peeked_again = mb
        .router()
        .read_custom_word_debug(custom_reg::paula::DSKBYTR);
    assert_eq!(peeked_again, 0x70A5);
    assert_eq!(mb.paula.dskbytr & 0x8000, 0);
}

#[test]
fn custom_live_and_debug_reads_select_the_owner_or_explicit_fallback() {
    let mut mb = TestMotherboard::new();
    mb.agnus.dmacon = 0x0123;
    mb.agnus.lof = true;
    mb.agnus.step_cck();
    mb.denise.joy0dat = 0x2345;
    mb.denise.joy1dat = 0x3456;
    mb.denise.clxdat = 0x4567;
    mb.paula.adkcon = 0x5678;
    mb.paula.pot0dat = 0x6789;
    mb.paula.pot1dat = 0x789a;
    mb.paula.potgor = 0x89ab;
    mb.paula.serial_port.serdatr = 0x9abc;
    mb.paula.dskbytr = 0x90cd;
    mb.paula.interrupts.intena = 0x1234;
    mb.paula.interrupts.intreq = 0x2346;
    assert!(mb.denise.set_color(0, 0x0abc));
    // Beam/status computation is delegated to the owner. This test establishes
    // reader selection; existing owner tests cover computed-value semantics.
    let readable: [(u16, u16); 14] = [
        (0x002, mb.agnus.dmaconr()),
        (0x004, mb.agnus.vposr()),
        (0x006, mb.agnus.vhposr()),
        (0x00a, 0x2345),
        (0x00c, 0x3456),
        (0x00e, 0x4567),
        (0x010, 0x5678),
        (0x012, 0x6789),
        (0x014, 0x789a),
        (0x016, 0x89ab),
        (0x018, 0x9abc),
        (0x01a, 0x90cd),
        (0x01c, 0x1234),
        (0x01e, 0x2346),
    ];
    let before = (mb.agnus.clone(), mb.denise.clone(), mb.paula.clone());
    for offset in (0..=0x1fe).step_by(2) {
        let expected = readable
            .iter()
            .find(|&&(reg, _)| reg == offset)
            .map_or(0xffff, |&(_, value)| value);
        assert_eq!(
            mb.router().read_word_debug(0xdff000 + u32::from(offset)),
            expected,
            "debug {offset:03x}"
        );
    }
    assert_eq!(
        (mb.agnus.clone(), mb.denise.clone(), mb.paula.clone()),
        before
    );
    for offset in (0..=0x1fe).step_by(2) {
        let expected = readable
            .iter()
            .find(|&&(reg, _)| reg == offset)
            .map_or(0xffff, |&(_, value)| value);
        assert_eq!(
            mb.router().read_word(0xdff000 + u32::from(offset)),
            BusResult::Ready(expected),
            "live {offset:03x}"
        );
    }
    assert_eq!(mb.denise.clxdat, 0);
    assert_eq!(mb.paula.dskbytr, 0x10cd);
    assert_eq!(mb.denise.mutations, before.1.mutations);
}

#[test]
fn test_open_bus_on_early_dma_and_write_only_registers() {
    let mut mb = TestMotherboard::new();

    // Populate various internal registers with non-zero values
    mb.router()
        .write_custom_word(custom_reg::agnus::BLTDDAT, 0x1234); // BLTDDAT
    mb.router()
        .write_custom_word(custom_reg::agnus::BLTCON0, 0x09F0); // BLTCON0
    mb.router()
        .write_custom_word(custom_reg::agnus::COP1LCH, 0x0003); // COP1LCH
    mb.router()
        .write_custom_word(custom_reg::agnus::COP1LCL, 0x8000); // COP1LCL
    mb.router()
        .write_custom_word(custom_reg::agnus::BPL1MOD, 0x0028); // BPL1MOD
    mb.router()
        .write_custom_word(custom_reg::denise::COLOR00, 0x0F00); // COLOR00 (Red)
    mb.router()
        .write_custom_word(custom_reg::paula::AUD0LEN, 0x0100); // AUD0LEN
    mb.router()
        .write_custom_word(custom_reg::paula::AUD0VOL, 0x0040); // AUD0VOL

    for _ in 0..2 {
        mb.agnus.step_cck();
        mb.denise.step_cck(mb.agnus.beam());
        mb.paula.step_cck();
    }
    let mut bus = mb.router();

    // CPU reads of the early DMA read address and write-only registers return $FFFF.
    assert_eq!(bus.read_custom_word(custom_reg::agnus::BLTDDAT), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::agnus::BLTCON0), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::agnus::COP1LCH), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::agnus::COP1LCL), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::agnus::BPL1MOD), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::denise::COLOR00), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::paula::AUD0LEN), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::paula::AUD0VOL), 0xFFFF);
    assert_eq!(bus.read_custom_word(custom_reg::agnus::DSKPTH), 0xFFFF); // DSKPTH
    assert_eq!(bus.read_custom_word(custom_reg::agnus::DSKPTL), 0xFFFF); // DSKPTL

    // Debug reads of those addresses also return $FFFF.
    assert_eq!(
        bus.read_custom_word_debug(custom_reg::agnus::BLTDDAT),
        0xFFFF
    );
    assert_eq!(
        bus.read_custom_word_debug(custom_reg::agnus::BLTCON0),
        0xFFFF
    );
    assert_eq!(
        bus.read_custom_word_debug(custom_reg::agnus::COP1LCH),
        0xFFFF
    );
    assert_eq!(
        bus.read_custom_word_debug(custom_reg::denise::COLOR00),
        0xFFFF
    );
}

#[test]
fn test_dskpt_routed_to_agnus() {
    let mut mb = TestMotherboard::new();

    // Write DSKPTH ($DFF020) = 0x0004 and DSKPTL ($DFF022) = 0x2000
    assert_eq!(
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::DSKPTH), 0x0004),
        BusResult::Ready(())
    );
    assert_eq!(
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::DSKPTL), 0x2000),
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
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::AUD0LCH), 0x0002),
        BusResult::Ready(())
    );
    assert_eq!(
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::AUD0LCL), 0x8000),
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
fn test_ocs_agnus_pointer_writes_ignore_bits_above_512k_chip_ram() {
    let mut mb = TestMotherboard::new();

    for (high, low) in [
        (custom_reg::agnus::COP1LCH, custom_reg::agnus::COP1LCL),
        (custom_reg::agnus::BLTAPTH, custom_reg::agnus::BLTAPTL),
        (custom_reg::agnus::BPL1PTH, custom_reg::agnus::BPL1PTL),
        (custom_reg::agnus::SPR0PTH, custom_reg::agnus::SPR0PTL),
        (custom_reg::agnus::DSKPTH, custom_reg::agnus::DSKPTL),
        (custom_reg::agnus::AUD0LCH, custom_reg::agnus::AUD0LCL),
    ] {
        assert_eq!(
            mb.router().write_word(0xDFF000 + u32::from(high), 0x001D),
            BusResult::Ready(())
        );
        assert_eq!(
            mb.router().write_word(0xDFF000 + u32::from(low), 0x2345),
            BusResult::Ready(())
        );
    }

    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();

    let expected = 0x0005_2344;
    assert_eq!(mb.agnus.copper.cop1lc, expected);
    assert_eq!(mb.agnus.blitter.bltapt, expected);
    assert_eq!(mb.agnus.bplpt[0], expected);
    assert_eq!(mb.agnus.sprpt[0], expected);
    assert_eq!(mb.agnus.dskpt, expected);
    assert_eq!(mb.agnus.audlc[0], expected);
    assert_eq!(mb.agnus.audpt[0], expected);
}

#[test]
fn test_dmacon_routing_to_agnus_and_paula() {
    let mut mb = TestMotherboard::new();

    // Write DMACON ($DFF096): SET (bit 15) | Master Enable (bit 9) | DSKEN (bit 4) | AUD0EN (bit 0) -> $8211
    assert_eq!(
        mb.router()
            .write_word(0xDFF000 + u32::from(custom_reg::agnus::DMACON), 0x8211),
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
    assert_eq!(memory_bus::MemoryBus::BANK_CUSTOM, 0xDF);
    assert_eq!(memory_bus::MemoryBus::BANK_CIA, 0xBF);
    assert_eq!(memory_bus::MemoryBus::BANK_RTC, 0xDC);
    assert_eq!(memory_bus::MemoryBus::RTC_START, 0xDC0000);
    assert_eq!(memory_bus::MemoryBus::RTC_END, 0xDC003F);
    assert_eq!(memory_bus::MemoryBus::CUSTOM_REG_OFFSET_MASK, 0x01FE);
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
    let res1 = mb
        .router()
        .read_word(0xDFF000 + u32::from(custom_reg::agnus::COPJMP1));
    assert_eq!(res1, BusResult::Ready(0xFFFF));
    assert_eq!(mb.agnus.copper.cop_pc, 0x0001_0000);

    // Reading COPJMP2 ($DFF08A) must strobe jump2 and return open-bus 0xFFFF
    let res2 = mb
        .router()
        .read_word(0xDFF000 + u32::from(custom_reg::agnus::COPJMP2));
    assert_eq!(res2, BusResult::Ready(0xFFFF));
    assert_eq!(mb.agnus.copper.cop_pc, 0x0002_0000);
}

#[test]
fn test_custom_byte_write_duplicates_byte_lanes() {
    let mut mb = TestMotherboard::new();

    // Writing a byte 0x42 to even address $DFF180 (COLOR00) must duplicate to 0x4242
    assert_eq!(
        mb.router()
            .write_byte(0xDFF000 + u32::from(custom_reg::denise::COLOR00), 0x42),
        BusResult::Ready(())
    );
    assert_eq!(mb.denise.color[0], 0);
    mb.denise.step_cck(mb.agnus.beam());
    // Denise color 0 should now be 0x4242 & 0x0FFF = 0x0242
    assert_eq!(mb.denise.color[0], 0x0242);

    // Writing a byte 0x55 to odd address $DFF181 (COLOR00) must also duplicate to 0x5555
    assert_eq!(
        mb.router()
            .write_byte(0xDFF000 + u32::from(custom_reg::denise::COLOR00) + 1, 0x55),
        BusResult::Ready(())
    );
    assert_eq!(mb.denise.color[0], 0x0242);
    mb.denise.step_cck(mb.agnus.beam());
    // Denise color 0 should now be 0x5555 & 0x0FFF = 0x0555
    assert_eq!(mb.denise.color[0], 0x0555);
}

#[test]
fn test_debug_read_and_register_method_conventions() {
    let mut mb = TestMotherboard::new();

    // 1. JOY0DAT / JOY1DAT via Denise fields & read_word_debug
    mb.denise.joy0dat = 0x1234;
    mb.denise.joy1dat = 0x5678;
    assert_eq!(
        mb.router()
            .read_word_debug(0xDFF000 + u32::from(custom_reg::denise::JOY0DAT)),
        0x1234
    );
    assert_eq!(
        mb.router()
            .read_word_debug(0xDFF000 + u32::from(custom_reg::denise::JOY1DAT)),
        0x5678
    );

    // 2. CLXDAT debug read does NOT clear collision bits
    mb.denise.clxdat = 0x00A5;
    assert_eq!(
        mb.router()
            .read_word_debug(0xDFF000 + u32::from(custom_reg::denise::CLXDAT)),
        0x00A5
    );
    assert_eq!(mb.denise.clxdat_debug(), 0x00A5);
    // Live read clears it
    assert_eq!(
        mb.router()
            .read_word(0xDFF000 + u32::from(custom_reg::denise::CLXDAT)),
        BusResult::Ready(0x00A5)
    );
    assert_eq!(mb.denise.clxdat_debug(), 0x0000);

    // 3. COPJMP1 strobe directly via strobe_copjmp1()
    mb.agnus.copper.cop1lc = 0x0003_4000;
    mb.agnus.strobe_copjmp1();
    assert_eq!(mb.agnus.copper.cop_pc, 0x0003_4000);

    // 4. CIA debug byte read does not clear or alter state
    mb.cia_a.pra = 0xAA;
    let debug_cia = mb.router().read_cia_byte_debug(0xBFE001);
    assert_eq!(debug_cia, mb.cia_a.pra());
    assert_eq!(debug_cia, 0xAA);
}

#[test]
fn test_chip_namespaced_custom_register_dispatch() {
    use config::custom_reg;

    let mut mb = TestMotherboard::new();

    // 1. Multi-chip decoded write: DMACON to Agnus and Paula
    mb.router()
        .write_custom_word(custom_reg::agnus::DMACON, 0x8201);
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    let _ = mb.paula.step_cck();
    let _ = mb.paula.step_cck();
    assert_eq!(mb.agnus.dmacon & 0x0201, 0x0201);
    assert_eq!(mb.paula.dma_enables & 0x0001, 0x0001);

    // 2. Denise-specific write: COLOR00
    mb.router()
        .write_custom_word(custom_reg::denise::COLOR00, 0x0ABC);
    assert_eq!(mb.denise.color[0], 0);
    mb.denise.step_cck(mb.agnus.beam());
    assert_eq!(mb.denise.color[0], 0x0ABC);

    // 3. Paula-specific write: INTENA
    mb.router()
        .write_custom_word(custom_reg::paula::INTENA, 0xC004);
    let _ = mb.paula.step_cck();
    assert_eq!(mb.paula.interrupts.intena & 0x4004, 0x4004);

    // 4. Agnus-specific read: VPOSR
    let vposr = mb.router().read_custom_word(custom_reg::agnus::VPOSR);
    assert_eq!(vposr, mb.agnus.vposr());

    // 5. Denise-specific read: JOY0DAT
    mb.denise.joy0dat = 0x4321;
    assert_eq!(
        mb.router().read_custom_word(custom_reg::denise::JOY0DAT),
        0x4321
    );

    // 6. Paula-specific read: INTENAR
    assert_eq!(
        mb.router().read_custom_word(custom_reg::paula::INTENAR),
        mb.paula.intenar()
    );

    // 7. Additional custom word dispatch: COLOR01 write and debug readback
    mb.router()
        .write_custom_word(custom_reg::denise::COLOR01, 0x0F00);
    assert_eq!(mb.denise.color[1], 0);
    mb.denise.step_cck(mb.agnus.beam());
    assert_eq!(mb.denise.color[1], 0x0F00);
    assert_eq!(
        mb.router()
            .read_custom_word_debug(custom_reg::denise::JOY0DAT),
        0x4321
    );
}

#[test]
fn test_agnus_writes_are_explicit_and_reserved_offset_is_ignored() {
    let mut mb = TestMotherboard::new();
    let agnus_writes = [
        (custom_reg::agnus::REFPTR, 0x1234),
        (custom_reg::agnus::VPOSW, 0x0001),
        (custom_reg::agnus::VHPOSW, 0x2345),
        (custom_reg::agnus::COPINS, 0x5678),
    ];

    for (offset, value) in agnus_writes {
        mb.router().write_custom_word(offset, value);
        assert!(mb
            .agnus
            .mutations
            .iter()
            .flatten()
            .any(|mutation| { mutation.reg_offset == offset && mutation.value == value }));
    }

    // $07C is reserved and must not consume an Agnus mutation slot.
    mb.router().write_custom_word(0x07C, 0x9ABC);
    assert!(mb
        .agnus
        .mutations
        .iter()
        .flatten()
        .all(|mutation| mutation.reg_offset != 0x07C));
}

#[test]
fn test_sprite_position_and_control_writes_reach_both_chips() {
    let mut mb = TestMotherboard::new();

    for sprite in 0..8 {
        for (register, value) in [(0, 0x2100 + sprite), (2, 0x4300 + sprite)] {
            let offset = custom_reg::denise::SPR0POS + sprite * 8 + register;
            mb.router().write_custom_word(offset, value);

            let first = mb.agnus.step_cck();
            let second = mb.agnus.step_cck();
            assert!(first
                .into_iter()
                .chain(second)
                .any(|write| write == Some((offset, value))));

            let beam = mb.agnus.beam();
            let _ = mb.denise.step_cck(beam);
            let channel = &mb.denise.sprites.channels[usize::from(sprite)];
            if register == 0 {
                assert_eq!(mb.agnus.sprpos[usize::from(sprite)], value);
                assert_eq!(channel.pos, value);
            } else {
                assert_eq!(mb.agnus.sprctl[usize::from(sprite)], value);
                assert_eq!(channel.ctl, value);
            }
        }
    }
}

#[test]
fn ocs_reserved_diwhigh_offset_does_not_change_denise_state() {
    let mut mb = TestMotherboard::new();
    let untouched = mb.denise.clone();
    assert!(mb.router().write_custom_word(0x1e4, 0x2020));
    assert_eq!(mb.denise, untouched, "OCS Denise ignores reserved $1E4");
}
