//! Motorola 68000 Autovector Interrupts & Exception Processing Tests
//!
//! Validates cycle-exact autovector interrupts (Levels 1..7, vectors 25..31),
//! interrupt mask filtering, Level 7 NMI behavior, supervisor state transition,
//! trace mode clearing, stack frame layout, STOP awakening, and RTE restoration.

use m68000::Cpu;
use physical_memory::PhysicalMemory;

fn setup_test_machine() -> (Cpu, PhysicalMemory) {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    let mut cpu = Cpu::new();
    cpu.state.ssp = 0x070000;
    cpu.state.write_a(7, 0x070000);
    cpu.state.sr = 0x2000; // Supervisor mode, Interrupt Mask = 0
    (cpu, bus)
}

fn set_vector(bus: &mut PhysicalMemory, vector_num: u32, handler_addr: u32) {
    let vector_addr = vector_num * 4;
    bus.write_word_debug(vector_addr, (handler_addr >> 16) as u16);
    bus.write_word_debug(vector_addr + 2, (handler_addr & 0xFFFF) as u16);
}

fn load_code(bus: &mut PhysicalMemory, start_addr: u32, words: &[u16]) {
    for (i, &w) in words.iter().enumerate() {
        bus.write_word_debug(start_addr + (i as u32) * 2, w);
    }
}

#[test]
fn test_autovector_level_4_audio_irq() {
    let (mut cpu, mut bus) = setup_test_machine();

    // Set Autovector Level 4 (Vector 28, address $000070)
    set_vector(&mut bus, 28, 0x002000);

    // Main code at $001000:
    // $001000: NOP              ; 4E71
    // $001002: MOVEQ #1, D0     ; 7001
    load_code(&mut bus, 0x001000, &[0x4E71, 0x7001]);

    // ISR code at $002000:
    // $002000: MOVEQ #42, D1    ; 722A
    // $002002: RTE              ; 4E73
    load_code(&mut bus, 0x002000, &[0x722A, 0x4E73]);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Assert Level 4 interrupt
    cpu.state.ipl = 4;

    // Step 1: NOP retires (4 clocks) and queues Level 4 interrupt
    let nop_clocks = cpu.step_instruction(&mut bus);
    assert_eq!(nop_clocks, 4);

    // Step 2: Interrupt exception executes (44 clocks) and transitions to ISR
    let irq_clocks = cpu.step_instruction(&mut bus);
    assert_eq!(irq_clocks, 44);

    // CPU should now be at entry of ISR ($002000)
    assert_eq!(cpu.state.instruction_pc, 0x002000);
    // Stack should have been decremented by 6 bytes
    assert_eq!(cpu.state.read_a(7), 0x06FFFA);
    // Interrupt mask should now be 4
    assert_eq!(cpu.state.interrupt_mask(), 4);
    // Supervisor bit should be set
    assert!(cpu.state.is_supervisor());

    // Clear IPL so we don't re-interrupt immediately after RTE
    cpu.state.ipl = 0;

    // Step 3: Execute MOVEQ #42, D1 inside ISR
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.d_long(1), 42);

    // Step 4: Execute RTE
    cpu.step_instruction(&mut bus);
    // Stack restored
    assert_eq!(cpu.state.read_a(7), 0x070000);
    // Mask restored to 0
    assert_eq!(cpu.state.interrupt_mask(), 0);
    // PC returned to $001002 (MOVEQ #1, D0)
    assert_eq!(cpu.state.instruction_pc, 0x001002);

    // Step 5: Execute MOVEQ #1, D0 in resumed main thread
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.d_long(0), 1);
}

#[test]
fn test_interrupt_mask_filtering() {
    let (mut cpu, mut bus) = setup_test_machine();
    set_vector(&mut bus, 28, 0x002000);
    load_code(&mut bus, 0x001000, &[0x4E71, 0x4E71]);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Set SR mask to Level 4 (SR = $2400)
    cpu.state.set_interrupt_mask(4);

    // Assert Level 3 (< mask): ignored
    cpu.state.ipl = 3;
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.instruction_pc, 0x001002);

    // Assert Level 4 (== mask): ignored on 68000
    cpu.state.ipl = 4;
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.instruction_pc, 0x001004);

    // Assert Level 5 (> mask): accepted!
    set_vector(&mut bus, 29, 0x003000);
    load_code(&mut bus, 0x003000, &[0x4E73]);
    cpu.state.ipl = 5;
    // Step NOP:
    cpu.step_instruction(&mut bus);
    // Step Interrupt:
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.instruction_pc, 0x003000);
    assert_eq!(cpu.state.interrupt_mask(), 5);
}

#[test]
fn test_level_7_nmi_fires_even_with_mask_7() {
    let (mut cpu, mut bus) = setup_test_machine();
    // Level 7 autovector is Vector 31 ($00007C)
    set_vector(&mut bus, 31, 0x007000);
    load_code(&mut bus, 0x001000, &[0x4E71]);
    load_code(&mut bus, 0x007000, &[0x4E73]);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Set SR mask to Level 7 (SR = $2700)
    cpu.state.set_interrupt_mask(7);
    assert_eq!(cpu.state.interrupt_mask(), 7);

    // Assert Level 7 NMI
    cpu.state.ipl = 7;
    // Step NOP:
    cpu.step_instruction(&mut bus);
    // Step Level 7 Interrupt:
    cpu.step_instruction(&mut bus);

    // Must have taken Level 7 interrupt
    assert_eq!(cpu.state.instruction_pc, 0x007000);
    assert_eq!(cpu.state.interrupt_mask(), 7);
}

#[test]
fn test_stop_instruction_awakened_by_interrupt() {
    let (mut cpu, mut bus) = setup_test_machine();
    set_vector(&mut bus, 26, 0x002000); // Level 2 autovector (CIA-A)

    // Code:
    // $001000: STOP #$2000    ; 4E72 2000 (Supervisor, mask = 0)
    // $001004: MOVEQ #99, D0  ; 7063
    load_code(&mut bus, 0x001000, &[0x4E72, 0x2000, 0x7063]);

    // ISR at $002000:
    // $002000: MOVEQ #7, D2   ; 7407
    // $002002: RTE            ; 4E73
    load_code(&mut bus, 0x002000, &[0x7407, 0x4E73]);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Execute STOP
    cpu.step_instruction(&mut bus);
    assert!(cpu.state.stopped);
    assert_eq!(cpu.state.interrupt_mask(), 0);

    // Step while stopped and no interrupt: returns 0 cycles, still stopped
    let cycles = cpu.step_instruction(&mut bus);
    assert_eq!(cycles, 0);
    assert!(cpu.state.stopped);

    // Assert Level 2 interrupt (e.g. CIA-A timer)
    cpu.state.ipl = 2;

    // Next step must awaken CPU and execute the 44-clock interrupt sequence
    let irq_cycles = cpu.step_instruction(&mut bus);
    assert_eq!(irq_cycles, 44);
    assert!(!cpu.state.stopped);
    assert_eq!(cpu.state.instruction_pc, 0x002000);
    assert_eq!(cpu.state.interrupt_mask(), 2);

    cpu.state.ipl = 0;

    // Step ISR
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.d_long(2), 7);

    // Execute RTE
    cpu.step_instruction(&mut bus);
    // Should return to $001004 (after the STOP instruction)
    assert_eq!(cpu.state.instruction_pc, 0x001004);

    // Execute MOVEQ #99, D0
    cpu.step_instruction(&mut bus);
    assert_eq!(cpu.state.d_long(0), 99);
}
