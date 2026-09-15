use config::BeamPosition;
use copper::{Copper, CopperState};

#[test]
fn test_copper_reset_and_restart() {
    let mut cop = Copper::new();
    cop.cop1lc = 0x00040000;
    cop.cop2lc = 0x00050000;

    cop.restart_list1();
    assert_eq!(cop.cop_pc, 0x00040000);
    assert!(cop.is_running);
    assert_eq!(cop.state, CopperState::FetchIR1(2));

    cop.restart_list2();
    assert_eq!(cop.cop_pc, 0x00050000);

    cop.set_copcon(0x0002);
    assert!(cop.cdang);

    cop.reset();
    assert_eq!(cop.cop_pc, 0);
    assert!(!cop.is_running);
    assert!(!cop.cdang);
    assert_eq!(cop.state, CopperState::Idle);
}

#[test]
fn test_copper_move_cycle_timing_and_data() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    // Copper list with two MOVE instructions:
    // MOVE $0180, $0F00 (COLOR00 = Red) -> Word 1: 0x0180, Word 2: 0x0F00
    // MOVE $0182, $00F0 (COLOR01 = Green) -> Word 1: 0x0182, Word 2: 0x00F0
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0x01, 0x80, 0x0F, 0x00]);
    ram[base + 4..base + 8].copy_from_slice(&[0x01, 0x82, 0x00, 0xF0]);

    let beam = BeamPosition::new(10, 5, false);

    // CCK 1: FetchIR1(2) ticks to FetchIR1(1)
    let w1 = cop.step_cck(beam, false, &ram);
    assert_eq!(w1, None);
    assert_eq!(cop.state, CopperState::FetchIR1(1));

    // CCK 2: FetchIR1(1), latches IR1 = 0x0180, cop_pc advances, transitions to FetchIR2(2)
    let w2 = cop.step_cck(beam, false, &ram);
    assert_eq!(w2, None);
    assert_eq!(cop.ir1, 0x0180);
    assert_eq!(cop.state, CopperState::FetchIR2(2));

    // CCK 3: FetchIR2(2) ticks to FetchIR2(1)
    let w3 = cop.step_cck(beam, false, &ram);
    assert_eq!(w3, None);
    assert_eq!(cop.state, CopperState::FetchIR2(1));

    // CCK 4: FetchIR2(1), latches IR2 = 0x0F00, commits MOVE to $0180!
    let w4 = cop.step_cck(beam, false, &ram);
    assert_eq!(w4, Some((0x0180, 0x0F00)));
    assert_eq!(cop.state, CopperState::FetchIR1(2));

    // Next instruction: 4 CCKs to commit COLOR01
    cop.step_cck(beam, false, &ram); // CCK 5: FetchIR1(2) -> FetchIR1(1)
    cop.step_cck(beam, false, &ram); // CCK 6: FetchIR1(1) -> FetchIR2(2)
    cop.step_cck(beam, false, &ram); // CCK 7: FetchIR2(2) -> FetchIR2(1)
    let w8 = cop.step_cck(beam, false, &ram); // CCK 8: FetchIR2(1) -> Commit
    assert_eq!(w8, Some((0x0182, 0x00F0)));
    assert_eq!(cop.cop_pc, 0x00010008);
}

#[test]
fn test_copper_danger_mode_cdang_halts_on_violation() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);

    // Instruction: MOVE $0040, $09F0 (BLTCON0 write)
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0x00, 0x40, 0x09, 0xF0]);

    let beam = BeamPosition::new(10, 5, false);

    // Case 1: CDANG == false -> register < $080 halts Copper!
    cop.cdang = false;
    cop.restart_list1();
    cop.step_cck(beam, false, &ram); // CCK 1
    cop.step_cck(beam, false, &ram); // CCK 2: IR1 latched
    cop.step_cck(beam, false, &ram); // CCK 3
    let blocked_write = cop.step_cck(beam, false, &ram); // CCK 4: IR2 -> Illegal write halts
    assert_eq!(blocked_write, None);
    assert_eq!(cop.state, CopperState::Idle);
    assert!(cop.is_waiting);

    // Case 2: CDANG == true -> register < $080 is allowed
    cop.cdang = true;
    cop.restart_list1();
    cop.step_cck(beam, false, &ram);
    cop.step_cck(beam, false, &ram);
    cop.step_cck(beam, false, &ram);
    let allowed_write = cop.step_cck(beam, false, &ram);
    assert_eq!(allowed_write, Some((0x0040, 0x09F0)));
}

