use config::{A500Config, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_dmacon_routing_to_all_subsystems() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Initially all DMA channels are disabled
    assert!(!machine.copper.dma_enabled);
    assert!(!machine.blitter.dma_enabled);
    assert!(!machine.blitter.bltpri);
    assert!(!machine.sprites.dma_enabled);
    assert!(!machine.floppy.dma_enabled);
    assert!(!machine.frame_builder.dma_enabled);
    for ch in 0..4 {
        assert!(!machine.audio.channels[ch].dma_enabled);
    }

    // Write DMACON ($DFF096) = 0x83FF:
    // SET (bit 15) | DMAEN (bit 9) | BPU (bit 8) | COPEN (bit 7) | BLTEN (bit 6) |
    // SPREN (bit 5) | DSKEN (bit 4) | AUD3..0EN (bits 3..0)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF096, 0x83FF),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles for DMACON write to mature in Agnus and Paula
    machine.step_cck();
    machine.step_cck();

    // Verify all subsystems received their DMA enables
    assert!(machine.copper.dma_enabled);
    assert!(machine.blitter.dma_enabled);
    assert!(!machine.blitter.bltpri);
    assert!(machine.sprites.dma_enabled);
    assert!(machine.floppy.dma_enabled);
    assert!(machine.frame_builder.dma_enabled);
    for ch in 0..4 {
        assert!(machine.audio.channels[ch].dma_enabled);
    }

    // Write DMACON = 0x8400 (SET BLTPRI)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF096, 0x8400),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();
    assert!(machine.blitter.bltpri);

    // Write DMACON = 0x0200 (CLR DMAEN master enable)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF096, 0x0200),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Master DMAEN is now 0; all subsystems must be disabled
    assert!(!machine.copper.dma_enabled);
    assert!(!machine.blitter.dma_enabled);
    assert!(!machine.sprites.dma_enabled);
    assert!(!machine.floppy.dma_enabled);
    assert!(!machine.frame_builder.dma_enabled);
    for ch in 0..4 {
        assert!(!machine.audio.channels[ch].dma_enabled);
    }
}

#[test]
fn test_copper_strobe_jumps_and_pointer_sync() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Program COP1LC via bus: COP1LCH ($080) = 0x0004, COP1LCL ($082) = 0x1000
    assert_eq!(
        machine.memory_bus().write_word(0xDFF080, 0x0004),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().write_word(0xDFF082, 0x1000),
        BusResult::Ready(())
    );

    // Step 2 CCK cycles so Agnus matures the writes
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.cop1lc, 0x00041000);
    assert_eq!(machine.copper.cop1lc, 0x00041000);

    // Strobe COPJMP1 ($088)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF088, 0x0000),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Copper PC should have reloaded from COP1LC
    assert_eq!(machine.copper.cop_pc, 0x00041000);

    // Program COP2LC via bus: COP2LCH ($084) = 0x0005, COP2LCL ($086) = 0x2000
    assert_eq!(
        machine.memory_bus().write_word(0xDFF084, 0x0005),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().write_word(0xDFF086, 0x2000),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.cop2lc, 0x00052000);
    assert_eq!(machine.copper.cop2lc, 0x00052000);

    // Strobe COPJMP2 ($08A)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF08A, 0x0000),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Copper PC should have reloaded from COP2LC
    assert_eq!(machine.copper.cop_pc, 0x00052000);
}

#[test]
fn test_blitter_size_triggers_busy_and_syncs_pointers() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Program Blitter pointers in Agnus
    assert_eq!(
        machine.memory_bus().write_word(0xDFF050, 0x0001),
        BusResult::Ready(())
    ); // BLTAPTH
    assert_eq!(
        machine.memory_bus().write_word(0xDFF052, 0x2344),
        BusResult::Ready(())
    ); // BLTAPTL
    assert_eq!(
        machine.memory_bus().write_word(0xDFF040, 0x09F0),
        BusResult::Ready(())
    ); // BLTCON0
    machine.step_cck();
    machine.step_cck();

    assert_eq!(machine.agnus.bltapt, 0x00012344);
    assert_eq!(machine.agnus.bltcon0, 0x09F0);

    // Initially blitter is idle
    assert!(!machine.blitter.is_busy);

    // Write BLTSIZE ($058) = 0x0404 (4 lines of 4 words)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF058, 0x0404),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Blitter should have synced pointers and triggered busy
    assert!(machine.blitter.is_busy);
    assert_eq!(machine.blitter.bltapt, 0x00012344);
    assert_eq!(machine.blitter.bltcon0, 0x09F0);
    assert_eq!(machine.blitter.bltsize, 0x0404);
}

#[test]
fn test_end_to_end_floppy_bus_control_and_sensor_readback() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));

    // Insert formatted disk into DF0
    machine.floppy.drives[0].insert_disk(&[0; 100]);

    // Initially, DF0 head is at cylinder 0, disk inserted
    assert_eq!(machine.floppy.drives[0].cylinder, 0);
    assert!(machine.floppy.drives[0].disk_inserted);

    // Step 1: Select DF0, Motor ON, Direction OUT (towards higher cylinders), STEP inactive (high)
    // CIA-B Port B: bit 7 = _MTR (0=on), bit 6 = _SEL3 (1), bit 5 = _SEL2 (1), bit 4 = _SEL1 (1),
    //               bit 3 = _SEL0 (0=active), bit 2 = _SIDE (1=lower), bit 1 = _DIR (0=towards higher), bit 0 = _STEP (1=inactive)
    // Value: 0b0111_0101 = 0x75
    assert_eq!(
        machine.memory_bus().write_byte(0xBFD100, 0x75),
        BusResult::Ready(())
    );
    machine.step_cck();

    assert!(machine.floppy.drives[0].motor_on);

    // Step 2: Pulse _STEP low (bit 0 = 0 -> 0x74)
    assert_eq!(
        machine.memory_bus().write_byte(0xBFD100, 0x74),
        BusResult::Ready(())
    );
    machine.step_cck();

    // Step 3: Return _STEP high (bit 0 = 1 -> 0x75)
    assert_eq!(
        machine.memory_bus().write_byte(0xBFD100, 0x75),
        BusResult::Ready(())
    );
    machine.step_cck();

    // Head stepped from cylinder 0 to cylinder 1!
    assert_eq!(machine.floppy.drives[0].cylinder, 1);

    // Step CCK updates peripheral pins into CIA-A
    machine.step_cck();

    // Read CIA-A Port A ($BFE001):
    // Bit 4 is _TK0 (Track 0 sensor, active low). Since head is at cylinder 1, _TK0 should be 1 (inactive).
    let val = match machine.memory_bus().read_byte(0xBFE001) {
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
        machine.memory_bus().write_word(0xDFF096, 0x8210),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();
    assert!(machine.floppy.dma_enabled);

    // Test DSKLEN 2-write arming sequence
    // First write: DSKLEN ($DFF024) = 0x9000 (SET bit 15, len = 0x1000)
    assert_eq!(
        machine.memory_bus().write_word(0xDFF024, 0x9000),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // First write does not arm DMA yet
    assert!(!machine.floppy.is_dma_active());

    // Second write: DSKLEN = 0x9000
    assert_eq!(
        machine.memory_bus().write_word(0xDFF024, 0x9000),
        BusResult::Ready(())
    );
    machine.step_cck();
    machine.step_cck();

    // Second consecutive write with bit 15 sets dma_active
    assert!(machine.floppy.is_dma_active());
}
