//! 256-Minterm Boolean ALU & Fill Logic for Agnus Blitter
//!
//! Implements the 8-bit truth table (LF0..LF7) combining channels A, B, and C
//! into destination D, as well as inclusive and exclusive fill modes.

/// Evaluates the 256-minterm Boolean ALU for 16-bit word operands A, B, and C.
///
/// Bits 7..0 of `minterm` correspond to truth table terms LF7..LF0:
/// - LF7 (bit 7): A & B & C
/// - LF6 (bit 6): A & B & !C
/// - LF5 (bit 5): A & !B & C
/// - LF4 (bit 4): A & !B & !C
/// - LF3 (bit 3): !A & B & C
/// - LF2 (bit 2): !A & B & !C
/// - LF1 (bit 1): !A & !B & C
/// - LF0 (bit 0): !A & !B & !C
#[inline(always)]
pub fn eval_minterm(a: u16, b: u16, c: u16, minterm: u8) -> u16 {
    let mut res = 0u16;
    if (minterm & 0x80) != 0 {
        res |= a & b & c;
    }
    if (minterm & 0x40) != 0 {
        res |= a & b & !c;
    }
    if (minterm & 0x20) != 0 {
        res |= a & !b & c;
    }
    if (minterm & 0x10) != 0 {
        res |= a & !b & !c;
    }
    if (minterm & 0x08) != 0 {
        res |= !a & b & c;
    }
    if (minterm & 0x04) != 0 {
        res |= !a & b & !c;
    }
    if (minterm & 0x02) != 0 {
        res |= !a & !b & c;
    }
    if (minterm & 0x01) != 0 {
        res |= !a & !b & !c;
    }
    res
}

/// Applies fill mode to a 16-bit word from right to left (low byte then high byte).
///
/// In inclusive fill (`exclusive == false`), filled bits are combined using OR (`|`).
/// In exclusive fill (`exclusive == true`), filled bits are combined using XOR (`^`).
/// The fill carry toggles whenever an original 1-bit is encountered.
#[inline]
pub fn apply_fill(data: u16, carry: &mut bool, exclusive: bool) -> u16 {
    let lo = (data & 0xFF) as u8;
    let hi = (data >> 8) as u8;

    let (res_lo, c1) = fill_byte(lo, *carry, exclusive);
    let (res_hi, c2) = fill_byte(hi, c1, exclusive);
    *carry = c2;

    ((res_hi as u16) << 8) | (res_lo as u16)
}

#[inline(always)]
fn fill_byte(byte: u8, mut carry: bool, exclusive: bool) -> (u8, bool) {
    let mut result = byte;
    for bit in 0..8 {
        let c_val = if carry { 1 << bit } else { 0 };
        if exclusive {
            result ^= c_val;
        } else {
            result |= c_val;
        }
        if (byte & (1 << bit)) != 0 {
            carry = !carry;
        }
    }
    (result, carry)
}