#[test]
fn test_copper_wait_beam_position() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    // WAIT for Line 20 (0x14), HPOS 60 (0x3C | 1 = 0x3D) -> IR1 = 0x143D (bit 0 = 1)
    // IR2 = 0x80FE (BFD = 1, masks: VPOS mask 0x7F, HPOS mask 0xFE, bit 0 = 0 -> 0xFFFE)
    // Followed by: MOVE $0180, $0FFF
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0x14, 0x3D, 0xFF, 0xFE]);
    ram[base + 4..base + 8].copy_from_slice(&[0x01, 0x80, 0x0F, 0xFF]);

    // Fetch WAIT instruction across 4 CCKs at line 10, HPOS 0
    let mut beam = BeamPosition::new(0, 10, false);
    for _ in 0..4 {
        cop.step_cck(beam, false, &ram);
    }
    // Now Copper should be in Waiting state
    assert!(cop.is_waiting);
    assert_eq!(cop.state, CopperState::Waiting);

    // Step at line 15: still waiting (line 15 < line 20)
    beam = BeamPosition::new(50, 15, false);
    cop.step_cck(beam, false, &ram);
    assert!(cop.is_waiting);

    // Step at line 20, HPOS 40: still waiting (HPOS target is 60)
    beam = BeamPosition::new(40, 20, false);
    cop.step_cck(beam, false, &ram);
    assert!(cop.is_waiting);

    // Step at line 20, HPOS 58: Condition met! (58 + 2 = 60). Even cycle wakes up to FetchIR1(2)
    beam = BeamPosition::new(58, 20, false);
    cop.step_cck(beam, false, &ram);
    assert!(!cop.is_waiting);
    assert_eq!(cop.state, CopperState::FetchIR1(2));

    // Now 4 CCKs to fetch and commit the MOVE instruction
    cop.step_cck(beam, false, &ram);
    cop.step_cck(beam, false, &ram);
    cop.step_cck(beam, false, &ram);
    let move_res = cop.step_cck(beam, false, &ram);
    assert_eq!(move_res, Some((0x0180, 0x0FFF)));
}

#[test]
fn test_copper_wait_bfd_blitter_busy() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    // WAIT for Line 10 with BFD == 0 (Bit 15 = 0, waits for Blitter Finished)
    // Word 1: 0x0A01 (VPOS 10, HPOS 0, bit 0 = 1)
    // Word 2: 0x7FFE (Bit 15 = 0: BFD active; VPOS_MASK=0x7F, HPOS_MASK=0x7F, bit 0 = 0)
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0x0A, 0x01, 0x7F, 0xFE]);

    let beam = BeamPosition::new(0, 10, false);

    // Fetch the instruction (4 CCKs)
    for _ in 0..4 {
        cop.step_cck(beam, true, &ram);
    }
    assert!(cop.is_waiting);

    // Beam is at line 10, but Blitter is busy: must continue waiting!
    cop.step_cck(beam, true, &ram);
    assert!(cop.is_waiting);

    // Blitter completes (busy = false): on even cycle condition is satisfied!
    cop.step_cck(beam, false, &ram);
    assert!(!cop.is_waiting);
    assert_eq!(cop.state, CopperState::FetchIR1(2));
}

