use debugger::DebuggerSession;
use std::fs;
use std::path::Path;

#[test]
fn test_all_sample_binaries_execution_and_generation() {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("bin");
    fs::create_dir_all(&out_dir).expect("Failed to create tests/bin dir");

    // =========================================================================
    // 1. Fibonacci Sequence ($001000)
    // =========================================================================
    #[rustfmt::skip]
    let fib_words: Vec<u16> = vec![
        0x41F9, 0x0000, 0x2000, // LEA     $002000, A0
        0x4240,                 // CLR.W   D0
        0x323C, 0x0001,         // MOVE.W  #1, D1
        0x30C0,                 // MOVE.W  D0, (A0)+
        0x30C1,                 // MOVE.W  D1, (A0)+
        0x363C, 0x000D,         // MOVE.W  #13, D3
        // fib_loop ($1014):
        0x3400,                 // MOVE.W  D0, D2
        0xD441,                 // ADD.W   D1, D2
        0x30C2,                 // MOVE.W  D2, (A0)+
        0x3001,                 // MOVE.W  D1, D0
        0x3202,                 // MOVE.W  D2, D1
        0x51CB, 0xFFF4,         // DBRA    D3, fib_loop
        // halt ($1022):
        0x60FE,                 // BRA.S   halt
    ];

    let fib_bytes = words_to_bytes(&fib_words);
    fs::write(out_dir.join("fibonacci.bin"), &fib_bytes).expect("write fibonacci.bin");

    let mut session = DebuggerSession::new();
    session.load_binary(0x001000, &fib_bytes, true);
    session.debugger.breakpoints.add_pc_breakpoint(0x001022); // halt
    session
        .debugger
        .run_until_breakpoint(&mut session.cpu, &mut session.bus, 1000);

    assert_eq!(session.cpu.state.pc.wrapping_sub(4), 0x001022);
    let expected_fib = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610];
    for (i, &val) in expected_fib.iter().enumerate() {
        let addr = 0x002000 + (i as u32 * 2);
        let read = session.bus.read_word_debug(addr);
        assert_eq!(read, val, "Fibonacci[{i}] mismatch at {addr:06X}");
    }

    // =========================================================================
    // 2. Bubble Sort ($001000)
    // =========================================================================
    #[rustfmt::skip]
    let sort_words: Vec<u16> = vec![
        // $1000: LEA sample_data(PC), A0 ($103A - $1002 = +$38)
        0x41FA, 0x0038,
        // $1004: LEA $002000, A1
        0x43F9, 0x0000, 0x2000,
        // $100A: MOVEQ #7, D0
        0x7007,
        // $100C: copy_loop: MOVE.W (A0)+, (A1)+
        0x32D8,
        // $100E: DBRA D0, copy_loop ($100C - $1010 = -4)
        0x51C8, 0xFFFC,
        // $1012: MOVE.W #6, D7
        0x3E3C, 0x0006,
        // $1016: outer_loop: LEA $002000, A0
        0x41F9, 0x0000, 0x2000,
        // $101C: MOVE.W D7, D6
        0x3C07,
        // $101E: inner_loop: MOVE.W (A0), D0
        0x3010,
        // $1020: MOVE.W 2(A0), D1
        0x3228, 0x0002,
        // $1024: CMP.W D1, D0
        0xB041,
        // $1026: BLS.S no_swap ($102E - $1028 = +6)
        0x6306,
        // $1028: MOVE.W D1, (A0)
        0x3081,
        // $102A: MOVE.W D0, 2(A0)
        0x3140, 0x0002,
        // $102E: no_swap: ADDQ.L #2, A0
        0x5488,
        // $1030: DBRA D6, inner_loop ($101E - $1032 = -$14 = 0xFFEC)
        0x51CE, 0xFFEC,
        // $1034: DBRA D7, outer_loop ($1016 - $1036 = -$20 = 0xFFE0)
        0x51CF, 0xFFE0,
        // $1038: halt: BRA.S halt
        0x60FE,
        // $103A: sample_data (8 words)
        0x0042, 0x0010, 0x0099, 0x0003, 0x0077, 0x0025, 0x0001, 0x0050,
    ];

    let sort_bytes = words_to_bytes(&sort_words);
    fs::write(out_dir.join("bubble_sort.bin"), &sort_bytes).expect("write bubble_sort.bin");

    let mut sort_session = DebuggerSession::new();
    sort_session.load_binary(0x001000, &sort_bytes, true);
    sort_session
        .debugger
        .breakpoints
        .add_pc_breakpoint(0x001038); // halt
    sort_session
        .debugger
        .run_until_breakpoint(&mut sort_session.cpu, &mut sort_session.bus, 5000);

    assert_eq!(sort_session.cpu.state.pc.wrapping_sub(4), 0x001038);
    let expected_sorted = [
        0x0001, 0x0003, 0x0010, 0x0025, 0x0042, 0x0050, 0x0077, 0x0099,
    ];
    for (i, &val) in expected_sorted.iter().enumerate() {
        let addr = 0x002000 + (i as u32 * 2);
        let read = sort_session.bus.read_word_debug(addr);
        assert_eq!(read, val, "BubbleSort[{i}] mismatch at {addr:06X}");
    }

    // =========================================================================
    // 3. Sieve of Eratosthenes ($001000)
    // =========================================================================
    #[rustfmt::skip]
    let sieve_words: Vec<u16> = vec![
        // $1000: LEA $002000, A0
        0x41F9, 0x0000, 0x2000,
        // $1006: MOVEQ #63, D0
        0x703F,
        // $1008: MOVEQ #1, D1
        0x7201,
        // $100A: init_loop: MOVE.B D1, 0(A0, D0.W)
        0x1181, 0x0800,
        // $100E: DBRA D0, init_loop ($100A - $1010 = -6 = 0xFFFA)
        0x51C8, 0xFFFA,
        // $1012: CLR.B (A0)
        0x4210,
        // $1014: CLR.B 1(A0)
        0x4228, 0x0001,
        // $1018: MOVEQ #2, D0
        0x7002,
        // $101A: sieve_outer: TST.B 0(A0, D0.W)
        0x4A30, 0x0800,
        // $101E: BEQ.S next_p ($1032 - $1020 = +$12)
        0x6712,
        // $1020: MOVE.W D0, D1
        0x3200,
        // $1022: ADD.W D0, D1
        0xD240,
        // $1024: strike_loop: CMPI.W #64, D1
        0x0C41, 0x0040,
        // $1028: BGE.S next_p ($1032 - $102A = +$08)
        0x6C08,
        // $102A: CLR.B 0(A0, D1.W)
        0x4230, 0x1800,
        // $102E: ADD.W D0, D1
        0xD240,
        // $1030: BRA.S strike_loop ($1024 - $1032 = -$0E = 0xFFF2)
        0x60F2,
        // $1032: next_p: ADDQ.W #1, D0
        0x5240,
        // $1034: CMPI.W #8, D0
        0x0C40, 0x0008,
        // $1038: BLT.S sieve_outer ($101A - $103A = -$20 = 0xFFE0)
        0x6DE0,
        // $103A: LEA $002100, A1
        0x43F9, 0x0000, 0x2100,
        // $1040: CLR.W D7
        0x4247,
        // $1042: MOVEQ #2, D0
        0x7002,
        // $1044: collect_loop: TST.B 0(A0, D0.W)
        0x4A30, 0x0800,
        // $1048: BEQ.S skip_collect ($104E - $104A = +$04)
        0x6704,
        // $104A: MOVE.B D0, (A1)+
        0x12C0,
        // $104C: ADDQ.W #1, D7
        0x5247,
        // $104E: skip_collect: ADDQ.W #1, D0
        0x5240,
        // $1050: CMPI.W #64, D0
        0x0C40, 0x0040,
        // $1054: BLT.S collect_loop ($1044 - $1056 = -$12 = 0xFFEE)
        0x6DEE,
        // $1056: halt: BRA.S halt
        0x60FE,
    ];

    let sieve_bytes = words_to_bytes(&sieve_words);
    fs::write(out_dir.join("sieve_primes.bin"), &sieve_bytes).expect("write sieve_primes.bin");

    let mut sieve_session = DebuggerSession::new();
    sieve_session.load_binary(0x001000, &sieve_bytes, true);
    sieve_session
        .debugger
        .breakpoints
        .add_pc_breakpoint(0x001056); // halt
    sieve_session.debugger.run_until_breakpoint(
        &mut sieve_session.cpu,
        &mut sieve_session.bus,
        10000,
    );

    assert_eq!(sieve_session.cpu.state.pc.wrapping_sub(4), 0x001056);
    // Prime count in D7 should be 18
    assert_eq!(sieve_session.cpu.state.d_long(7), 18);
    let expected_primes = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61,
    ];
    for (i, &p) in expected_primes.iter().enumerate() {
        let read = sieve_session.bus.read_byte_debug(0x002100 + i as u32);
        assert_eq!(read, p, "Prime[{i}] mismatch at $0021{:02X}", i);
    }

    // =========================================================================
    // 4. String Reversal & Palindrome ($001000)
    // =========================================================================
    // String 1: "AMIGA 500 RULEZ!" (16 bytes) copied to $002000 and reversed to $002040
    // String 2: "RACECAR" (7 bytes) tested for palindrome -> D0 = 1 if match
    #[rustfmt::skip]
    let string_words: Vec<u16> = vec![
        // $1000: LEA str1(PC), A0 ($1044 - $1002 = +$42)
        0x41FA, 0x0042,
        // $1004: LEA $002000, A1
        0x43F9, 0x0000, 0x2000,
        // $100A: MOVEQ #15, D0
        0x700F,
        // $100C: copy1: MOVE.B (A0)+, (A1)+
        0x12D8,
        // $100E: DBRA D0, copy1 ($100C - $1010 = -4 = 0xFFFC)
        0x51C8, 0xFFFC,

        // Reverse $002000 into $002040
        // $1012: LEA $002010, A1 (one byte past end of 16-byte string)
        0x43F9, 0x0000, 0x2010,
        // $1018: LEA $002040, A2
        0x45F9, 0x0000, 0x2040,
        // $101E: MOVEQ #15, D0
        0x700F,
        // $1020: rev_loop: MOVE.B -(A1), (A2)+
        0x14E1,
        // $1022: DBRA D0, rev_loop ($1020 - $1024 = -4 = 0xFFFC)
        0x51C8, 0xFFFC,

        // Palindrome check on "RACECAR" (7 chars at str2)
        // $1026: LEA str2(PC), A3 ($1054 - $1028 = +$2C)
        0x47FA, 0x002C,
        // $102A: LEA 7(A3), A4
        0x49E8, 0x0007,
        // $102E: MOVEQ #2, D0 (compare 3 pairs: 0&6, 1&5, 2&4)
        0x7002,
        // $1030: pal_loop: MOVE.B (A3)+, D1
        0x121B,
        // $1032: MOVE.B -(A4), D2
        0x1424,
        // $1034: CMP.B D1, D2
        0xB401,
        // $1036: BNE.S not_pal ($1040 - $1038 = +$08)
        0x6608,
        // $1038: DBRA D0, pal_loop ($1030 - $103A = -$0A = 0xFFF6)
        0x51C8, 0xFFF6,
        // $103C: MOVEQ #1, D0 (is palindrome!)
        0x7001,
        // $103E: halt: BRA.S halt
        0x60FE,
        // $1040: not_pal: MOVEQ #0, D0
        0x7000,
        // $1042: BRA.S halt ($103E - $1044 = -6 = 0xFFFA)
        0x60FA,

        // Data section:
        // $1044: str1: "AMIGA 500 RULEZ!" (16 bytes)
        0x414D, 0x4947, 0x4120, 0x3530, 0x3020, 0x5255, 0x4C45, 0x5A21,
        // $1054: str2: "RACECAR\0" (8 bytes)
        0x5241, 0x4345, 0x4341, 0x5200,
    ];

    let string_bytes = words_to_bytes(&string_words);
    fs::write(out_dir.join("string_reverse.bin"), &string_bytes).expect("write string_reverse.bin");

    let mut str_session = DebuggerSession::new();
    str_session.load_binary(0x001000, &string_bytes, true);
    str_session.debugger.breakpoints.add_pc_breakpoint(0x00103E); // halt
    str_session
        .debugger
        .run_until_breakpoint(&mut str_session.cpu, &mut str_session.bus, 2000);

    assert_eq!(str_session.cpu.state.pc.wrapping_sub(4), 0x00103E);
    // D0 should be 1 (palindrome verified)
    assert_eq!(str_session.cpu.state.d_long(0), 1);

    // Verify reversed string at $002040: "!ZELUR 005 AGIMA"
    let expected_rev = b"!ZELUR 005 AGIMA";
    for (i, &ch) in expected_rev.iter().enumerate() {
        let read = str_session.bus.read_byte_debug(0x002040 + i as u32);
        assert_eq!(read, ch, "Reversed string char[{i}] mismatch");
    }

    // =========================================================================
    // 5. Factorial with Subroutine & Stack Frame ($001000)
    // =========================================================================
    #[rustfmt::skip]
    let fact_words: Vec<u16> = vec![
        // $1000: LEA $008000, SP
        0x4FF9, 0x0000, 0x8000,
        // $1006: LEA $002000, A0
        0x41F9, 0x0000, 0x2000,
        // $100C: MOVEQ #1, D2
        0x7401,
        // $100E: main_loop: MOVE.W D2, D0
        0x3002,
        // $1010: BSR.W fact ($1020 - $1012 = +$0E)
        0x6100, 0x000E,
        // $1014: MOVE.L D0, (A0)+
        0x20C0,
        // $1016: ADDQ.W #1, D2
        0x5242,
        // $1018: CMPI.W #9, D2
        0x0C42, 0x0009,
        // $101C: BLT.S main_loop ($100E - $101E = -$10 = 0xFFF0)
        0x6DF0,
        // $101E: halt: BRA.S halt
        0x60FE,

        // Subroutine fact ($1020):
        // Inputs: D0.W = N
        // Outputs: D0.L = N!
        // $1020: MOVE.L D1, -(SP)
        0x2F01,
        // $1022: MOVE.W D0, D1
        0x3200,
        // $1024: MOVEQ #1, D0
        0x7001,
        // $1026: fact_loop: CMPI.W #1, D1
        0x0C41, 0x0001,
        // $102A: BLE.S fact_done ($1032 - $102C = +$06)
        0x6F06,
        // $102C: MULU.W D1, D0
        0xC0C1,
        // $102E: SUBQ.W #1, D1
        0x5341,
        // $1030: BRA.S fact_loop ($1026 - $1032 = -$0C = 0xFFF4)
        0x60F4,
        // $1032: fact_done: MOVE.L (SP)+, D1
        0x221F,
        // $1034: RTS
        0x4E75,
    ];

    let fact_bytes = words_to_bytes(&fact_words);
    fs::write(out_dir.join("factorial.bin"), &fact_bytes).expect("write factorial.bin");

    let mut fact_session = DebuggerSession::new();
    fact_session.load_binary(0x001000, &fact_bytes, true);
    fact_session
        .debugger
        .breakpoints
        .add_pc_breakpoint(0x00101E); // halt
    fact_session
        .debugger
        .run_until_breakpoint(&mut fact_session.cpu, &mut fact_session.bus, 5000);

    assert_eq!(fact_session.cpu.state.pc.wrapping_sub(4), 0x00101E);
    // Expected factorials 1! through 8!:
    let expected_facts: [u32; 8] = [1, 2, 6, 24, 120, 720, 5040, 40320];
    for (i, &f) in expected_facts.iter().enumerate() {
        let addr = 0x002000 + (i as u32 * 4);
        let hi = fact_session.bus.read_word_debug(addr) as u32;
        let lo = fact_session.bus.read_word_debug(addr + 2) as u32;
        let val = (hi << 16) | lo;
        assert_eq!(val, f, "Factorial[{}] mismatch at {addr:06X}", i + 1);
    }
}

fn words_to_bytes(words: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * 2);
    for &w in words {
        bytes.push((w >> 8) as u8);
        bytes.push((w & 0xFF) as u8);
    }
    bytes
}
