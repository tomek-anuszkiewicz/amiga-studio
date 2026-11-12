use agnus::copper::{Copper, CopperState};

fn running() -> Copper {
    let mut copper = Copper::new();
    copper.cop1lc = 0x1000;
    copper.restart_list1();
    copper.set_dma_enabled(true);
    copper
}

fn instruction(
    copper: &mut Copper,
    words: [u16; 2],
    hpos: u16,
    vpos: u16,
    busy: bool,
) -> Option<(u16, u16)> {
    assert_eq!(copper.receive_dma_word(words[0], hpos, vpos, busy), None);
    copper.receive_dma_word(words[1], hpos, vpos, busy)
}

#[test]
fn move_uses_two_words_and_enforces_both_ocs_address_limits() {
    // HRM 2 MOVE / COPCON; HRM Appendix B register restriction markers.
    for (danger, register, allowed) in [
        (false, 0x3E, false),
        (true, 0x3E, false),
        (false, 0x40, false),
        (true, 0x40, true),
        (false, 0x7E, false),
        (true, 0x7E, true),
        (false, 0x80, true),
        (false, 0x1FE, true),
    ] {
        let mut copper = running();
        copper.set_copcon(if danger { 2 } else { 0 });
        let result = instruction(&mut copper, [register, 0x1234], 0x40, 20, false);
        assert_eq!(copper.cop_pc, 0x1004);
        assert_eq!(result, allowed.then_some((register, 0x1234)));
        assert_eq!(copper.state == CopperState::Idle, !allowed);
    }
}

#[test]
fn disabling_dma_preserves_the_first_word_and_resumes_the_second() {
    let mut copper = running();
    assert_eq!(copper.receive_dma_word(0x180, 0x40, 20, false), None);
    copper.set_dma_enabled(false);
    for hpos in 0x41..0x60 {
        copper.step_cck(hpos, 20, false);
        assert_eq!(copper.receive_dma_word(0xFFFF, hpos, 20, false), None);
    }
    assert_eq!(copper.cop_pc, 0x1002);
    assert_eq!(copper.ir1, 0x180);
    copper.set_dma_enabled(true);
    assert_eq!(
        copper.receive_dma_word(0x0F00, 0x60, 20, false),
        Some((0x180, 0x0F00))
    );
}

#[test]
fn wait_uses_masked_ordered_positions_and_blitter_finish() {
    // HRM 2 WAIT, comparison enables and Using the Copper with the Blitter.
    for (words, hpos, vpos, busy, ready) in [
        ([0x2041, 0xFFFE], 0x40, 0x1F, false, false),
        ([0x2041, 0xFFFE], 0x3E, 0x20, false, false),
        ([0x2041, 0xFFFE], 0x40, 0x20, false, true),
        ([0x2041, 0xFFFE], 0x10, 0x21, false, true),
        ([0x2041, 0xFF00], 0x10, 0x20, false, true),
        ([0x8041, 0x8000], 0x40, 0x7F, false, false),
        ([0x8041, 0x8000], 0x00, 0x80, false, true),
        ([0x0041, 0x80FE], 0x40, 0x100, false, true),
        ([0x2041, 0x7FFE], 0x40, 0x20, true, false),
        ([0x2041, 0x7FFE], 0x40, 0x20, false, true),
        ([0x2041, 0xFFFE], 0x40, 0x20, true, true),
    ] {
        let mut copper = running();
        instruction(&mut copper, words, 2, 1, false);
        copper.step_cck(hpos, vpos, busy);
        assert_eq!(
            copper.is_active_fetching(),
            ready,
            "{words:04X?} at {vpos:04X}/{hpos:02X}"
        );
        assert_eq!(copper.cop_pc, 0x1004);
    }
}

#[test]
fn terminal_wait_is_impossible_in_both_standards_until_frame_restart() {
    for lines in [262, 263, 312, 313] {
        let mut copper = running();
        instruction(&mut copper, [0xFFFF, 0xFFFE], 2, 1, false);
        for vpos in 0..lines {
            for hpos in 1..=0xE3 {
                copper.step_cck(hpos, vpos, false);
                assert_eq!(copper.state, CopperState::Waiting);
            }
        }
        copper.step_cck(0, 0, false);
        assert_eq!(copper.cop_pc, 0x1000);
        assert!(copper.is_active_fetching());
    }
}

#[test]
fn skip_discards_exactly_one_complete_instruction() {
    for skipped in [[0x180, 0x0F00], [0xFFFF, 0xFFFE], [0x0001, 0xFFFF]] {
        let mut copper = running();
        instruction(&mut copper, [0x2001, 0xFFFF], 0x40, 0x21, false);
        assert!(copper.discard_next);
        assert_eq!(instruction(&mut copper, skipped, 0x44, 0x21, false), None);
        assert_eq!(copper.cop_pc, 0x1008);
        assert!(!copper.discard_next);
        assert_eq!(
            instruction(&mut copper, [0x180, 0x00F0], 0x48, 0x21, false),
            Some((0x180, 0x00F0))
        );
    }
}

#[test]
fn skip_preserves_the_next_instruction_when_beam_or_blitter_condition_fails() {
    for (vpos, busy, control) in [(0x1F, false, 0xFFFF), (0x21, true, 0x7FFF)] {
        let mut copper = running();
        instruction(&mut copper, [0x2001, control], 0x40, vpos, busy);
        assert!(!copper.discard_next);
        assert_eq!(
            instruction(&mut copper, [0x180, 0x0F00], 0x44, vpos, busy),
            Some((0x180, 0x0F00))
        );
    }
}

#[test]
fn list_strobes_clear_skip_state_and_dma_pointer_wraps() {
    let mut copper = running();
    copper.cop1lc = 0xFFFF_FFFF;
    copper.cop2lc = 0x1234;
    copper.discard_next = true;
    copper.restart_list1();
    assert_eq!(copper.cop_pc, 0x7FFFE);
    assert!(!copper.discard_next);
    assert_eq!(copper.receive_dma_word(0x180, 0x40, 20, false), None);
    assert_eq!(copper.cop_pc, 0);
    copper.restart_list2();
    assert_eq!(copper.cop_pc, 0x1234);
    assert_eq!(copper.state, CopperState::FetchIR1);
    copper.reset();
    assert_eq!(copper, Copper::new());
}
