#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration & Unit Tests for Motherboard MemoryBus Router
//!
//! Validates 24-bit physical address routing across PhysicalMemory,
//! Custom Chips ($DFFxxx), CIAs ($BFDxxx/$BFExxx), and RTC ($DCxxxx).

use agnus::Agnus;
use cia::Cia;
use config::{cia_reg, custom_reg};
use config::{A500Config, A500Preset, RtcModel, VideoStandard};
use denise::{Denise, DeniseModel};
use memory_bus::MemoryBus;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
use rtc::RtcMsm6242b;

// These routing tests inspect completed transfers. Phase/retry tests below use raw calls.
fn clock_cias(bus: &mut MemoryBus<'_>) {
    bus.cia_a.step_cck();
    bus.cia_b.step_cck();
    if bus.cia_a.poll_pra_output().is_some() {
        MemoryBus::update_ciaa_overlay(bus.mem, bus.cia_a);
    }
}

fn read_byte_debug(bus: &MemoryBus<'_>, addr: u32) -> u8 {
    bus.read_word_debug(addr & !1).to_be_bytes()[(addr & 1) as usize]
}

fn sync_read_byte(bus: &mut MemoryBus<'_>, addr: u32) -> BusResult<u8> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(_) = bus.read_byte(addr) {
            return result;
        }
        clock_cias(bus);
    }
    panic!("CIA access did not complete");
}

fn sync_read_word(bus: &mut MemoryBus<'_>, addr: u32) -> BusResult<u16> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(_) = bus.read_word(addr) {
            return result;
        }
        clock_cias(bus);
    }
    panic!("CIA access did not complete");
}

fn sync_write_byte(bus: &mut MemoryBus<'_>, addr: u32, value: u8) -> BusResult<()> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(_) = bus.write_byte(addr, value) {
            clock_cias(bus);
            return result;
        }
        clock_cias(bus);
    }
    panic!("CIA access did not complete");
}

fn sync_write_word(bus: &mut MemoryBus<'_>, addr: u32, value: u16) -> BusResult<()> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(_) = bus.write_word(addr, value) {
            clock_cias(bus);
            return result;
        }
        clock_cias(bus);
    }
    panic!("CIA access did not complete");
}

struct TestMotherboard {
    mem: PhysicalMemory,
    agnus: Agnus,
    denise: Denise,
    paula: Paula,
    cia_a: Cia,
    cia_b: Cia,
    rtc: RtcMsm6242b,
}

#[test]
fn shared_bus_refusal_retains_live_read_and_custom_write_effects_until_acceptance() {
    let mut mb = TestMotherboard::new();
    mb.mem.chip_ram_blocked = true;
    mb.denise.clxdat = 0x0042;
    for _ in 0..3 {
        assert_eq!(mb.router().read_word(0xdff00e), BusResult::WaitState);
        assert_eq!(mb.router().read_byte(0xde000f), BusResult::WaitState);
        assert_eq!(
            mb.router().write_word(0xdff180, 0x0abc),
            BusResult::WaitState
        );
        assert_eq!(mb.router().write_byte(0xde0180, 0xab), BusResult::WaitState);
        assert_eq!(
            mb.router().write_tas_byte(0xdff180, 0xab),
            BusResult::WaitState
        );
        assert_eq!(mb.denise.clxdat, 0x0042);
        assert!(mb.denise.mutations.iter().all(Option::is_none));
        assert!(mb.agnus.dma.cpu_request_pending);
        assert_eq!(mb.agnus.dma.cpu_starvation_counter, 1);
    }
    mb.mem.chip_ram_blocked = false;
    assert_eq!(mb.router().read_word(0xdff00e), BusResult::Ready(0x0042));
    assert_eq!(mb.denise.clxdat, 0);
    assert!(!mb.agnus.dma.cpu_request_pending);
    assert_eq!(
        mb.router().write_word(0xdff180, 0x0abc),
        BusResult::Ready(())
    );
    assert_eq!(mb.denise.mutations.iter().flatten().count(), 1);
    mb.denise.step_cck();
    assert_eq!(mb.denise.color[0], 0x0abc);
}

#[test]
fn ram_handlers_retain_blocked_byte_word_and_tas_transfers() {
    let mut mb = TestMotherboard::new();
    for addr in [0x001000, 0xc01000] {
        assert_eq!(mb.router().write_word(addr, 0x1234), BusResult::Ready(()));
        mb.mem.chip_ram_blocked = true;
        assert_eq!(mb.router().read_byte(addr), BusResult::WaitState);
        assert!(mb.agnus.dma.cpu_request_pending);
        assert_eq!(mb.router().read_word(addr), BusResult::WaitState);
        assert_eq!(mb.router().write_byte(addr, 0x56), BusResult::WaitState);
        assert_eq!(mb.router().write_word(addr, 0x789a), BusResult::WaitState);
        assert_eq!(mb.router().write_tas_byte(addr, 0xff), BusResult::WaitState);
        assert_eq!(mb.mem.read_word_debug(addr), 0x1234);
        assert_eq!(mb.agnus.dma.cpu_starvation_counter, 1);

        mb.mem.chip_ram_blocked = false;
        assert_eq!(mb.router().write_tas_byte(addr, 0xff), BusResult::Ready(()));
        assert!(!mb.agnus.dma.cpu_request_pending);
        assert_eq!(mb.agnus.dma.cpu_starvation_counter, 0);
        assert_eq!(mb.router().read_word(addr), BusResult::Ready(0x1234));
        assert_eq!(mb.router().write_byte(addr, 0x56), BusResult::Ready(()));
        assert_eq!(mb.router().read_byte(addr), BusResult::Ready(0x56));
        assert_eq!(mb.router().write_word(addr, 0x789a), BusResult::Ready(()));
        assert_eq!(mb.router().read_word(addr), BusResult::Ready(0x789a));
    }
}

