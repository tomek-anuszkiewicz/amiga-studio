//! Multi-Instruction Program & Control Flow Integration Tests
//!
//! Tests realistic multi-instruction sequences running against a live Amiga `MemoryBus`
//! with Chip RAM mapped at `$000000-$07FFFF`.
//!
//! Covers:
//! 1. Sequential arithmetic & logic computation
//! 2. Conditional branch loops (`BNE.S`)
//! 3. Decrement & branch loops (`DBF` / `DBRA`)
//! 4. Subroutine calls & returns (`BSR`, `JSR`, `RTS`, stack verification)
//! 5. Stack frames (`LINK` and `UNLK`)
//! 6. Software trap exceptions (`TRAP #0` and `RTE`)
//! 7. Divide-by-zero exception processing (`DIVU` by 0 -> Vector 5)
//! 8. Privilege violation exception processing (User mode `MOVE to SR` -> Vector 8)
//! 9. Address error recovery (Unaligned access -> Vector 3, 7-word stack frame)
//! 10. Multi-register block transfers (`MOVEM.L` save and restore)

use m68000::Cpu;
use memory_bus::MemoryBus;

/// Helper: Initializes CPU and Amiga MemoryBus with Chip RAM mapped at `$000000`.
fn setup_machine() -> (Cpu, MemoryBus) {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    let cpu = Cpu::new();
    (cpu, bus)
}

/// Helper: Injects machine code words into memory starting at `start_addr`.
fn load_program(bus: &mut MemoryBus, start_addr: u32, code: &[u16]) {
    for (i, &word) in code.iter().enumerate() {
        bus.write_word_debug(start_addr + (i as u32) * 2, word);
    }
}

/// Helper: Sets an exception vector (address = `vector_num * 4`) to point to `handler_addr`.
fn set_vector(bus: &mut MemoryBus, vector_num: u32, handler_addr: u32) {
    let vector_addr = vector_num * 4;
    bus.write_word_debug(vector_addr, (handler_addr >> 16) as u16);
    bus.write_word_debug(vector_addr + 2, (handler_addr & 0xFFFF) as u16);
}

/// Helper: Steps the CPU by a specified number of instructions.
fn execute_instructions(cpu: &mut Cpu, bus: &mut MemoryBus, count: usize) {
    for _ in 0..count {
        cpu.step_instruction(bus);
    }
}

// ============================================================================
// Test 1: Sequential Arithmetic & Logic Computation
// ============================================================================

#[test]
fn test_program_arithmetic_computation() {
    // Assembly program:
    // $001000: MOVE.L #15, D0           ; 203C 0000 000F
    // $001006: ADD.L  #27, D0           ; D0BC 0000 001B -> D0 = 42
    // $00100C: LSL.L  #2, D0            ; E588           -> D0 = 168 (0x000000A8)
    // $00100E: SUB.L  #8, D0            ; 90BC 0000 0008 -> D0 = 160 (0x000000A0)
    // $001014: MOVE.L D0, (0x002000).L  ; 23C0 0000 2000 -> store result to memory
    let code: [u16; 13] = [
        0x203C, 0x0000, 0x000F, // MOVE.L #15, D0
        0xD0BC, 0x0000, 0x001B, // ADD.L  #27, D0
        0xE588, // LSL.L  #2, D0
        0x90BC, 0x0000, 0x0008, // SUB.L  #8, D0
        0x23C0, 0x0000, 0x2000, // MOVE.L D0, (0x002000).L
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Step 5 instructions
    for _ in 0..5 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        160,
        "D0 must equal (15 + 27) * 4 - 8 = 160"
    );
    let stored_hi = bus.read_word_debug(0x002000);
    let stored_lo = bus.read_word_debug(0x002002);
    let stored_val = ((stored_hi as u32) << 16) | (stored_lo as u32);
    assert_eq!(
        stored_val, 160,
        "Memory at $002000 must contain stored result 160"
    );
}

// ============================================================================
// Test 2: Conditional Branch Loop (Sum 1..N via BNE.S)
// ============================================================================

