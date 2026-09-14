//! End-to-End Machine Loop Interrupt Pipeline Integration Tests
//!
//! Tests complete multi-chip interrupt signaling: from peripheral trigger
//! (Paula Audio, CIA-A, Agnus VBlank), through Paula INTREQ/INTENA and
//! machine arbitration, to M68000 autovector exception processing and RTE.

use config::A500Config;
use machine_loop::A500Machine;

fn setup_test_machine() -> A500Machine {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);
    // Engage Chip RAM in low memory ($000000) for vector table setup
    machine.physical_memory.map_chip_ram_to_low_memory();

    // Setup supervisor stack pointer at top of 512KB Chip RAM
    machine.cpu.state.ssp = 0x070000;
    machine.cpu.state.write_a(7, 0x070000);
    machine.cpu.state.sr = 0x2000; // Supervisor mode, Interrupt Mask = 0
    machine
}

fn set_vector(machine: &mut A500Machine, vector_num: u32, handler_addr: u32) {
    let vector_addr = vector_num * 4;
    machine
        .physical_memory
        .write_word_debug(vector_addr, (handler_addr >> 16) as u16);
    machine
        .physical_memory
        .write_word_debug(vector_addr + 2, (handler_addr & 0xFFFF) as u16);
}

fn load_code(machine: &mut A500Machine, start_addr: u32, words: &[u16]) {
    for (i, &w) in words.iter().enumerate() {
        machine
            .physical_memory
            .write_word_debug(start_addr + (i as u32) * 2, w);
    }
}

#[test]
fn test_end_to_end_audio_interrupt_to_cpu_isr_and_rte() {
    let mut machine = setup_test_machine();

    // 1. Setup Autovector Level 4 (Vector 28, address $000070) -> ISR at $002000
    set_vector(&mut machine, 28, 0x002000);

    // 2. Main code at $001000:
    // $001000: NOP              ; 4E71
    // $001002: MOVEQ #1, D0     ; 7001
    load_code(&mut machine, 0x001000, &[0x4E71, 0x7001]);

    // 3. ISR code at $002000:
    // $002000: MOVEQ #42, D1    ; 722A (Flag that ISR executed)
    // $002002: RTE              ; 4E73 (Return from exception)
    load_code(&mut machine, 0x002000, &[0x722A, 0x4E73]);

    // Prime CPU prefetch at $001000
    machine.set_pc_and_prime_prefetch(0x001000);

    // 4. Enable Paula interrupts: Master enable (bit 14) + Audio Channel 0 (bit 7)
    // Write INTENA ($DFF09A) = 0xC080 (SET bit 15 | INTEN bit 14 | AUD0 bit 7)
    machine.paula.write_intena(0xC080);
    assert_eq!(machine.paula.intena & 0x4080, 0x4080);

    // 5. Trigger Paula Audio Channel 0 buffer finish (AUD0DSR)
    machine.paula.audio.trigger_buffer_finish(0);

    // Step machine Color Clocks until CPU completes NOP and enters the ISR at $002000
    let mut cck_count = 0;
    while machine.cpu.state.instruction_pc != 0x002000 {
        machine.step_cck();
        cck_count += 1;
        assert!(
            cck_count < 200,
            "Timed out waiting for CPU to enter Audio ISR"
        );
    }

    // Assert CPU is now inside the ISR at $002000
    assert_eq!(machine.cpu.state.instruction_pc, 0x002000);
    // Interrupt mask should have been raised to Level 4
    assert_eq!(machine.cpu.state.interrupt_mask(), 4);
    // Supervisor mode active
    assert!(machine.cpu.state.is_supervisor());

    // Clear Paula INTREQ bit 7 inside the handler to prevent infinite loop
    machine.paula.clear_interrupt_request(0x0080);

    // 6. Step through ISR: execute MOVEQ #42, D1
    machine.step_instruction();
    assert_eq!(machine.cpu.state.d_long(1), 42);

    // Execute RTE to return to main thread ($001002)
    machine.step_instruction();
    assert_eq!(machine.cpu.state.instruction_pc, 0x001002);

    // 7. Step through resumed main thread: execute MOVEQ #1, D0
    machine.step_instruction();

    // Verify main thread resumed and committed D0 = 1
    assert_eq!(machine.cpu.state.d_long(0), 1);
    assert_eq!(machine.cpu.state.instruction_pc, 0x001004);
}