#[test]
fn kickstart_overlay_uses_rom_access_rules_while_chip_bus_is_blocked() {
    let mut mb = TestMotherboard::new();
    mb.mem.write_bytes_debug(0xf80000, &[0xab, 0xcd]);
    mb.mem.chip_ram_blocked = true;
    assert_eq!(mb.router().read_word(0), BusResult::WaitState);
    assert!(mb.agnus.dma.cpu_request_pending);

    mb.mem.map_kickstart_to_low_memory();
    for addr in [0, 0xf80000] {
        assert_eq!(mb.router().read_byte(addr), BusResult::Ready(0xab));
        assert!(!mb.agnus.dma.cpu_request_pending);
        assert_eq!(mb.agnus.dma.cpu_starvation_counter, 0);
        assert_eq!(mb.router().read_word(addr), BusResult::Ready(0xabcd));
        assert_eq!(mb.router().write_byte(addr, 0x56), BusResult::Ready(()));
        assert_eq!(mb.router().write_word(addr, 0x789a), BusResult::Ready(()));
        assert_eq!(mb.router().write_tas_byte(addr, 0xff), BusResult::Ready(()));
        assert_eq!(mb.router().read_word(addr), BusResult::Ready(0xabcd));
    }
    mb.mem.map_chip_ram_to_low_memory();
    assert_eq!(mb.router().read_word(0), BusResult::WaitState);
    assert!(mb.agnus.dma.cpu_request_pending);
}

#[test]
fn cia_and_rtc_accesses_do_not_wait_for_the_chip_bus() {
    let mut mb = TestMotherboard::new();
    mb.mem.chip_ram_blocked = true;
    assert_eq!(mb.router().read_word(0), BusResult::WaitState);
    assert!(mb.agnus.dma.cpu_request_pending);

    // CIA still waits for E-clock acceptance, without requesting an Agnus slot.
    assert_eq!(mb.router().read_byte(0xbfec01), BusResult::WaitState);
    assert!(!mb.agnus.dma.cpu_request_pending);
    assert_eq!(mb.agnus.dma.cpu_starvation_counter, 0);
    assert_eq!(
        sync_read_byte(&mut mb.router(), 0xbfec01),
        BusResult::Ready(0)
    );
    assert_eq!(
        sync_write_byte(&mut mb.router(), 0xbfec01, 0x5a),
        BusResult::Ready(())
    );
    assert_eq!(
        sync_read_byte(&mut mb.router(), 0xbfec01),
        BusResult::Ready(0x5a)
    );
    assert!(!mb.agnus.dma.cpu_request_pending);

    assert_eq!(mb.router().write_byte(0xdc003d, 0x05), BusResult::Ready(()));
    assert_eq!(mb.router().read_byte(0xdc003d), BusResult::Ready(0x05));
    assert_eq!(
        mb.router().write_word(0xdc003c, 0x0004),
        BusResult::Ready(())
    );
    assert_eq!(mb.router().read_word(0xdc003c), BusResult::Ready(0xff04));
    assert!(!mb.agnus.dma.cpu_request_pending);
    assert!(mb.mem.chip_ram_blocked);
}

#[test]
fn granted_custom_dma_write_does_not_use_cpu_contention_guard() {
    let mut mb = TestMotherboard::new();
    mb.mem.chip_ram_blocked = true;
    assert_eq!(
        mb.router().write_word(0xdff180, 0x0abc),
        BusResult::WaitState
    );
    assert!(mb.agnus.dma.cpu_request_pending);
    assert!(mb
        .router()
        .write_custom_word(custom_reg::denise::REGISTER_COLOR00, 0x0abc));
    assert!(mb.agnus.dma.cpu_request_pending);
    assert_eq!(mb.denise.mutations.iter().flatten().count(), 1);
    mb.denise.step_cck();
    assert_eq!(mb.denise.color[0], 0x0abc);
}

#[test]
fn private_access_and_debug_reads_do_not_create_shared_bus_requests() {
    let mut mb = TestMotherboard::with_config(A500Config::from_preset(
        config::A500Preset::ExpandedPowerUser,
        VideoStandard::Pal,
    ));
    mb.mem.chip_ram_blocked = true;
    assert!(matches!(
        mb.router().read_word(0x200000),
        BusResult::Ready(_)
    ));
    assert!(matches!(
        mb.router().read_word(0xf80000),
        BusResult::Ready(_)
    ));
    mb.denise.clxdat = 0x1234;
    assert_eq!(mb.router().read_word_debug(0xdff00e), 0x1234);
    assert_eq!(mb.denise.clxdat, 0x1234);
    assert!(!mb.agnus.dma.cpu_request_pending);
    assert_eq!(mb.agnus.dma.cpu_starvation_counter, 0);
    assert_eq!(mb.router().read_word(0xc00000), BusResult::WaitState);
    assert!(mb.agnus.dma.cpu_request_pending);
}

impl TestMotherboard {
    fn new() -> Self {
        Self::with_config(A500Config::standard_1mb(VideoStandard::Pal))
    }

