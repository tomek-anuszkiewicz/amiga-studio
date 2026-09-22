//! Deterministic Pseudo-Random Number Generator (PRNG) & Domain Clamping for M68000 Benchmarking
//!
//! Provides a fast, deterministic 64-bit PRNG (XorShift64) to ensure varied, non-zero operand
//! distributions and prevent host CPU ALU/branch shortcuts, per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

/// Canonical seed for reproducible benchmark execution across runs and host architectures
pub const BENCH_PRNG_SEED: u64 = 0x8547_5938_4721_9381;

/// Deterministic 64-bit XorShift pseudo-random number generator
#[derive(Debug, Clone)]
pub struct XorShift64 {
    state: u64,
}

impl Default for XorShift64 {
    #[inline]
    fn default() -> Self {
        Self::new(BENCH_PRNG_SEED)
    }
}

impl XorShift64 {
    /// Creates a new PRNG with the specified non-zero seed.
    /// If seed is 0, defaults to `BENCH_PRNG_SEED`.
    #[inline]
    pub const fn new(seed: u64) -> Self {
        let state = if seed == 0 { BENCH_PRNG_SEED } else { seed };
        Self { state }
    }

    /// Generates the next pseudo-random 64-bit unsigned integer
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generates the next pseudo-random 32-bit unsigned integer
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Generates the next pseudo-random 16-bit unsigned integer
    #[inline]
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }

    /// Fills a memory buffer with pseudo-random bytes
    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_exact_mut(8) {
            let val = self.next_u64().to_be_bytes();
            chunk.copy_from_slice(&val);
        }
        let rem = buf.chunks_exact_mut(8).into_remainder();
        if !rem.is_empty() {
            let val = self.next_u64().to_be_bytes();
            rem.copy_from_slice(&val[..rem.len()]);
        }
    }

    /// Returns a non-zero 16-bit divisor suitable for `DIVU` (range 1..=0xFFFF)
    #[inline]
    pub fn next_non_zero_u16(&mut self) -> u16 {
        let val = self.next_u16();
        if val == 0 {
            0x00A5
        } else {
            val
        }
    }

    /// Returns a non-zero 16-bit signed divisor suitable for `DIVS`
    #[inline]
    pub fn next_non_zero_i16(&mut self) -> i16 {
        let val = self.next_u16() as i16;
        if val == 0 {
            0x007B
        } else {
            val
        }
    }

    /// Returns a valid packed BCD byte containing two decimal digits (high nibble 0..9, low nibble 0..9)
    #[inline]
    pub fn next_bcd_byte(&mut self) -> u8 {
        let r = self.next_u16();
        let high = ((r >> 8) % 10) as u8;
        let low = ((r & 0xFF) % 10) as u8;
        (high << 4) | low
    }

    /// Returns a 32-bit word-aligned Chip RAM address strictly within `[min_addr, max_addr]`
    #[inline]
    pub fn next_aligned_chip_ram_addr(&mut self, min_addr: u32, max_addr: u32) -> u32 {
        let span = max_addr.saturating_sub(min_addr);
        if span == 0 {
            return min_addr & !1;
        }
        let offset = (self.next_u32() % span) & !1;
        (min_addr + offset) & !1
    }
}
