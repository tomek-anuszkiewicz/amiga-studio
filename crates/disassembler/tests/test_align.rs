use disassembler::find_aligned_disassembly_start;
use std::collections::HashMap;

#[test]
fn test_find_aligned_disassembly_start_edge_cases() {
    let dummy_mem = |_| 0x4E71; // NOP

    // desired_prior_instructions == 0 returns target_pc
    assert_eq!(
        find_aligned_disassembly_start(0x1000, 0, dummy_mem, &[]),
        0x1000
    );

    // target_pc == 0 returns 0
    assert_eq!(
        find_aligned_disassembly_start(0x0000, 2, dummy_mem, &[]),
        0x0000
    );

    // Odd address masked to even word boundary
    assert_eq!(
        find_aligned_disassembly_start(0x1001, 0, dummy_mem, &[]),
        0x1000
    );
}

#[test]
fn test_find_aligned_disassembly_start_from_history() {
    // Memory with sequence of NOPs (0x4E71, 2 bytes each)
    // 0x1000: NOP
    // 0x1002: NOP
    // 0x1004: NOP
    // Target is 0x1006.
    let mem = |addr| match addr {
        0x1000..=0x1006 => 0x4E71,
        _ => 0x0000,
    };

    let history = [0x1000, 0x1002, 0x1004];
    // Request 2 prior instructions: from 0x1006, 2 prior NOPs should land at 0x1002
    let start = find_aligned_disassembly_start(0x1006, 2, mem, &history);
    assert_eq!(start, 0x1002);
}

#[test]
fn test_find_aligned_disassembly_start_heuristic_clean_stream() {
    // Memory map without history:
    // 0x2000 has 0x0000 (zero padding, penalized by heuristic -60)
    // 0x2002: ADDQ.L #1, D0  (0x5280, 2 bytes)
    // 0x2004: NOP            (0x4E71, 2 bytes)
    // 0x2006: target
    let mem = |addr| match addr {
        0x2002 => 0x5280,
        0x2004 => 0x4E71,
        _ => 0x0000,
    };

    // Asking for 2 prior instructions: avoids 0x2000 (penalized zero padding) and anchors at 0x2002
    let start = find_aligned_disassembly_start(0x2006, 2, mem, &[]);
    assert_eq!(start, 0x2002);
}

#[test]
fn test_find_aligned_disassembly_start_variable_length_sync() {
    // Memory containing variable length instructions:
    // 0x3000: MOVE.L #$12345678, D0  (0x203C, 0x1234, 0x5678 -> 6 bytes)
    // 0x3006: NOP                    (0x4E71 -> 2 bytes)
    // 0x3008: target
    let mut map = HashMap::new();
    map.insert(0x3000, 0x203C);
    map.insert(0x3002, 0x1234);
    map.insert(0x3004, 0x5678);
    map.insert(0x3006, 0x4E71);

    let mem = move |addr| *map.get(&addr).unwrap_or(&0x0000);

    // If we request 2 instructions back, it must not desynchronize by starting inside
    // the immediate operands (0x3002 or 0x3004), but anchor at 0x3000!
    let start2 = find_aligned_disassembly_start(0x3008, 2, &mem, &[]);
    assert_eq!(start2, 0x3000);
}

#[test]
fn test_find_aligned_disassembly_start_fibonacci() {
    // Memory layout:
    // $0FFE: 0000 (padding before entry)
    // $1000: 41F9 0000 2000 (LEA ($2000).L, A0 - 6 bytes)
    // $1006: 4240 (CLR.W D0 - 2 bytes)
    // $1008: 323C 0001 (MOVE.W #1, D1 - 4 bytes)
    // $100C: 4E71 (NOP)
    let read = |pc: u32| match pc {
        0x1000 => 0x41F9,
        0x1002 => 0x0000,
        0x1004 => 0x2000,
        0x1006 => 0x4240,
        0x1008 => 0x323C,
        0x100A => 0x0001,
        0x100C => 0x4E71,
        _ => 0x0000,
    };

    // 1. At entry point ($1000): should NOT back up into zeros ($0FFC..$0FFE)
    let start_at_entry = find_aligned_disassembly_start(0x1000, 3, read, &[]);
    assert_eq!(start_at_entry, 0x1000);

    // 2. At second instruction ($1006): with history [0x1000]
    let start_at_1006_with_hist = find_aligned_disassembly_start(0x1006, 3, read, &[0x1000]);
    assert_eq!(start_at_1006_with_hist, 0x1000);

    // 3. At second instruction ($1006): without history (pure heuristic code guessing)
    let start_at_1006_no_hist = find_aligned_disassembly_start(0x1006, 3, read, &[]);
    assert_eq!(start_at_1006_no_hist, 0x1000);

    // 4. At third instruction ($1008): should anchor at $1000
    let start_at_1008 = find_aligned_disassembly_start(0x1008, 3, read, &[0x1000, 0x1006]);
    assert_eq!(start_at_1008, 0x1000);
}