    fn with_config(config: A500Config) -> Self {
        let mut mem = PhysicalMemory::from_config(config.clone());
        mem.map_chip_ram_to_low_memory();
        Self {
            mem,
            agnus: Agnus::new(config.agnus_model()),
            denise: Denise::new(DeniseModel::Ocs8362),
            paula: Paula::new(),
            cia_a: Cia::new(),
            cia_b: Cia::new(),
            rtc: RtcMsm6242b::new(config.rtc()),
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
fn cia_acceptance_tracks_each_eclock_phase_without_duplicate_writes() {
    // Even CPU-phase entries of vAmiga Agnus::syncWithEClock, in CCK units.
    for (phase, expected_wait) in [(0, 6), (1, 5), (2, 4), (3, 3), (4, 7)] {
        for addr in [cia_reg::cia_a::REGISTER_SDR, cia_reg::cia_b::REGISTER_SDR] {
            let mut motherboard = TestMotherboard::new();
            let mut bus = motherboard.router();
            for _ in 0..phase {
                clock_cias(&mut bus);
            }
            assert_eq!(bus.write_byte(addr, 0x5a), BusResult::WaitState);
            for elapsed in 1..=expected_wait {
                assert!(bus.cia_a.mutations.iter().all(Option::is_none));
                assert!(bus.cia_b.mutations.iter().all(Option::is_none));
                assert_eq!(read_byte_debug(&bus, addr), 0);
                clock_cias(&mut bus);
                let expected = if elapsed == expected_wait {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                };
                assert_eq!(
                    bus.write_byte(addr, 0x5a),
                    expected,
                    "phase {phase}, elapsed {elapsed}"
                );
            }
            assert_eq!(bus.cia_a.eclock_phase, 1);
            assert_eq!(bus.cia_b.eclock_phase, 1);
            assert_eq!(
                bus.cia_a.mutations.iter().flatten().count()
                    + bus.cia_b.mutations.iter().flatten().count(),
                1
            );
            assert_eq!(read_byte_debug(&bus, addr), 0);
            clock_cias(&mut bus);
            assert_eq!(read_byte_debug(&bus, addr), 0x5a);
            for _ in 0..5 {
                clock_cias(&mut bus);
            }
            assert!(bus.cia_a.mutations.iter().all(Option::is_none));
            assert!(bus.cia_b.mutations.iter().all(Option::is_none));
        }
    }
}

#[test]
fn each_canonical_cia_write_selects_one_receiver_and_one_capture() {
    // The independent hardware register list includes both chip instances and all halves.
    let rows = [
        (cia_reg::cia_a::REGISTER_PRA, 0x5a),
        (cia_reg::cia_a::REGISTER_PRB, 0x5a),
        (cia_reg::cia_a::REGISTER_DDRA, 0x5a),
        (cia_reg::cia_a::REGISTER_DDRB, 0x5a),
        (cia_reg::cia_a::REGISTER_TALO, 0xff),
        (cia_reg::cia_a::REGISTER_TAHI, 0x5a),
        (cia_reg::cia_a::REGISTER_TBLO, 0xff),
        (cia_reg::cia_a::REGISTER_TBHI, 0x5a),
        (cia_reg::cia_a::REGISTER_TODLO, 0x5a),
        (cia_reg::cia_a::REGISTER_TODMID, 0x5a),
        (cia_reg::cia_a::REGISTER_TODHI, 0x5a),
        (cia_reg::cia_a::REGISTER_SDR, 0x5a),
        (cia_reg::cia_a::REGISTER_ICR, 0),
        (cia_reg::cia_a::REGISTER_CRA, 0x4a),
        (cia_reg::cia_a::REGISTER_CRB, 0x4a),
        (cia_reg::cia_b::REGISTER_PRA, 0x5a),
        (cia_reg::cia_b::REGISTER_PRB, 0x5a),
        (cia_reg::cia_b::REGISTER_DDRA, 0x5a),
        (cia_reg::cia_b::REGISTER_DDRB, 0x5a),
        (cia_reg::cia_b::REGISTER_TALO, 0xff),
        (cia_reg::cia_b::REGISTER_TAHI, 0x5a),
        (cia_reg::cia_b::REGISTER_TBLO, 0xff),
        (cia_reg::cia_b::REGISTER_TBHI, 0x5a),
        (cia_reg::cia_b::REGISTER_TODLO, 0x5a),
        (cia_reg::cia_b::REGISTER_TODMID, 0x5a),
        (cia_reg::cia_b::REGISTER_TODHI, 0x5a),
        (cia_reg::cia_b::REGISTER_SDR, 0x5a),
        (cia_reg::cia_b::REGISTER_ICR, 0),
        (cia_reg::cia_b::REGISTER_CRA, 0x4a),
        (cia_reg::cia_b::REGISTER_CRB, 0x4a),
    ];
    for (addr, expected) in rows {
        let mut motherboard = TestMotherboard::new();
        // Port readback observes pins; this test drives the written latch outward.
        for cia in [&mut motherboard.cia_a, &mut motherboard.cia_b] {
            if addr & 0x0f00 == 0 {
                cia.ddra = 0xff;
            } else if addr & 0x0f00 == 0x0100 {
                cia.ddrb = 0xff;
            }
        }
        let mut bus = motherboard.router();
        assert_eq!(
            bus.write_byte(addr, 0x5a),
            BusResult::WaitState,
            "{addr:06x}"
        );
        for _ in 0..6 {
            clock_cias(&mut bus);
        }
        assert_eq!(
            bus.write_byte(addr, 0x5a),
            BusResult::Ready(()),
            "{addr:06x}"
        );
        let selected = if addr & 1 == 1 {
            &bus.cia_a
        } else {
            &bus.cia_b
        };
        let other = if addr & 1 == 1 {
            &bus.cia_b
        } else {
            &bus.cia_a
        };
        assert!(other.mutations.iter().all(Option::is_none), "{addr:06x}");
        let pending: Vec<_> = selected.mutations.iter().flatten().collect();
        assert_eq!(pending.len(), 1, "{addr:06x}");
        assert_eq!(pending[0].reg_offset, ((addr >> 8) & 15) as u16);
        assert_eq!(pending[0].value, 0x5a);
        clock_cias(&mut bus);
        assert_eq!(read_byte_debug(&bus, addr), expected, "{addr:06x}");
        assert_eq!(
            sync_read_byte(&mut bus, addr),
            BusResult::Ready(expected),
            "{addr:06x}"
        );
    }
}

#[test]
fn refused_live_icr_reads_have_no_acknowledgment_effect() {
    let mut motherboard = TestMotherboard::new();
    motherboard.cia_a.icr_data = 0x81;
    let mut bus = motherboard.router();
    assert_eq!(
        bus.read_byte(cia_reg::cia_a::REGISTER_ICR),
        BusResult::WaitState
    );
    for _ in 0..5 {
        clock_cias(&mut bus);
        let before_debug = bus.cia_a.clone();
        assert_eq!(read_byte_debug(&bus, cia_reg::cia_a::REGISTER_ICR), 0x81);
        assert_eq!(*bus.cia_a, before_debug);
        assert_eq!(
            bus.read_byte(cia_reg::cia_a::REGISTER_ICR),
            BusResult::WaitState
        );
        assert_eq!(bus.cia_a.icr_debug(), 0x81);
    }
    clock_cias(&mut bus);
    assert_eq!(
        bus.read_byte(cia_reg::cia_a::REGISTER_ICR),
        BusResult::Ready(0x81)
    );
    assert_eq!(bus.cia_a.icr_debug(), 0);
    assert_eq!(
        bus.read_byte(cia_reg::cia_a::REGISTER_ICR),
        BusResult::WaitState
    );
}

#[test]
fn test_custom_register_broadcast_and_routing() {
    let mut mb = TestMotherboard::new();
    {
        let mut bus = mb.router();
        // Write DMACON ($DFF096): set master DMA enable and Copper DMA enable ($8280)
        let _ = bus.write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACON),
            0x8280,
        );
    }
    // Check that Agnus staged DMACON mutation with physical propagation delay
    let agnus_staged = mb
        .agnus
        .mutations
        .iter()
        .flatten()
        .any(|m| m.reg_offset == custom_reg::agnus::REGISTER_DMACON && m.value == 0x8280);
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
    let _ = sync_write_byte(&mut bus, 0xBFEE01, 0x55);
    assert_eq!(sync_read_byte(&mut bus, 0xBFEE01), BusResult::Ready(0x45));

    // Even byte in CIA-A range returns open bus $FF
    assert_eq!(sync_read_byte(&mut bus, 0xBFEE00), BusResult::Ready(0xFF));

    // CIA-B is at even byte addresses ($BFD000..$BFDF00)
    // Write CRB at offset $F -> $BFD000 + ($F << 8) = $BFDF00
    let _ = sync_write_byte(&mut bus, 0xBFDF00, 0xAA);
    assert_eq!(sync_read_byte(&mut bus, 0xBFDF00), BusResult::Ready(0xAA));

    // Odd byte in CIA-B range returns open bus $FF
    assert_eq!(sync_read_byte(&mut bus, 0xBFDF01), BusResult::Ready(0xFF));
}

#[test]
fn test_all_cia_addresses_select_their_registers() {
    let mut mb = TestMotherboard::new();
    // The CIA-B selector pattern enables timer PB6. Establish its high toggle
    // output before populating the address-decoder fixture below.
    assert!(mb.cia_b.set_cra(0x07));
    mb.cia_b.step_cck();
    mb.cia_b.step_cck();
    for (cia, base) in [(&mut mb.cia_a, 0x20u8), (&mut mb.cia_b, 0x40u8)] {
        cia.pra = base;
        cia.prb = base + 1;
        cia.set_input_pins_a(base, 0xff);
        cia.pins.port_b_input = base + 1;
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
        (cia_reg::cia_a::REGISTER_PRA, 0x20),
        (cia_reg::cia_a::REGISTER_PRB, 0x21),
        (cia_reg::cia_a::REGISTER_DDRA, 0x22),
        (cia_reg::cia_a::REGISTER_DDRB, 0x23),
        (cia_reg::cia_a::REGISTER_TALO, 0x24),
        (cia_reg::cia_a::REGISTER_TAHI, 0x25),
        (cia_reg::cia_a::REGISTER_TBLO, 0x26),
        (cia_reg::cia_a::REGISTER_TBHI, 0x27),
        (cia_reg::cia_a::REGISTER_TODLO, 0x28),
        (cia_reg::cia_a::REGISTER_TODMID, 0x29),
        (cia_reg::cia_a::REGISTER_TODHI, 0x2A),
        (cia_reg::cia_a::REGISTER_SDR, 0x2C),
        (cia_reg::cia_a::REGISTER_ICR, 0x2D),
        (cia_reg::cia_a::REGISTER_CRA, 0x2E),
        (cia_reg::cia_a::REGISTER_CRB, 0x2F),
        (cia_reg::cia_b::REGISTER_PRA, 0x40),
        (cia_reg::cia_b::REGISTER_PRB, 0x41),
        (cia_reg::cia_b::REGISTER_DDRA, 0x42),
        (cia_reg::cia_b::REGISTER_DDRB, 0x43),
        (cia_reg::cia_b::REGISTER_TALO, 0x44),
        (cia_reg::cia_b::REGISTER_TAHI, 0x45),
        (cia_reg::cia_b::REGISTER_TBLO, 0x46),
        (cia_reg::cia_b::REGISTER_TBHI, 0x47),
        (cia_reg::cia_b::REGISTER_TODLO, 0x48),
        (cia_reg::cia_b::REGISTER_TODMID, 0x49),
        (cia_reg::cia_b::REGISTER_TODHI, 0x4A),
        (cia_reg::cia_b::REGISTER_SDR, 0x4C),
        (cia_reg::cia_b::REGISTER_ICR, 0x4D),
        (cia_reg::cia_b::REGISTER_CRA, 0x4E),
        (cia_reg::cia_b::REGISTER_CRB, 0x4F),
    ] {
        assert_eq!(read_byte_debug(&bus, addr), expected, "address {addr:06X}");
    }
}

#[test]
fn test_cia_register_addresses_mirror_on_unused_low_bits() {
    let mut mb = TestMotherboard::new();
    let mut bus = mb.router();

    let _ = sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_CRA, 0x55);
    let _ = sync_write_byte(&mut bus, cia_reg::cia_b::REGISTER_CRA, 0x2A);