#[test]
fn test_copper_skip_conditional_bypass() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);

    // SKIP when VPOS >= 20: Word 1 = 0x1401, Word 2 = 0xFFFF (Bit 0 = 1 -> SKIP)
    // Inst 2 (to be skipped if VPOS >= 20): MOVE $0180, $0111
    // Inst 3: MOVE $0182, $0222
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0x14, 0x01, 0xFF, 0xFF]);
    ram[base + 4..base + 8].copy_from_slice(&[0x01, 0x80, 0x01, 0x11]);
    ram[base + 8..base + 12].copy_from_slice(&[0x01, 0x82, 0x02, 0x22]);

    // Test Case A: Beam is at line 10 (< 20). SKIP condition fails.
    let beam_low = BeamPosition::new(0, 10, false);
    cop.restart_list1();
    for _ in 0..4 {
        cop.step_cck(beam_low, false, &ram);
    }
    // Next instruction fetched is Inst 2 (MOVE $0180, $0111) in 4 CCKs
    for _ in 0..3 {
        cop.step_cck(beam_low, false, &ram);
    }
    let inst2_write = cop.step_cck(beam_low, false, &ram);
    assert_eq!(inst2_write, Some((0x0180, 0x0111)));

    // Test Case B: Beam is at line 25 (>= 20). SKIP condition succeeds!
    let beam_high = BeamPosition::new(0, 25, false);
    cop.restart_list1();
    for _ in 0..4 {
        cop.step_cck(beam_high, false, &ram);
    }
    // cop_pc skipped Inst 2 and points to Inst 3!
    assert_eq!(cop.cop_pc, 0x00010008);
    // Fetch Inst 3 in 4 CCKs
    for _ in 0..3 {
        cop.step_cck(beam_high, false, &ram);
    }
    let inst3_write = cop.step_cck(beam_high, false, &ram);
    assert_eq!(inst3_write, Some((0x0182, 0x0222)));
}

#[test]
fn test_copper_terminator_and_vblank_restart() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    // WAIT $FFFF, $FFFE (Standard list terminator)
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFE]);

    let beam = BeamPosition::new(50, 100, false);
    for _ in 0..4 {
        cop.step_cck(beam, false, &ram);
    }
    // Terminator halts Copper into Idle
    assert_eq!(cop.state, CopperState::Idle);
    assert!(cop.is_waiting);

    // Reaching VBlank (Line 0, HPOS 0) automatically restarts Copper list 1 and begins fetch
    let vblank_beam0 = BeamPosition::new(0, 0, false);
    cop.step_cck(vblank_beam0, false, &ram);
    assert_eq!(cop.state, CopperState::FetchIR1(1));
    let vblank_beam1 = BeamPosition::new(1, 0, false);
    cop.step_cck(vblank_beam1, false, &ram);
    assert_eq!(cop.cop_pc, 0x00010002);
    assert!(cop.is_running);
    assert_eq!(cop.state, CopperState::FetchIR2(2));
}

#[test]
fn test_copper_wait_vertical_boundary_cross_above_line_128() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    // WAIT $FFDF, $FFFE (Cross vertical boundary at line 255, hpos 222)
    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    ram[base..base + 4].copy_from_slice(&[0xFF, 0xDF, 0xFF, 0xFE]);

    // Beam at line 127, hpos 222: must NOT wake up (bit 7 of vertical mask must be forced on)
    let beam_127 = BeamPosition::new(222, 127, false);
    for _ in 0..4 {
        cop.step_cck(beam_127, false, &ram);
    }
    assert_eq!(cop.state, CopperState::Waiting);
    assert!(cop.is_waiting);

    // Beam at line 255, hpos 222: matches target coordinates on even cycle (222 is even)
    let beam_255 = BeamPosition::new(222, 255, false);
    cop.step_cck(beam_255, false, &ram);
    assert_eq!(cop.state, CopperState::FetchIR1(2));
    assert!(!cop.is_waiting);
}

#[test]
fn test_copper_dma_denied_at_cycle_e0() {
    let mut cop = Copper::new();
    cop.set_dma_enabled(true);
    cop.set_cop1lc(0x00010000);
    cop.restart_list1();

    let mut ram = vec![0u8; 0x20000];
    let base = 0x10000;
    // MOVE $0180, $0F00
    ram[base..base + 4].copy_from_slice(&[0x01, 0x80, 0x0F, 0x00]);

    // Beam at HPOS 0xE0: Copper DMA is denied by Agnus hardware
    let beam_e0 = BeamPosition::new(0xE0, 50, false);
    let res = cop.step_cck(beam_e0, false, &ram);
    assert_eq!(res, None);
    // State must not advance while stalled at cycle 0xE0
    assert_eq!(cop.state, CopperState::FetchIR1(2));

    // When advancing to HPOS 0xE1 (free cycle), fetch progresses normally
    let beam_e1 = BeamPosition::new(0xE1, 50, false);
    cop.step_cck(beam_e1, false, &ram);
    assert_eq!(cop.state, CopperState::FetchIR1(1));
}