#[test]
fn test_program_conditional_branch_loop() {
    // Calculates 5 + 4 + 3 + 2 + 1 = 15
    // Assembly:
    // $001000: CLR.L  D0                ; 4280
    // $001002: MOVEQ  #5, D1            ; 7205
    // loop ($001004):
    // $001004: ADD.L  D1, D0            ; D081
    // $001006: SUBQ.L #1, D1            ; 5381
    // $001008: BNE.S  loop              ; 66FA (offset -6 bytes: from $00100A to $001004)
    let code: [u16; 5] = [
        0x4280, // CLR.L  D0
        0x7205, // MOVEQ  #5, D1
        0xD081, // ADD.L  D1, D0
        0x5381, // SUBQ.L #1, D1
        0x66FA, // BNE.S  $001004
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Total instructions: 2 setup + 5 iterations * 3 (ADD, SUBQ, BNE) = 17 instructions
    for _ in 0..17 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(cpu.state.d_regs()[0], 15, "D0 must equal sum 1..5 = 15");
    assert_eq!(cpu.state.d_regs()[1], 0, "Counter D1 must reach 0");
    assert!(
        (cpu.state.sr & 0x04) != 0,
        "Zero flag (Z) must be set after SUBQ reaches 0"
    );
}

// ============================================================================
// Test 3: Decrement & Branch Loop (`DBF` / `DBRA`)
// ============================================================================

#[test]
fn test_program_dbf_array_sum() {
    // Sums 4 array words at $002000: [10, 20, 30, 40]
    // Assembly:
    // $001000: LEA    (0x002000).L, A0  ; 41F9 0000 2000
    // $001006: CLR.L  D0                ; 4280
    // $001008: MOVEQ  #3, D1            ; 7203 (counter: 4 elements -> count 3 down to -1)
    // loop ($00100A):
    // $00100A: ADD.W  (A0)+, D0         ; D058
    // $00100C: DBF    D1, loop          ; 51C9 FFFC (offset -4: from $00100E to $00100A)
    let code: [u16; 8] = [
        0x41F9, 0x0000, 0x2000, // LEA    (0x002000).L, A0
        0x4280, // CLR.L  D0
        0x7203, // MOVEQ  #3, D1
        0xD058, // ADD.W  (A0)+, D0
        0x51C9, 0xFFFC, // DBF    D1, $00100A
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);

    // Initialize array in Chip RAM
    bus.write_word_debug(0x002000, 10);
    bus.write_word_debug(0x002002, 20);
    bus.write_word_debug(0x002004, 30);
    bus.write_word_debug(0x002006, 40);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 3 setup + 4 * 2 (ADD, DBF) = 11 instructions
    for _ in 0..11 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        100,
        "D0 must equal sum of array = 100"
    );
    assert_eq!(
        cpu.state.d_regs()[1] & 0xFFFF,
        0xFFFF,
        "DBF counter D1 must terminate at -1 ($FFFF)"
    );
    assert_eq!(
        cpu.state.a_regs()[0],
        0x002008,
        "A0 must point past the 4 words"
    );
}

// ============================================================================
// Test 4: Subroutine Call & Return (`BSR`, `JSR`, `RTS`)
// ============================================================================