    // A7..A1 do not participate in CIA register selection.
    assert_eq!(
        sync_read_byte(&mut bus, cia_reg::cia_a::REGISTER_CRA + 2),
        BusResult::Ready(0x45)
    );
    assert_eq!(
        sync_read_byte(&mut bus, cia_reg::cia_b::REGISTER_CRA + 2),
        BusResult::Ready(0x2A)
    );
    assert_eq!(
        read_byte_debug(&bus, cia_reg::cia_a::REGISTER_CRA + 2),
        0x45
    );
    assert_eq!(
        read_byte_debug(&bus, cia_reg::cia_b::REGISTER_CRA + 2),
        0x2A
    );

    let _ = sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_CRA + 2, 0x11);
    let _ = sync_write_byte(&mut bus, cia_reg::cia_b::REGISTER_CRA + 2, 0x12);
    assert_eq!(
        sync_read_byte(&mut bus, cia_reg::cia_a::REGISTER_CRA),
        BusResult::Ready(0x01)
    );
    assert_eq!(
        sync_read_byte(&mut bus, cia_reg::cia_b::REGISTER_CRA),
        BusResult::Ready(0x02)
    );

    // Selector $B is unconnected on both chips.
    assert_eq!(sync_read_byte(&mut bus, 0xBFEB01), BusResult::Ready(0xFF));
    assert_eq!(sync_read_byte(&mut bus, 0xBFDB00), BusResult::Ready(0xFF));
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

    // An input PA0 is pulled high even when its PRA latch contains zero.
    assert_eq!(
        sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_PRA, 0),
        BusResult::Ready(())
    );
    assert_eq!(bus.read_word(0), BusResult::Ready(0x1122));

    // Driving the zero latch onto PA0 removes ROM immediately after the DDRA write.
    assert_eq!(
        sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_DDRA, 1),
        BusResult::Ready(())
    );
    assert_eq!(bus.read_word(0x000000), BusResult::Ready(0x0000));

    // OVL high selects ROM; OVL low selects Chip RAM.
    assert_eq!(
        sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_PRA, 1),
        BusResult::Ready(())
    );
    assert_eq!(bus.read_word(0), BusResult::Ready(0x1122));
    assert_eq!(
        sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_PRA, 0),
        BusResult::Ready(())
    );
    assert_eq!(bus.read_word(0), BusResult::Ready(0x0000));

    // Returning PA0 to input releases it to the pull-up and restores ROM.
    assert_eq!(
        sync_write_byte(&mut bus, cia_reg::cia_a::REGISTER_DDRA, 0),
        BusResult::Ready(())
    );
    assert_eq!(bus.read_word(0), BusResult::Ready(0x1122));
}

