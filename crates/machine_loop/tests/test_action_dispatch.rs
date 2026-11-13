#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use config::custom_reg;
use config::{A500Config, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_dmacon_routing_to_all_subsystems() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Initially all DMA channels are disabled
    assert!(!machine.agnus.copper.dma_enabled);
    assert!(!machine.agnus.blitter.dma_enabled);
    assert!(!machine.agnus.blitter.bltpri);
    assert!(!machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_SPREN));
    assert_eq!(
        machine.paula.dma_enables & config::mask::dmacon::BIT_DSKEN,
        0
    );
    assert!(!machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_BPLEN));
    for ch in 0..4 {
        assert!(!machine.paula.audio.channels[ch].dma_enabled);
    }

    // Write DMACON ($DFF096) = 0x83FF:
    // SET (bit 15) | DMAEN (bit 9) | BPU (bit 8) | COPEN (bit 7) | BLTEN (bit 6) |
    // SPREN (bit 5) | DSKEN (bit 4) | AUD3..0EN (bits 3..0)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACON),
            0x83FF
        ),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles for DMACON write to mature in Agnus and Paula
    machine.step_cck();
    machine.step_cck();

    // Verify all subsystems received their DMA enables
    assert!(machine.agnus.copper.dma_enabled);
    assert!(machine.agnus.blitter.dma_enabled);
    assert!(!machine.agnus.blitter.bltpri);
    assert!(machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_SPREN));
    assert_ne!(
        machine.paula.dma_enables & config::mask::dmacon::BIT_DSKEN,
        0
    );
    assert!(machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_BPLEN));
    for ch in 0..4 {
        assert!(machine.paula.audio.channels[ch].dma_enabled);
    }

    // Write DMACON = 0x8400 (SET BLTPRI)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACON),
            0x8400
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();
    assert!(machine.agnus.blitter.bltpri);

    // Write DMACON = 0x0200 (CLR DMAEN master enable)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_DMACON),
            0x0200
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Master DMAEN is now 0; all subsystems must be disabled
    assert!(!machine.agnus.copper.dma_enabled);
    assert!(!machine.agnus.blitter.dma_enabled);
    assert!(!machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_SPREN));
    assert!(!machine.paula.dma_master);
    assert!(!machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_BPLEN));
    for ch in 0..4 {
        assert!(!machine.paula.audio.channels[ch].dma_enabled);
    }
}

#[test]
fn test_copper_strobe_jumps_and_pointer_sync() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Program COP1LC via bus: COP1LCH ($080) = 0x0004, COP1LCL ($082) = 0x1000
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COP1LCH),
            0x0004
        ),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COP1LCL),
            0x1000
        ),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles so Agnus matures the writes
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.copper.cop1lc, 0x00041000);

    // Strobe COPJMP1 ($088)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COPJMP1),
            0x0000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Copper PC should have reloaded from COP1LC
    assert_eq!(machine.agnus.copper.cop_pc, 0x00041000);

    // Program COP2LC via bus: COP2LCH ($084) = 0x0005, COP2LCL ($086) = 0x2000
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COP2LCH),
            0x0005
        ),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COP2LCL),
            0x2000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.copper.cop2lc, 0x00052000);

    // Strobe COPJMP2 ($08A)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_COPJMP2),
            0x0000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Copper PC should have reloaded from COP2LC
    assert_eq!(machine.agnus.copper.cop_pc, 0x00052000);
}

