//! HRM chapter 6 function generator and right-to-left area fill.

/// Select C, then B, then A branches of the eight-bit truth table.
pub fn eval_minterm(a: u16, b: u16, c: u16, minterm: u8) -> u16 {
    let mut c_terms = [0u16; 4];
    for (pair, term) in c_terms.iter_mut().enumerate() {
        let low = 0u16.wrapping_sub(u16::from((minterm >> (pair * 2)) & 1));
        let high = 0u16.wrapping_sub(u16::from((minterm >> (pair * 2 + 1)) & 1));
        *term = (c & high) | (!c & low);
    }
    let a_low = (b & c_terms[1]) | (!b & c_terms[0]);
    let a_high = (b & c_terms[3]) | (!b & c_terms[2]);
    (a & a_high) | (!a & a_low)
}

/// The original boundary bit toggles carry after producing the current bit.
pub fn apply_fill(data: u16, carry: &mut bool, exclusive: bool) -> u16 {
    let mut result = 0;
    for bit in 0..u16::BITS {
        let boundary = data & (1 << bit) != 0;
        let output = if exclusive {
            boundary ^ *carry
        } else {
            boundary | *carry
        };
        result |= u16::from(output) << bit;
        *carry ^= boundary;
    }
    result
}