#[test]
fn test_program_subroutine_call_and_return() {
    // Main calls sub1 via JSR, sub1 computes and returns via RTS.
    // Assembly:
    // $001000: MOVEQ  #10, D0           ; 700A
    // $001002: JSR    (0x001010).L      ; 4EB9 0000 1010
    // $001008: ADDQ.L #5, D0            ; 5080 (D0 = 20 + 5 = 25)
    // $00100A: MOVE.L D0, (0x003000).L  ; 23C0 0000 3000
    // sub1 ($001010):
    // $001010: ADD.L  D0, D0            ; D080 (D0 = 10 * 2 = 20)
    // $001012: RTS                      ; 4E75
    let code: [u16; 8] = [
        0x700A, // $001000: MOVEQ  #10, D0
        0x4EB9, 0x0000, 0x1010, // $001002: JSR    (0x001010).L
        0x5A80, // $001008: ADDQ.L #5, D0
        0x23C0, 0x0000, 0x3000, // $00100A: MOVE.L D0, (0x003000).L
    ];
    let sub_code: [u16; 2] = [
        0xD080, // $001010: ADD.L  D0, D0
        0x4E75, // $001012: RTS
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    load_program(&mut bus, 0x001010, &sub_code);

    // Set initial Stack Pointer (SSP/USP)
    let initial_sp = 0x004000;
    cpu.state.ssp = initial_sp;
    cpu.state.usp = initial_sp;
    cpu.state.write_a(7, initial_sp);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 1: MOVEQ, 2: JSR, 3: ADD.L (in sub1), 4: RTS, 5: ADDQ.L, 6: MOVE.L
    for _ in 0..6 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(cpu.state.d_regs()[0], 25, "D0 must equal 25");
    assert_eq!(
        cpu.state.read_a(7),
        initial_sp,
        "Stack pointer must be balanced after RTS"
    );

    let val_hi = bus.read_word_debug(0x003000);
    let val_lo = bus.read_word_debug(0x003002);
    let result = ((val_hi as u32) << 16) | (val_lo as u32);
    assert_eq!(result, 25, "Stored memory result must be 25");
}

// ============================================================================
// Test 5: Stack Frames (`LINK` and `UNLK`)
// ============================================================================

#[test]
fn test_program_stack_frame_link_unlk() {
    // Tests creating a stack frame, writing to local stack variable, and tearing down frame.
    // Assembly:
    // $001000: LINK   A6, #-8           ; 4E56 FFF8 (pushes A6, A6 = SP, SP -= 8)
    // $001004: MOVE.L #0x12345678, -4(A6); 2D7C 1234 5678 FFFC (stores to local var)
    // $00100C: MOVE.L -4(A6), D0        ; 202E FFFC (reads back local var)
    // $001010: UNLK   A6                ; 4E5E (SP = A6, pops A6)
    let code: [u16; 9] = [
        0x4E56, 0xFFF8, // LINK   A6, #-8
        0x2D7C, 0x1234, 0x5678, 0xFFFC, // MOVE.L #0x12345678, -4(A6)
        0x202E, 0xFFFC, // MOVE.L -4(A6), D0
        0x4E5E, // UNLK   A6
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);

    let initial_sp = 0x004000;
    cpu.state.ssp = initial_sp;
    cpu.state.usp = initial_sp;
    cpu.state.write_a(7, initial_sp);
    cpu.state.write_a(6, 0x00000000); // Initial A6

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    for _ in 0..4 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        0x12345678,
        "D0 must contain value read from stack frame"
    );
    assert_eq!(
        cpu.state.read_a(6),
        0x00000000,
        "A6 must be restored by UNLK"
    );
    assert_eq!(
        cpu.state.read_a(7),
        initial_sp,
        "SP must be restored to initial value by UNLK"
    );
}

// ============================================================================
// Test 6: Software Trap Exception (`TRAP #0` & `RTE`)
// ============================================================================