#[test]
fn test_end_to_end_cia_a_timer_interrupt_to_cpu() {
    let mut machine = setup_test_machine();

    // Setup Autovector Level 2 (Vector 26, address $000068) -> ISR at $003000
    set_vector(&mut machine, 26, 0x003000);

    // Main code at $001000: loop with NOPs
    load_code(&mut machine, 0x001000, &[0x4E71, 0x4E71, 0x60FC]); // NOP, NOP, BRA.S $001000

    // ISR code at $003000: MOVEQ #88, D2; RTE;
    load_code(&mut machine, 0x003000, &[0x7458, 0x4E73]);

    machine.set_pc_and_prime_prefetch(0x001000);

    // Enable Paula PORTS interrupt (Level 2): Master enable (bit 14) + PORTS (bit 3)
    // Write INTENA ($DFF09A) = 0xC008 (SET bit 15 | INTEN bit 14 | PORTS bit 3)
    machine.paula.write_intena(0xC008);

    // Trigger CIA-A Timer A underflow
    machine.cia_a.write_register(0xD, 0x81); // Enable Timer A IRQ in ICR
    machine.cia_a.write_register(0x4, 1);
    machine.cia_a.write_register(0x5, 0);
    machine.cia_a.write_register(0xE, 0x01); // Start Timer A

    // Step machine until CIA underflows and asserts IRQ
    let mut cck_count = 0;
    while !machine.cia_a.irq_pending() {
        machine.step_cck();
        cck_count += 1;
        assert!(cck_count < 200, "Timed out waiting for CIA-A IRQ");
    }

    // Verify CIA-A asserts IRQ and Paula reflects PORTS bit 3
    assert!(machine.cia_a.irq_pending());
    assert_eq!(machine.paula.intreq & 0x0008, 0x0008);
    assert_eq!(machine.cpu.state.ipl, 2);

    // Step until CPU enters the ISR at $003000
    let mut cck_count = 0;
    while machine.cpu.state.instruction_pc != 0x003000 {
        machine.step_cck();
        cck_count += 1;
        assert!(
            cck_count < 200,
            "Timed out waiting for CPU to enter CIA-A ISR"
        );
    }

    assert_eq!(machine.cpu.state.instruction_pc, 0x003000);
    assert_eq!(machine.cpu.state.interrupt_mask(), 2);

    // Acknowledge CIA interrupt by reading ICR (clears IRQ line)
    machine.cia_a.read_register(0xD);
    machine.paula.clear_interrupt_request(0x0008);

    // Step through ISR: execute MOVEQ #88, D2
    machine.step_instruction();
    assert_eq!(machine.cpu.state.d_long(2), 88);

    // Execute RTE
    machine.step_instruction();
    assert_eq!(machine.cpu.state.interrupt_mask(), 0);
}

#[test]
fn test_end_to_end_vblank_interrupt_to_cpu() {
    let mut machine = setup_test_machine();

    // Setup Autovector Level 3 (Vector 27, address $00006C) -> ISR at $004000
    set_vector(&mut machine, 27, 0x004000);

    // Main code at $001000: loop with NOPs
    load_code(&mut machine, 0x001000, &[0x4E71, 0x4E71, 0x60FC]);

    // ISR code at $004000: MOVEQ #99, D3; RTE;
    load_code(&mut machine, 0x004000, &[0x7663, 0x4E73]);

    machine.set_pc_and_prime_prefetch(0x001000);

    // Enable Paula VERTB interrupt: Master enable (bit 14) + VERTB (bit 5)
    // Write INTENA ($DFF09A) = 0xC020 (SET bit 15 | INTEN bit 14 | VERTB bit 5)
    machine.paula.write_intena(0xC020);

    // Advance Agnus beam to end of frame so it rolls over into line 0, CCK 0
    machine.agnus.vpos = 312;
    machine.agnus.hpos = 226;

    // Step machine until VBlank IRQ is asserted
    let mut cck_count = 0;
    while (machine.paula.intreq & 0x0020) == 0 {
        machine.step_cck();
        cck_count += 1;
        assert!(cck_count < 200, "Timed out waiting for VBlank INTREQ");
    }

    // Verify VBlank asserted
    assert_eq!(machine.paula.intreq & 0x0020, 0x0020);
    assert_eq!(machine.cpu.state.ipl, 3);

    // Step CPU until it enters the ISR at $004000
    let mut cck_count = 0;
    while machine.cpu.state.instruction_pc != 0x004000 {
        machine.step_cck();
        cck_count += 1;
        assert!(
            cck_count < 200,
            "Timed out waiting for CPU to enter VBlank ISR"
        );
    }

    assert_eq!(machine.cpu.state.instruction_pc, 0x004000);
    assert_eq!(machine.cpu.state.interrupt_mask(), 3);

    // Clear VERTB request
    machine.paula.clear_interrupt_request(0x0020);

    // Step through ISR: execute MOVEQ #99, D3
    machine.step_instruction();
    assert_eq!(machine.cpu.state.d_long(3), 99);

    // Execute RTE
    machine.step_instruction();
    assert_eq!(machine.cpu.state.interrupt_mask(), 0);
}

#[test]
fn test_master_intena_masking_suppresses_cpu_interrupt() {
    let mut machine = setup_test_machine();

    set_vector(&mut machine, 28, 0x002000);
    // Main code: infinite loop at $001000 (BRA.S $001000 = 0x60FE)
    load_code(&mut machine, 0x001000, &[0x60FE]);

    machine.set_pc_and_prime_prefetch(0x001000);

    // Disable Master INTEN (bit 14 = 0), but set AUD0 channel enable (bit 7)
    // Write INTENA ($DFF09A) = 0x8080 (SET bit 15 | AUD0 bit 7, bit 14 INTEN is 0)
    machine.paula.write_intena(0x8080);

    // Assert Paula Audio Channel 0 request
    machine.paula.audio.trigger_buffer_finish(0);

    // Step machine
    for _ in 0..20 {
        machine.step_cck();
    }

    // INTREQ bit 7 is set, but because INTEN is 0, IPL remains 0
    assert_eq!(machine.paula.intreq & 0x0080, 0x0080);
    assert_eq!(machine.cpu.state.ipl, 0);
    // CPU continues normal execution without entering ISR
    assert_ne!(machine.cpu.state.instruction_pc, 0x002000);
    assert_eq!(machine.cpu.state.instruction_pc, 0x001000);
}