#[test]
fn test_blitter_size_triggers_busy_and_syncs_pointers() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Program Blitter pointers in Agnus
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_BLTAPTH),
            0x0001
        ),
        BusResult::Ready(())
    ); // BLTAPTH
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_BLTAPTL),
            0x2344
        ),
        BusResult::Ready(())
    ); // BLTAPTL
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_BLTCON0),
            0x09F0
        ),
        BusResult::Ready(())
    ); // BLTCON0
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.blitter.bltapt, 0x00012344);
    assert_eq!(machine.agnus.blitter.bltcon0, 0x09F0);

    // Initially blitter is idle
    assert!(!machine.agnus.blitter.is_busy);

    // Write BLTSIZE ($058) = 0x0404 (4 lines of 4 words)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_BLTSIZE),
            0x0404
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Blitter should have synced pointers and triggered busy
    assert!(machine.agnus.blitter.is_busy);
    assert_eq!(machine.agnus.blitter.bltapt, 0x00012344);
    assert_eq!(machine.agnus.blitter.bltcon0, 0x09F0);
    assert_eq!(machine.agnus.blitter.bltsize, 0x0404);
}

#[test]
fn test_end_to_end_floppy_bus_control_and_sensor_readback() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    common::cia::configure_drive_control_outputs(&mut machine);

    // Insert formatted disk into DF0
    machine.floppy.drives[0].insert_disk(&[0; 100]);

    // Initially, DF0 head is at cylinder 0, disk inserted
    assert_eq!(machine.floppy.drives[0].cylinder, 0);
    assert!(machine.floppy.drives[0].disk_inserted);

    // HRM Table 8-5: establish MOTOR before the selection edge latches it.
    assert_eq!(
        common::cia::write_byte(&mut machine, 0xBFD100, 0x7f),
        BusResult::Ready(())
    );
    machine.step_cck();

    // Step 1: Select DF0, Motor ON, Direction OUT (towards higher cylinders), STEP inactive (high)
    // CIA-B Port B: bit 7 = _MTR (0=on), bit 6 = _SEL3 (1), bit 5 = _SEL2 (1), bit 4 = _SEL1 (1),
    //               bit 3 = _SEL0 (0=active), bit 2 = _SIDE (1=lower), bit 1 = _DIR (0=towards higher), bit 0 = _STEP (1=inactive)
    // Value: 0b0111_0101 = 0x75
    assert_eq!(
        common::cia::write_byte(&mut machine, 0xBFD100, 0x75),
        BusResult::Ready(())
    );
    machine.step_cck();

    assert!(machine.floppy.drives[0].motor_on);

    // Step 2: Pulse _STEP low (bit 0 = 0 -> 0x74)
    assert_eq!(
        common::cia::write_byte(&mut machine, 0xBFD100, 0x74),
        BusResult::Ready(())
    );
    machine.step_cck();

    // Step 3: Return _STEP high (bit 0 = 1 -> 0x75)
    assert_eq!(
        common::cia::write_byte(&mut machine, 0xBFD100, 0x75),
        BusResult::Ready(())
    );
    machine.step_cck();

    // Head stepped from cylinder 0 to cylinder 1!
    assert_eq!(machine.floppy.drives[0].cylinder, 1);

    // Step CCK updates peripheral pins into CIA-A
    machine.step_cck();

    // Read CIA-A Port A ($BFE001):
    // Bit 4 is _TK0 (Track 0 sensor, active low). Since head is at cylinder 1, _TK0 should be 1 (inactive).
    let val = match common::cia::read_byte(&mut machine, 0xBFE001) {
        BusResult::Ready(v) => v,
        _ => panic!("Expected ready read"),
    };
    assert_eq!(
        val & (1 << 4),
        1 << 4,
        "TK0 must be inactive (high) at cylinder 1"
    );

    // Enable Floppy DMA via DMACON ($DFF096) = 0x8210 (SET bit 15, DMAEN bit 9, DSKEN bit 4)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_DMACON),
            0x8210
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();
    assert_ne!(
        machine.paula.dma_enables & config::mask::dmacon::BIT_DSKEN,
        0
    );

    // Test DSKLEN 2-write arming sequence
    // First write: DSKLEN ($DFF024) = 0x9000 (SET bit 15, len = 0x1000)
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_DSKLEN),
            0x9000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // First write does not activate DMA yet (armed only)
    assert!(machine.paula.dma_armed);
    assert!(!machine.paula.dma_active);

    // Second write: DSKLEN = 0x9000
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_DSKLEN),
            0x9000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Second consecutive write with bit 15 sets dma_active
    assert!(machine.paula.dma_active);
}