#[test]
fn test_ciaa_overlay_word_mirrors_and_ciab_isolation() {
    let mut mb = TestMotherboard::new();
    mb.mem.map_kickstart_to_low_memory();

    // A mirrored CIA-A word write uses its odd lane to drive PA0 low.
    assert_eq!(
        sync_write_word(
            &mut mb.router(),
            (cia_reg::cia_a::REGISTER_DDRA & !1) | 0xFE,
            0xFF01
        ),
        BusResult::Ready(())
    );
    assert!(!mb.mem.is_low_memory_overlay_active());

    // Neither CIA-B's direction nor its output latch controls the overlay.
    for value in [0x00, 0xFF] {
        for reg in [cia_reg::cia_b::REGISTER_PRA, cia_reg::cia_b::REGISTER_DDRA] {
            assert_eq!(
                sync_write_byte(&mut mb.router(), reg, value),
                BusResult::Ready(())
            );
            assert!(!mb.mem.is_low_memory_overlay_active());
        }
    }

    assert_eq!(
        sync_write_word(
            &mut mb.router(),
            (cia_reg::cia_a::REGISTER_PRA & !1) | 0xFE,
            0x0001
        ),
        BusResult::Ready(())
    );
    assert!(mb.mem.is_low_memory_overlay_active());
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
        let _ = bus.write_word(
            0xDFF000 + u32::from(custom_reg::denise::REGISTER_COLOR00),
            0x0F00,
        );
        // Paula: INTENA ($DFF09A) - set bit 15 (SET) | bit 14 (INTEN) | bit 0 (TBE) = 0xC001
        let _ = bus.write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_INTENA),
            0xC001,
        );
        // Agnus: BLTCON0 ($DFF040)
        let _ = bus.write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_BLTCON0),
            0x09F0,
        );
        // Shared: BPLCON0 ($DFF100)
        let _ = bus.write_word(
            0xDFF000 + u32::from(custom_reg::denise::REGISTER_BPLCON0),
            0x1200,
        );
        // CIA-A: CRA ($BFEE01)
        let _ = sync_write_byte(&mut bus, 0xBFEE01, 0x55);
        // CIA-B: CRB ($BFDF00)
        let _ = sync_write_byte(&mut bus, 0xBFDF00, 0xAA);
    }

    assert_eq!(mb.denise.color[0], 0);
    mb.denise.step_cck();
    assert_eq!(mb.denise.color[0], 0x0F00);
    // Step Paula 1 CCK so staged INTENA write matures
    let _ = mb.paula.step_cck();
    assert_eq!(mb.paula.interrupts.intena, 0x4001);
    // Step Agnus 2 CCKs so staged BLTCON0 write matures
    let _ = mb.agnus.step_cck();
    let _ = mb.agnus.step_cck();
    assert_eq!(mb.agnus.blitter.bltcon0, 0x09F0);
    assert_eq!(mb.cia_a.cra, 0x45);
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
    assert_eq!(sync_read_byte(&mut bus, 0xBFEE00), BusResult::Ready(0xFF));
    assert_eq!(sync_read_byte(&mut bus, 0xBFDF01), BusResult::Ready(0xFF));
    assert_eq!(sync_read_byte(&mut bus, 0xBFEB01), BusResult::Ready(0xFF));
    assert_eq!(sync_read_byte(&mut bus, 0xBFDB00), BusResult::Ready(0xFF));
    assert_eq!(
        sync_write_byte(&mut bus, 0xBFEE01, 0x55),
        BusResult::Ready(())
    );
    assert_eq!(sync_read_word(&mut bus, 0xBFEE00), BusResult::Ready(0xFF45));

    // The RTC drives odd addresses only; the rest of its bank is unmapped.
    assert_eq!(bus.write_byte(0xDC0001, 0x07), BusResult::Ready(()));
    assert_eq!(bus.read_byte(0xDC0000), BusResult::Ready(0xFF));
    assert_eq!(bus.read_word(0xDC0000), BusResult::Ready(0xFF07));
    assert_eq!(bus.read_word(0xDC0040), BusResult::Ready(0xFFFF));

    // Early DMA addresses and write-only registers do not return their stored data.
    for offset in [
        custom_reg::agnus::REGISTER_BLTDDAT,
        custom_reg::paula::REGISTER_DSKDATR,
        custom_reg::agnus::REGISTER_BLTCON0,
        custom_reg::denise::REGISTER_COLOR00,
        custom_reg::paula::REGISTER_AUD0LEN,
    ] {
        let addr = 0xDFF000 + u32::from(offset);
        assert_eq!(bus.read_word(addr), BusResult::Ready(0xFFFF), "{addr:06X}");
        assert_eq!(bus.read_byte(addr), BusResult::Ready(0xFF), "{addr:06X}");
    }
    assert_eq!(
        bus.read_word(0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COPJMP1)),
        BusResult::Ready(0xFFFF)
    );
    assert_eq!(
        bus.read_word(0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COPJMP2)),
        BusResult::Ready(0xFFFF)
    );

    // A populated bank and an active register still return driven data.
    assert_eq!(bus.write_word(0x001000, 0x1234), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0x1234));
    assert_eq!(sync_read_byte(&mut bus, 0xBFEE01), BusResult::Ready(0x45));
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