#[test]
fn test_program_trap_exception_and_rte() {
    // Vector 32 (TRAP #0) points to handler at $002000.
    // User program:
    // $001000: MOVEQ  #7, D0            ; 7007
    // $001002: TRAP   #0                ; 4E40 -> triggers Vector 32
    // $001004: ADDQ.L #3, D0            ; 5680 (D0 = 14 + 3 = 17)
    // Handler ($002000):
    // $002000: ADD.L  D0, D0            ; D080 (D0 = 7 * 2 = 14)
    // $002002: RTE                      ; 4E73 (restores User SR and PC $001004)
    let code: [u16; 3] = [
        0x7007, // MOVEQ  #7, D0
        0x4E40, // TRAP   #0
        0x5680, // ADDQ.L #3, D0
    ];
    let handler: [u16; 2] = [
        0xD080, // ADD.L  D0, D0
        0x4E73, // RTE
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    load_program(&mut bus, 0x002000, &handler);
    set_vector(&mut bus, 32, 0x002000); // Vector 32 = TRAP #0

    let ssp = 0x005000;
    let usp = 0x003000;
    cpu.state.ssp = ssp;
    cpu.state.usp = usp;
    cpu.state.write_a(7, usp);
    cpu.state.sr = 0x0000; // User mode (S=0)

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 1: MOVEQ, 2: TRAP #0, 3: ADD.L (handler), 4: RTE, 5: ADDQ.L (post-trap)
    for _ in 0..5 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(cpu.state.d_regs()[0], 17, "D0 must equal (7 * 2) + 3 = 17");
    assert!(
        !cpu.state.is_supervisor(),
        "CPU must be restored to User mode after RTE"
    );
    assert_eq!(
        cpu.state.read_a(7),
        usp,
        "USP must remain balanced after TRAP and RTE"
    );
    assert_eq!(cpu.state.ssp, ssp, "SSP must be balanced after RTE");
}

// ============================================================================
// Test 7: Divide-by-Zero Exception (`DIVU` by 0 -> Vector 5)
// ============================================================================

#[test]
fn test_program_divide_by_zero_exception() {
    // Vector 5 points to handler at $002500.
    // Code:
    // $001000: MOVE.W #100, D0          ; 303C 0064
    // $001004: MOVE.W #0, D1            ; 323C 0000
    // $001008: DIVU.W D1, D0            ; 80C1 -> Exception Vector 5
    // Handler ($002500):
    // $002500: MOVEQ  #99, D0           ; 7063
    // $002502: RTE                      ; 4E73
    let code: [u16; 5] = [
        0x303C, 0x0064, // MOVE.W #100, D0
        0x323C, 0x0000, // MOVE.W #0, D1
        0x80C1, // DIVU.W D1, D0
    ];
    let handler: [u16; 2] = [
        0x7063, // MOVEQ  #99, D0
        0x4E73, // RTE
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    load_program(&mut bus, 0x002500, &handler);
    set_vector(&mut bus, 5, 0x002500); // Vector 5 = Divide by zero

    let ssp = 0x005000;
    cpu.state.ssp = ssp;
    cpu.state.set_supervisor(true);
    cpu.state.write_a(7, ssp);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 1: MOVE, 2: MOVE, 3: DIVU (traps to $002500), 4: MOVEQ in handler
    for _ in 0..4 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        99,
        "Handler must execute and set D0 to 99"
    );
}

// ============================================================================
// Test 8: Privilege Violation Exception (User Mode `MOVE to SR` -> Vector 8)
// ============================================================================

#[test]
fn test_program_privilege_violation_exception() {
    // Vector 8 points to handler at $002600.
    // User program:
    // $001000: MOVE.W #0x2700, SR       ; 46FC 2700 (Privileged -> traps to Vector 8)
    // Handler at $002600:
    // $002600: MOVEQ  #42, D0           ; 702A
    // $002602: MOVE.W (SP)+, D1         ; 321F (pop stacked SR)
    // $002604: MOVE.L (SP)+, A0         ; 205F (pop stacked return PC)
    // $002606: ADDQ.L #4, A0            ; 5888 (skip the 4-byte MOVE to SR instruction)
    // $002608: JMP    (A0)              ; 4ED0 (jump to $001004)
    // Resumed code ($001004):
    // $001004: ADDQ.L #1, D0            ; 5280 -> D0 = 42 + 1 = 43
    let code: [u16; 3] = [
        0x46FC, 0x2700, // MOVE.W #0x2700, SR
        0x5280, // ADDQ.L #1, D0
    ];
    let handler: [u16; 5] = [
        0x702A, // MOVEQ  #42, D0
        0x321F, // MOVE.W (SP)+, D1
        0x205F, // MOVE.L (SP)+, A0
        0x5888, // ADDQ.L #4, A0
        0x4ED0, // JMP    (A0)
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    load_program(&mut bus, 0x002600, &handler);
    set_vector(&mut bus, 8, 0x002600); // Vector 8 = Privilege Violation

    let ssp = 0x005000;
    let usp = 0x003000;
    cpu.state.ssp = ssp;
    cpu.state.usp = usp;
    cpu.state.write_a(7, usp);
    cpu.state.sr = 0x0000; // User mode (S=0)

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 1: MOVE to SR (traps), 2: MOVEQ, 3: MOVE.W (SP)+, 4: MOVE.L (SP)+, 5: ADDQ.L A0, 6: JMP, 7: ADDQ.L D0
    for _ in 0..7 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        43,
        "D0 must equal 43 after exception fixup and resume"
    );
    assert!(
        cpu.state.is_supervisor(),
        "CPU remains in supervisor mode after handler jumped"
    );
}

// ============================================================================
// Test 9: Address Error Exception Handling & Recovery (Vector 3)
// ============================================================================

#[test]
fn test_program_address_error_recovery() {
    // Vector 3 points to recovery handler at $002800.
    // Unaligned word read generates Vector 3 (Address Error).
    // Code:
    // $001000: LEA    (0x003001).L, A0  ; 41F9 0000 3001 (odd address)
    // $001006: MOVE.W (A0), D0          ; 3010 (unaligned read -> Vector 3)
    // $001008: ADDQ.L #2, D1           ; 5481 (resumed code)
    // Handler at $002800:
    // Discards 7-word exception frame (14 bytes) from SSP and jumps to $001008:
    // $002800: LEA    14(SP), SP        ; 4FEF 000E
    // $002804: MOVEQ  #1, D2            ; 7401 (flag handler ran)
    // $002806: JMP    (0x001008).L      ; 4EF9 0000 1008
    let code: [u16; 5] = [
        0x41F9, 0x0000, 0x3001, // LEA    (0x003001).L, A0
        0x3010, // MOVE.W (A0), D0
        0x5481, // ADDQ.L #2, D1
    ];
    let handler: [u16; 6] = [
        0x4FEF, 0x000E, // LEA    14(SP), SP
        0x7401, // MOVEQ  #1, D2
        0x4EF9, 0x0000, 0x1008, // JMP    (0x001008).L
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    load_program(&mut bus, 0x002800, &handler);
    set_vector(&mut bus, 3, 0x002800); // Vector 3 = Address Error

    let initial_ssp = 0x005000;
    cpu.state.ssp = initial_ssp;
    cpu.state.set_supervisor(true);
    cpu.state.write_a(7, initial_ssp);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 1: LEA, 2: MOVE.W (faults to $002800), 3: LEA 14(SP), 4: MOVEQ, 5: JMP, 6: ADDQ.L
    for _ in 0..6 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(cpu.state.d_regs()[2], 1, "Handler flag D2 must equal 1");
    assert_eq!(
        cpu.state.d_regs()[1],
        2,
        "Resumed instruction must execute and set D1 = 2"
    );
    assert_eq!(
        cpu.state.read_a(7),
        initial_ssp,
        "SSP must be restored to initial value"
    );
}

// ============================================================================
// Test 10: Multi-Register Block Transfer (`MOVEM.L`)
// ============================================================================

#[test]
fn test_program_movem_block_transfer() {
    // Saves D0, D1, D2 to memory buffer at $003000, clears registers, and restores them.
    // Assembly:
    // $001000: MOVE.L #0x11111111, D0   ; 203C 1111 1111
    // $001006: MOVE.L #0x22222222, D1   ; 223C 2222 2222
    // $00100C: MOVE.L #0x33333333, D2   ; 243C 3333 3333
    // $001012: LEA    (0x003000).L, A0  ; 41F9 0000 3000
    // $001018: MOVEM.L D0-D2, (A0)      ; 48D0 0007 (mask = $0007)
    // $00101C: CLR.L  D0                ; 4280
    // $00101E: CLR.L  D1                ; 4281
    // $001020: CLR.L  D2                ; 4282
    // $001022: MOVEM.L (A0), D0-D2      ; 4CD0 0007 (mask = $0007)
    let code: [u16; 19] = [
        0x203C, 0x1111, 0x1111, // MOVE.L #0x11111111, D0
        0x223C, 0x2222, 0x2222, // MOVE.L #0x22222222, D1
        0x243C, 0x3333, 0x3333, // MOVE.L #0x33333333, D2
        0x41F9, 0x0000, 0x3000, // LEA    (0x003000).L, A0
        0x48D0, 0x0007, // MOVEM.L D0-D2, (A0)
        0x4280, // CLR.L  D0
        0x4281, // CLR.L  D1
        0x4282, // CLR.L  D2
        0x4CD0, 0x0007, // MOVEM.L (A0), D0-D2
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    for _ in 0..9 {
        cpu.step_instruction(&mut bus);
    }

    assert_eq!(
        cpu.state.d_regs()[0],
        0x11111111,
        "D0 must be restored to 0x11111111"
    );
    assert_eq!(
        cpu.state.d_regs()[1],
        0x22222222,
        "D1 must be restored to 0x22222222"
    );
    assert_eq!(
        cpu.state.d_regs()[2],
        0x33333333,
        "D2 must be restored to 0x33333333"
    );

    // Verify memory buffer
    let d0_mem =
        ((bus.read_word_debug(0x003000) as u32) << 16) | (bus.read_word_debug(0x003002) as u32);
    let d1_mem =
        ((bus.read_word_debug(0x003004) as u32) << 16) | (bus.read_word_debug(0x003006) as u32);
    let d2_mem =
        ((bus.read_word_debug(0x003008) as u32) << 16) | (bus.read_word_debug(0x00300A) as u32);
    assert_eq!(d0_mem, 0x11111111);
    assert_eq!(d1_mem, 0x22222222);
    assert_eq!(d2_mem, 0x33333333);
}

// ============================================================================
// Test 11: Pre-decrement & Post-increment Addressing (`-(An)` and `(An)+`)
// ============================================================================

#[test]
fn test_program_pre_decrement_post_increment_copy() {
    // Copies 3 longwords from $002000 to $002010 in reverse order using -(A0) and (A1)+
    // Assembly:
    // $001000: LEA    (0x00200C).L, A0  ; 41F9 0000 200C (points past source buffer)
    // $001006: LEA    (0x002010).L, A1  ; 43F9 0000 2010 (destination buffer)
    // $00100C: MOVEQ  #2, D0            ; 7002 (3 elements: count 2, 1, 0)
    // loop ($00100E):
    // $00100E: MOVE.L -(A0), (A1)+      ; 22E0
    // $001010: DBF    D0, loop          ; 51C8 FFFC
    let code: [u16; 10] = [
        0x41F9, 0x0000, 0x200C, // LEA    (0x00200C).L, A0
        0x43F9, 0x0000, 0x2010, // LEA    (0x002010).L, A1
        0x7002, // MOVEQ  #2, D0
        0x22E0, // MOVE.L -(A0), (A1)+
        0x51C8, 0xFFFC, // DBF    D0, $00100E
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);

    // Populate source data: 0xAAAA1111, 0xBBBB2222, 0xCCCC3333
    bus.write_word_debug(0x002000, 0xAAAA);
    bus.write_word_debug(0x002002, 0x1111);
    bus.write_word_debug(0x002004, 0xBBBB);
    bus.write_word_debug(0x002006, 0x2222);
    bus.write_word_debug(0x002008, 0xCCCC);
    bus.write_word_debug(0x00200A, 0x3333);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // 3 setup + 3 * 2 (MOVE.L, DBF) = 9 instructions
    execute_instructions(&mut cpu, &mut bus, 9);

    assert_eq!(
        cpu.state.a_regs()[0],
        0x002000,
        "A0 must point to start of source"
    );
    assert_eq!(
        cpu.state.a_regs()[1],
        0x00201C,
        "A1 must point past destination"
    );

    let dest0 =
        ((bus.read_word_debug(0x002010) as u32) << 16) | (bus.read_word_debug(0x002012) as u32);
    let dest1 =
        ((bus.read_word_debug(0x002014) as u32) << 16) | (bus.read_word_debug(0x002016) as u32);
    let dest2 =
        ((bus.read_word_debug(0x002018) as u32) << 16) | (bus.read_word_debug(0x00201A) as u32);
    assert_eq!(
        dest0, 0xCCCC3333,
        "First copied word must be last source word"
    );
    assert_eq!(
        dest1, 0xBBBB2222,
        "Second copied word must be middle source word"
    );
    assert_eq!(
        dest2, 0xAAAA1111,
        "Third copied word must be first source word"
    );
}

// ============================================================================
// Test 12: Address Register Indirect with Index (`d8(An, Xn)`)
// ============================================================================

#[test]
fn test_program_indexed_addressing_lookup() {
    // Array lookup with base register, displacement, and register index:
    // Reads element at A0 + 4 + D1.W:
    // Table at $002000: [100, 200, 300, 400]
    // A0 = $002000, displacement = 4, D1 = 2 -> Address = $002006 -> reads 400
    // Assembly:
    // $001000: LEA    (0x002000).L, A0  ; 41F9 0000 2000
    // $001006: MOVEQ  #2, D1            ; 7202
    // $001008: MOVE.W 4(A0, D1.W), D0   ; 3030 1004 (ext: D1.W, disp=4)
    let code: [u16; 6] = [
        0x41F9, 0x0000, 0x2000, // LEA    (0x002000).L, A0
        0x7202, // MOVEQ  #2, D1
        0x3030, 0x1004, // MOVE.W 4(A0, D1.W), D0
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);

    bus.write_word_debug(0x002000, 100);
    bus.write_word_debug(0x002002, 200);
    bus.write_word_debug(0x002004, 300);
    bus.write_word_debug(0x002006, 400);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    execute_instructions(&mut cpu, &mut bus, 3);

    assert_eq!(
        cpu.state.d_regs()[0] & 0xFFFF,
        400,
        "D0 must contain indexed value 400"
    );
}

// ============================================================================
// Test 13: Program Counter Relative Addressing (`d16(PC)`)
// ============================================================================

#[test]
fn test_program_pc_relative_addressing() {
    // Position-Independent Code reading constant embedded in code segment:
    // Assembly:
    // $001000: MOVE.W 6(PC), D0         ; 303A 0006 (reads word at $001002 + 6 = $001008)
    // $001004: ADD.W  #10, D0           ; 0640 000A
    // $001008: DC.W   1234              ; constant data
    let code: [u16; 5] = [
        0x303A, 0x0006, // MOVE.W 6(PC), D0
        0x0640, 0x000A, // ADD.W  #10, D0
        1234,   // embedded constant data
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    execute_instructions(&mut cpu, &mut bus, 2);

    assert_eq!(
        cpu.state.d_regs()[0] & 0xFFFF,
        1244,
        "D0 must equal 1234 + 10 = 1244"
    );
}

// ============================================================================
// Test 14: Absolute Short Addressing (`(xxx).W`)
// ============================================================================

#[test]
fn test_program_absolute_short_addressing() {
    // Accesses Chip RAM at $000400 using 16-bit sign-extended absolute short address:
    // Assembly:
    // $001000: MOVE.L #0xCAFEBABE, (0x0400).W ; 21FC CAFE BABE 0400
    // $001008: MOVE.L (0x0400).W, D0          ; 2038 0400
    let code: [u16; 6] = [
        0x21FC, 0xCAFE, 0xBABE, 0x0400, // MOVE.L #0xCAFEBABE, (0x0400).W
        0x2038, 0x0400, // MOVE.L (0x0400).W, D0
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    execute_instructions(&mut cpu, &mut bus, 2);

    assert_eq!(
        cpu.state.d_regs()[0],
        0xCAFEBABE,
        "D0 must equal 0xCAFEBABE"
    );
    let mem_hi = bus.read_word_debug(0x000400);
    let mem_lo = bus.read_word_debug(0x000402);
    let mem_val = ((mem_hi as u32) << 16) | (mem_lo as u32);
    assert_eq!(
        mem_val, 0xCAFEBABE,
        "Memory at $000400 must contain 0xCAFEBABE"
    );
}

// ============================================================================
// Test 15: Program Counter with Index (`d8(PC, Xn)`)
// ============================================================================

#[test]
fn test_program_pc_indexed_addressing_lookup() {
    // Reads from PC-relative jump/data table using index register D1:
    // Base PC is at extension word address ($001004). Displacement = 6 -> Table starts at $00100A.
    // Index D1 = 4 -> Element at $00100E (value 333).
    // Assembly:
    // $001000: MOVEQ  #4, D1            ; 7204
    // $001002: MOVE.W 6(PC, D1.W), D0   ; 303B 1006
    // $001006: NOP                      ; 4E71
    // $001008: NOP                      ; 4E71
    // $00100A: DC.W   111               ; Table element 0
    // $00100C: DC.W   222               ; Table element 1
    // $00100E: DC.W   333               ; Table element 2 (selected)
    let code: [u16; 8] = [
        0x7204, // MOVEQ  #4, D1
        0x303B, 0x1006, // MOVE.W 6(PC, D1.W), D0
        0x4E71, // NOP
        0x4E71, // NOP
        111,    // element 0
        222,    // element 1
        333,    // element 2
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    execute_instructions(&mut cpu, &mut bus, 2);

    assert_eq!(
        cpu.state.d_regs()[0] & 0xFFFF,
        333,
        "D0 must contain PC-indexed table value 333"
    );
}

// ============================================================================
// Test 16: Packed BCD Decimal Arithmetic (`ABCD`)
// ============================================================================

#[test]
fn test_program_bcd_arithmetic() {
    // Performs packed BCD addition: 0x48 + 0x37 = 0x85 (48 + 37 = 85 decimal)
    // Assembly:
    // $001000: MOVE.B #0x48, D0         ; 103C 0048
    // $001004: MOVE.B #0x37, D1         ; 123C 0037
    // $001008: ABCD   D1, D0            ; C101
    let code: [u16; 5] = [
        0x103C, 0x0048, // MOVE.B #0x48, D0
        0x123C, 0x0037, // MOVE.B #0x37, D1
        0xC101, // ABCD   D1, D0
    ];

    let (mut cpu, mut bus) = setup_machine();
    load_program(&mut bus, 0x001000, &code);
    cpu.state.sr &= !0x0015; // Ensure X, Z, C flags are 0
    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    execute_instructions(&mut cpu, &mut bus, 3);

    assert_eq!(
        cpu.state.d_regs()[0] & 0xFF,
        0x85,
        "D0 low byte must equal packed BCD sum 0x85"
    );
}