#[test]
fn test_blitter_finish_asserts_blitint_and_escalates_ipl() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Enable master INTEN (bit 14) and Level 3 BLIT interrupt (bit 6): 0xC040
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_INTENA),
            0xC040
        ),
        BusResult::Ready(())
    );
    machine.step_cck(); // Mature INTENA write

    assert_eq!(machine.cpu.state.ipl(), 0);
    assert!(!machine.agnus.blitter.is_busy);

    // Trigger a blit
    machine.agnus.blitter.trigger_blit(0x0408);
    assert!(machine.agnus.blitter.is_busy);

    // Blitter completes execution
    machine.agnus.blitter.finish_blit();
    assert!(!machine.agnus.blitter.is_busy);

    // TRM Fat Agnus: INT3* reports completion to Paula. vAmiga SlowBlitter
    // schedules BLIT after one DMA cycle; our capture boundary is a model phase,
    // not a measured electrical delay. Capture must not mutate the recipient.
    machine.step_cck();
    assert!(!machine.agnus.pins.int3_high);
    assert!(machine.paula.pins.int3_high);
    assert_eq!(machine.paula.interrupts.intreq & 0x0040, 0);
    assert_eq!(machine.cpu.state.ipl(), 0);
    machine.step_cck();
    assert!(!machine.paula.pins.int3_high);
    assert_eq!(machine.paula.interrupts.intreq & 0x0040, 0x0040);
    assert_eq!(machine.cpu.state.ipl(), 3);
}

#[test]
fn test_audio_restart_reloads_audpt_and_asserts_level4_ipl() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    // Keep register-delivery assertions outside reserved refresh slots.
    machine.agnus.hpos = 0x3f;
    machine.agnus.vpos = 100; // STRHOR anchors the request frame outside blanking.

    // Enable master INTEN (bit 14) and Audio Channel 0 interrupt (bit 7): 0xC080
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::paula::REGISTER_INTENA),
            0xC080
        ),
        BusResult::Ready(())
    );
    machine.step_cck(); // Mature INTENA write

    // Set Audio Channel 0 loop address via AUD0LCH/LCL: $00025000
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_AUD0LCH),
            0x0002
        ),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().write_word(
            0xDFF000 + u32::from(custom_reg::agnus::REGISTER_AUD0LCL),
            0x5000
        ),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck(); // Mature in Agnus

    assert_eq!(machine.agnus.audlc[0], 0x0002_5000);
    assert_eq!(machine.agnus.audpt[0], 0x0002_5000);

    // Simulate DMA playback advancing the pointer past the buffer
    machine.agnus.audpt[0] = 0x0002_5100;

    // A request must traverse DMAL; it cannot reload a pointer in the coordinator.
    machine.cpu.state.set_stopped(true);
    machine.physical_memory.map_chip_ram_to_low_memory();
    machine
        .physical_memory
        .write_word_debug(0x0002_5000, 0x1234);
    machine
        .physical_memory
        .write_word_debug(0x0002_5100, 0x5678);
    machine.paula.audio.set_len(0, 1);
    machine.paula.audio.set_per(0, 200);
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8201));
    machine.step_cycles(3);
    assert_eq!(machine.agnus.audpt[0], 0x0002_5100);
    for _ in 0..600 {
        machine.step_cck();
        if machine.paula.audio.channels[0].dat == 0x1234 {
            break;
        }
    }
    assert_eq!(machine.paula.audio.channels[0].dat, 0x1234);
    assert_eq!(machine.agnus.audpt[0], 0x0002_5002);
    assert_eq!(machine.paula.interrupts.intreq & 0x0080, 0x0080);
    assert_eq!(machine.cpu.state.ipl(), 4);
}