// Roadmap MEM-GARY-01: expected selection follows A500 schematic sheet 6,
// HRM Appendix F address decode, and the approved Old Gary contract.
#[test]
fn gary_cia_banks_selection_lanes_and_byte_write_broadcast() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for preset in [
            A500Preset::Bare512k,
            A500Preset::Standard1Mb,
            A500Preset::ExpandedPowerUser,
        ] {
            let mut mb = TestMotherboard::with_config(A500Config::from_preset(preset, standard));
            for bank in 0xa0..=0xbf {
                for block in 0..16 {
                    // A15/A14 and A7..A1 are mirrors; selector C is nonzero.
                    let addr = (bank << 16) | (block << 12) | 0xcfe;
                    let select_a = block & 1 == 0;
                    let select_b = block & 2 == 0;
                    mb.cia_a.sdr = 0x35;
                    mb.cia_b.sdr = 0xa6;
                    let high = if select_b { 0xa6 } else { 0xff };
                    let low = if select_a { 0x35 } else { 0xff };
                    let mut bus = mb.router();
                    assert_eq!(
                        bus.read_word_debug(addr),
                        u16::from_be_bytes([high, low]),
                        "{addr:06x}"
                    );
                    assert_eq!(
                        sync_read_word(&mut bus, addr),
                        BusResult::Ready(u16::from_be_bytes([high, low])),
                        "{addr:06x}"
                    );
                    assert_eq!(sync_read_byte(&mut bus, addr), BusResult::Ready(high));
                    assert_eq!(sync_read_byte(&mut bus, addr | 1), BusResult::Ready(low));
                    // Both byte parities drive every selected receiver.
                    for parity in [0, 1] {
                        assert_eq!(
                            sync_write_byte(&mut bus, addr | parity, 0x59 + parity as u8),
                            BusResult::Ready(())
                        );
                        assert_eq!(
                            bus.cia_a.sdr,
                            if select_a { 0x59 + parity as u8 } else { 0x35 }
                        );
                        assert_eq!(
                            bus.cia_b.sdr,
                            if select_b { 0x59 + parity as u8 } else { 0xa6 }
                        );
                    }
                    assert_eq!(
                        sync_write_word(&mut bus, addr, 0x8765),
                        BusResult::Ready(())
                    );
                    assert_eq!(bus.cia_a.sdr, if select_a { 0x65 } else { 0x35 });
                    assert_eq!(bus.cia_b.sdr, if select_b { 0x87 } else { 0xa6 });
                }
            }
        }
    }
}

