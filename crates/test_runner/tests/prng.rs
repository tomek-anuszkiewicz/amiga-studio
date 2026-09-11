//! Deterministic PRNG & Domain Clamping Unit & Integration Tests
//!
//! Validates XorShift64 sequence determinism against canonical seed BENCH_PRNG_SEED,
//! packed BCD generation, non-zero divisors, and memory buffer population.

use test_runner::benchmark::{XorShift64, BENCH_PRNG_SEED};

#[test]
fn test_prng_determinism() {
    let mut prng1 = XorShift64::new(BENCH_PRNG_SEED);
    let mut prng2 = XorShift64::new(BENCH_PRNG_SEED);

    for _ in 0..100 {
        assert_eq!(prng1.next_u64(), prng2.next_u64());
    }
}

#[test]
fn test_prng_domain_clamping_bcd() {
    let mut prng = XorShift64::default();
    for _ in 0..1000 {
        let bcd = prng.next_bcd_byte();
        let high = bcd >> 4;
        let low = bcd & 0x0F;
        assert!(high <= 9, "High nibble {} exceeds 9", high);
        assert!(low <= 9, "Low nibble {} exceeds 9", low);
    }
}

#[test]
fn test_prng_non_zero_divisors() {
    let mut prng = XorShift64::new(12345);
    for _ in 0..1000 {
        let u_div = prng.next_non_zero_u16();
        assert_ne!(u_div, 0, "Unsigned divisor must never be zero");

        let s_div = prng.next_non_zero_i16();
        assert_ne!(s_div, 0, "Signed divisor must never be zero");
    }
}

#[test]
fn test_prng_buffer_filling_and_alignment() {
    let mut prng = XorShift64::default();
    let mut buf = [0u8; 64];
    prng.fill_bytes(&mut buf);

    // Verify buffer was populated with non-zero bytes
    assert!(buf.iter().any(|&b| b != 0));

    // Test aligned Chip RAM address clamping
    for _ in 0..100 {
        let addr = prng.next_aligned_chip_ram_addr(0x006000, 0x007000);
        assert!(addr >= 0x006000 && addr <= 0x007000);
        assert_eq!(addr & 1, 0, "Chip RAM address must be word-aligned (even)");
    }
}