#[test]
fn test_floppy_ciab_prb_polling_and_dskbytr_paula_latching() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    common::cia::configure_drive_control_outputs(&mut machine);

    // HRM Table 8-5 requires motor data before drive selection. Hold MTR low
    // with every SEL released, then make a separate write to select DF0.
    assert_eq!(
        common::cia::write_byte(&mut machine, 0xBFD100, 0x7f),
        BusResult::Ready(())
    );
    machine.step_cck();
    assert_eq!(machine.floppy.pins.control_input, 0x7f);
    assert!(
        !machine.floppy.drives[0].motor_on,
        "no select edge has latched the motor"
    );

    // CPU writes to CIA-B PRB ($BFD100) selecting DF0 with motor on: 0x75
    assert_eq!(
        common::cia::accept_write(&mut machine, 0xBFD100, 0x75),
        BusResult::Ready(())
    );

    // The accepted write is pending; pins and drive control have not changed.
    assert!(!machine.cia_b.prb_mutated);
    assert!(!machine.floppy.drives[0].motor_on);
    assert_eq!(machine.cia_b.mutations.len(), 1);

    // CIA capture changes the source latch first. The retained connector frame
    // is delivered at our next input phase, not during that register commitment.
    machine.step_cck();
    assert!(!machine.cia_b.prb_mutated);
    assert_eq!(machine.floppy.pins.control_input, 0x7f);
    assert!(!machine.floppy.drives[0].motor_on);
    machine.step_cck();
    assert_eq!(machine.floppy.pins.control_input, 0x75);
    assert!(machine.floppy.drives[0].motor_on);

    // Deterministic raw input, with repeated sync aligning the byte stream.
    assert!(machine.paula.set_adkcon(0x8500));
    assert!(machine.paula.set_dsksync(0x5a42));
    machine.step_cycles(2);
    for _ in 0..2 {
        for bit in (0..16).rev() {
            machine.floppy.pins.read_data_high = 0x5a42 & (1 << bit) == 0;
            machine.step_cck();
            machine.floppy.pins.read_data_high = true;
            machine.step_cycles(6);
        }
    }
    // Byte availability belongs to Paula; no DMA was armed in this fixture.
    assert_eq!(machine.paula.disk.buffered_words(), 0);
    assert_eq!(machine.paula.dskbytr & 0x90FF, 0x8000 | 0x1000 | 0x42);

    // CPU reads DSKBYTR from Paula via memory bus
    let val = match machine
        .memory_bus()
        .read_word(0xDFF000 + u32::from(custom_reg::paula::REGISTER_DSKBYTR))
    {
        BusResult::Ready(v) => v,
        _ => panic!("Expected Ready"),
    };
    assert_eq!(val & 0x90FF, 0x8000 | 0x1000 | 0x42);

    // Paula bit 15 is cleared on read
    assert_eq!(machine.paula.dskbytr & 0x8000, 0);
}

#[test]
fn test_game_ports_fire_buttons_ciaa_pra_polling() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Default: both fire pins float high (1)
    machine.step_cycles(2);
    assert_eq!(
        machine.cia_a.pra() & game_ports::MASK_CIAA_PRA_FIRE,
        game_ports::MASK_CIAA_PRA_FIRE
    );

    // Press mouse left button (Port 1 /FIR0)
    machine.game_ports.set_mouse_buttons(true, false, false);
    machine.step_cycles(2);
    assert_eq!(
        machine.cia_a.pra() & game_ports::MASK_CIAA_PRA_FIRE,
        game_ports::MASK_CIAA_PRA_FIRE & !game_ports::BIT_CIAA_PRA_FIR0
    );

    // Press joystick fire (Port 2 /FIR1)
    machine
        .game_ports
        .set_joystick(false, false, false, false, true, false);
    machine.step_cycles(2);
    assert_eq!(machine.cia_a.pra() & game_ports::MASK_CIAA_PRA_FIRE, 0);
}
