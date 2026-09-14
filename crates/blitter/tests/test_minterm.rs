//! Unit Tests for Agnus Blitter 256-Minterm Boolean ALU & Fill Logic
//!
//! Validates `eval_minterm` across all 8 truth-table terms, standard Amiga graphic
//! minterms, and `apply_fill` in both inclusive and exclusive modes.

use blitter::{apply_fill, eval_minterm};

#[test]
fn test_eval_minterm_individual_terms() {
    let a = 0b1111_0000_1111_0000u16;
    let b = 0b1100_1100_1100_1100u16;
    let c = 0b1010_1010_1010_1010u16;

    // Test each term in isolation:
    // LF7: A & B & C (bit 7)
    let lf7 = a & b & c;
    assert_eq!(eval_minterm(a, b, c, 0x80), lf7);

    // LF6: A & B & !C (bit 6)
    let lf6 = a & b & !c;
    assert_eq!(eval_minterm(a, b, c, 0x40), lf6);

    // LF5: A & !B & C (bit 5)
    let lf5 = a & !b & c;
    assert_eq!(eval_minterm(a, b, c, 0x20), lf5);

    // LF4: A & !B & !C (bit 4)
    let lf4 = a & !b & !c;
    assert_eq!(eval_minterm(a, b, c, 0x10), lf4);

    // LF3: !A & B & C (bit 3)
    let lf3 = !a & b & c;
    assert_eq!(eval_minterm(a, b, c, 0x08), lf3);

    // LF2: !A & B & !C (bit 2)
    let lf2 = !a & b & !c;
    assert_eq!(eval_minterm(a, b, c, 0x04), lf2);

    // LF1: !A & !B & C (bit 1)
    let lf1 = !a & !b & c;
    assert_eq!(eval_minterm(a, b, c, 0x02), lf1);

    // LF0: !A & !B & !C (bit 0)
    let lf0 = !a & !b & !c;
    assert_eq!(eval_minterm(a, b, c, 0x01), lf0);

    // All terms combined should equal 0xFFFF
    assert_eq!(eval_minterm(a, b, c, 0xFF), 0xFFFF);
    // Zero minterm should equal 0x0000
    assert_eq!(eval_minterm(a, b, c, 0x00), 0x0000);
}

#[test]
fn test_eval_minterm_classic_amiga_modes() {
    let a = 0xAA55u16; // Mask / Source A
    let b = 0xF0F0u16; // Source B
    let c = 0x0F0Fu16; // Background C

    // Cookie-cut: A*B + !A*C -> Minterm 0xCA
    let cookie_cut = (a & b) | (!a & c);
    assert_eq!(eval_minterm(a, b, c, 0xCA), cookie_cut);

    // Straight copy: D = A (Minterm 0xF0)
    assert_eq!(eval_minterm(a, b, c, 0xF0), a);

    // Invert source: D = !A (Minterm 0x0F)
    assert_eq!(eval_minterm(a, b, c, 0x0F), !a);

    // D = A XOR C (Minterm 0x5A)
    assert_eq!(eval_minterm(a, b, c, 0x5A), a ^ c);

    // D = A OR C (Minterm 0xFA)
    assert_eq!(eval_minterm(a, b, c, 0xFA), a | c);
}

#[test]
fn test_apply_fill_inclusive_and_exclusive() {
    // A single 1-bit at bit 2: 0b0000_0100 = 0x04
    let mut carry = false;
    let inclusive = apply_fill(0x0004, &mut carry, false);
    // In inclusive fill, bit 2 sets carry; bits 3..15 should be filled with 1s
    assert_eq!(inclusive, 0xFFFC);
    assert!(carry); // carry should remain true at word end

    // Feeding the next word with incoming carry = true
    let next = apply_fill(0x0000, &mut carry, false);
    assert_eq!(next, 0xFFFF);
    assert!(carry);

    // Toggling carry off with a 1-bit at bit 4 in next word
    let toggled = apply_fill(0x0010, &mut carry, false);
    // bits 0..3 filled with 1s, bit 4 toggles carry off, bits 5..15 are 0
    assert_eq!(toggled & 0x000F, 0x000F);
    assert_eq!(toggled & 0xFFE0, 0x0000);
    assert!(!carry);

    // Exclusive fill: filled bits XOR with data
    let mut ex_carry = false;
    let exclusive = apply_fill(0x0004, &mut ex_carry, true);
    // Original bit 2 is 1; fill from bit 3 onward: bits 2..15 are 1 (0xFFFC)
    assert_eq!(exclusive, 0xFFFC);
    assert!(ex_carry);
}