#[test]
fn gary_dual_cia_word_acceptance_is_atomic() {
    let mut mb = TestMotherboard::new();
    mb.cia_a.icr_data = 0x81;
    mb.cia_b.icr_data = 0x92;
    for _ in 0..6 {
        assert_eq!(mb.router().read_word(0xbfcd00), BusResult::WaitState);
        assert_eq!((mb.cia_a.icr_data, mb.cia_b.icr_data), (0x81, 0x92));
        clock_cias(&mut mb.router());
    }
    assert_eq!(mb.router().read_word(0xbfcd00), BusResult::Ready(0x9281));
    assert_eq!((mb.cia_a.icr_data, mb.cia_b.icr_data), (0, 0));
    assert_eq!((mb.cia_a.bus_wait_cck, mb.cia_b.bus_wait_cck), (None, None));
}

#[test]
fn gary_custom_de_bank_shares_live_debug_and_delayed_paths() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for preset in [
            A500Preset::Bare512k,
            A500Preset::Standard1Mb,
            A500Preset::ExpandedPowerUser,
        ] {
            let mut mb = TestMotherboard::with_config(A500Config::from_preset(preset, standard));
            mb.agnus.dma.dmacon = 0x0211;
            assert_eq!(mb.router().read_word(0x01de_f202), BusResult::Ready(0x2211));
            assert_eq!(mb.router().read_word_debug(0xde0002), 0x2211);
            assert_eq!(
                mb.router().write_word(0xde_f096, 0x8002),
                BusResult::Ready(())
            );
            assert_eq!(mb.agnus.dma.dmacon, 0x0211);
            let _ = mb.agnus.step_cck();
            assert_eq!(mb.agnus.dma.dmacon, 0x0211);
            let _ = mb.agnus.step_cck();
            assert_eq!(mb.agnus.dma.dmacon, 0x0213);
            mb.denise.clxdat = 0x1234;
            assert_eq!(mb.router().read_word_debug(0xde000e), 0x1234);
            assert_eq!(mb.router().read_byte(0xde000f), BusResult::Ready(0x34));
            assert_eq!(mb.denise.clxdat, 0);
        }
    }
}

#[test]
fn gary_selected_cia_accepts_byte_writes_on_either_lane() {
    let mut mb = TestMotherboard::new();
    assert_eq!(
        sync_write_byte(&mut mb.router(), 0xbfec00, 0x5a),
        BusResult::Ready(())
    );
    assert_eq!(mb.cia_a.sdr, 0x5a);
    assert_eq!(mb.cia_b.sdr, 0);
}

#[test]
fn gary_dual_writes_wait_for_eclock_and_drain_before_the_next_transfer() {
    for phase in 0..5 {
        let mut mb = TestMotherboard::new();
        for _ in 0..phase {
            clock_cias(&mut mb.router());
        }
        let wait = [6, 5, 4, 3, 7][phase as usize];
        for _ in 0..wait {
            assert_eq!(
                mb.router().write_word(0xa00c00, 0x8765),
                BusResult::WaitState
            );
            assert!(mb.cia_a.mutations.iter().all(Option::is_none));
            assert!(mb.cia_b.mutations.iter().all(Option::is_none));
            clock_cias(&mut mb.router());
        }
        assert_eq!(
            mb.router().write_word(0xa00c00, 0x8765),
            BusResult::Ready(())
        );
        for (chip, value) in [(&mb.cia_a, 0x65), (&mb.cia_b, 0x87)] {
            assert_eq!(chip.eclock_phase, 1);
            assert_eq!(chip.sdr, 0);
            let pending: Vec<_> = chip.mutations.iter().flatten().collect();
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0].reg_offset, 0x0c);
            assert_eq!(pending[0].value, value);
        }
        clock_cias(&mut mb.router());
        assert_eq!((mb.cia_b.sdr, mb.cia_a.sdr), (0x87, 0x65));
    }

    // Consecutive bus writes drain before the next synchronized acceptance.
    // Each receiver needs at most one of its sixteen slots for CPU traffic.
    let mut mb = TestMotherboard::new();
    for value in 1..=32 {
        let payload = u16::from_be_bytes([value, value + 32]);
        let mut accepted = false;
        for _ in 0..8 {
            if mb.router().write_word(0xbfcc00, payload) == BusResult::Ready(()) {
                accepted = true;
                break;
            }
            assert!(mb.cia_a.mutations.iter().all(Option::is_none));
            assert!(mb.cia_b.mutations.iter().all(Option::is_none));
            clock_cias(&mut mb.router());
        }
        assert!(accepted);
        assert_eq!(mb.cia_a.mutations.iter().flatten().count(), 1);
        assert_eq!(mb.cia_b.mutations.iter().flatten().count(), 1);
        clock_cias(&mut mb.router());
        assert_eq!((mb.cia_b.sdr, mb.cia_a.sdr), (value, value + 32));
        assert!(mb.cia_a.mutations.iter().all(Option::is_none));
        assert!(mb.cia_b.mutations.iter().all(Option::is_none));
    }
}

#[test]
fn gary_inactive_and_unconnected_paths_leave_chips_untouched() {
    let mut mb = TestMotherboard::new();
    mb.cia_a.icr_data = 0x81;
    mb.cia_b.icr_data = 0x92;
    let before_a = mb.cia_a.clone();
    let before_b = mb.cia_b.clone();
    for addr in [0xa03d00, 0xbf3d00, 0xbfcb00, 0xa00bfe] {
        assert_eq!(mb.router().read_word(addr), BusResult::Ready(0xffff));
        assert_eq!(mb.router().write_word(addr, 0x1234), BusResult::Ready(()));
        assert_eq!(mb.router().write_byte(addr | 1, 0x56), BusResult::Ready(()));
    }
    assert_eq!(mb.router().read_word_debug(0xbfcd00), 0x9281);
    assert_eq!(mb.router().read_word_debug(0xa00a00), 0);
    assert_eq!(mb.cia_a, before_a);
    assert_eq!(mb.cia_b, before_b);

    // An already ready first receiver must not acknowledge before the second.
    mb.cia_a.eclock_phase = 1;
    mb.cia_b.eclock_phase = 1;
    mb.cia_a.bus_wait_cck = Some(0);
    for _ in 0..5 {
        assert_eq!(mb.router().read_word(0xbfcd00), BusResult::WaitState);
        assert_eq!((mb.cia_a.icr_data, mb.cia_b.icr_data), (0x81, 0x92));
        assert_eq!(mb.cia_a.bus_wait_cck, Some(0));
        clock_cias(&mut mb.router());
    }
    assert_eq!(mb.router().read_word(0xbfcd00), BusResult::Ready(0x9281));
}

#[test]
fn gary_custom_alias_byte_duplication_and_shared_queue_capacity_checks() {
    let mut mb = TestMotherboard::new();
    for (addr, value) in [(0xde0180, 0x12), (0x01deff81, 0x34)] {
        assert_eq!(mb.router().write_byte(addr, value), BusResult::Ready(()));
        let pending: Vec<_> = mb.denise.mutations.iter().flatten().collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].value, u16::from_be_bytes([value; 2]));
        let _ = mb.denise.step_cck();
        assert_eq!(mb.denise.color[0], u16::from_be_bytes([value; 2]) & 0xfff);
        assert_eq!(mb.router().read_word_debug(addr & !1), 0xffff);
    }
    // An alias must use the shared-register preflight before inserting anywhere.
    for _ in 0..32 {
        assert!(mb.paula.set_serdat(0));
    }
    assert_eq!(
        mb.router().write_word(0xde0096, 0x8211),
        BusResult::WaitState
    );
    assert!(!mb.agnus.dma.cpu_request_pending);
    assert_eq!(mb.agnus.dma.cpu_starvation_counter, 0);
    assert!(mb.agnus.mutations.iter().all(Option::is_none));
    assert_eq!(mb.paula.mutations.iter().flatten().count(), 32);
    let _ = mb.paula.step_cck();
    let _ = mb.paula.step_cck();
    assert_eq!(
        mb.router().write_word(0xde0096, 0x8211),
        BusResult::Ready(())
    );
    assert_eq!(mb.agnus.mutations.iter().flatten().count(), 1);
    assert_eq!(mb.paula.mutations.iter().flatten().count(), 1);
}

#[test]
fn gary_device_decode_preserves_installed_memory_and_rtc_boundaries() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for preset in [
            A500Preset::Bare512k,
            A500Preset::Standard1Mb,
            A500Preset::ExpandedPowerUser,
        ] {
            let mut mb = TestMotherboard::with_config(A500Config::from_preset(preset, standard));
            let fast = preset == A500Preset::ExpandedPowerUser;
            let slow = preset != A500Preset::Bare512k;
            for (start, end, present) in [(0x200000, 0x5ffffe, fast), (0xc00000, 0xc7fffe, slow)] {
                for addr in [start, end] {
                    assert_eq!(mb.router().write_word(addr, 0x1234), BusResult::Ready(()));
                    assert_eq!(
                        mb.router().read_word(addr),
                        BusResult::Ready(if present { 0x1234 } else { 0xffff })
                    );
                }
            }
            mb.mem.chip_ram_blocked = true;
            assert_eq!(
                mb.router().read_word(0xc00000),
                if slow {
                    BusResult::WaitState
                } else {
                    BusResult::Ready(0xffff)
                }
            );
            assert_eq!(
                mb.router().read_word(0x200000),
                BusResult::Ready(if fast { 0x1234 } else { 0xffff })
            );
            mb.mem.chip_ram_blocked = false;
            // RTC register F and both edges retain their own bank and lane rules.
            assert_eq!(mb.router().write_byte(0xdc003d, 0x05), BusResult::Ready(()));
            assert_eq!(
                mb.router().read_byte(0xdc003d),
                BusResult::Ready(if slow { 0x05 } else { 0xff })
            );
            for addr in [
                0x1ffffe, 0x600000, 0x9ffffe, 0xc80000, 0xdbfffe, 0xdc0040, 0xdd0000,
            ] {
                assert_eq!(
                    mb.router().read_word(addr),
                    BusResult::Ready(0xffff),
                    "{addr:06x}"
                );
            }
            // Physical classification supplies no live register side effects.
            mb.cia_a.sdr = 0x5a;
            assert_eq!(mb.mem.read_word(0xa00c00), BusResult::Ready(0xffff));
            assert_eq!(mb.mem.read_word_debug(0xde0002), 0xffff);
            assert_eq!(mb.cia_a.sdr, 0x5a);
        }
    }
}
