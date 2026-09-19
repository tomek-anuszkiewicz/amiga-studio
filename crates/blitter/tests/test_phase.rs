#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Dedicated unit tests for Blitter phases and Table 6.2 channel cycle sequencing

use blitter::{barrel_shift, word_phases, BlitterPhase};

#[test]
fn test_table_6_2_word_phases_zero_and_single_channels() {
    // 0: No channels enabled (2 idle cycles)
    let (p0, c0) = word_phases(false, false, false, false, false);
    assert_eq!(c0, 2);
    assert_eq!(p0[0], BlitterPhase::BusIdle);
    assert_eq!(p0[1], BlitterPhase::BusIdle);

    // 1: D only
    let (p1, c1) = word_phases(false, false, false, true, false);
    assert_eq!(c1, 2);
    assert_eq!(p1[0], BlitterPhase::BusIdle);
    assert_eq!(p1[1], BlitterPhase::WriteD);

    // 1 with fill
    let (p1f, c1f) = word_phases(false, false, false, true, true);
    assert_eq!(c1f, 3);
    assert_eq!(p1f[0], BlitterPhase::BusIdle);
    assert_eq!(p1f[1], BlitterPhase::WriteD);
    assert_eq!(p1f[2], BlitterPhase::BusIdle);

    // 2: C only
    let (p2, c2) = word_phases(false, false, true, false, false);
    assert_eq!(c2, 2);
    assert_eq!(p2[0], BlitterPhase::BusIdle);
    assert_eq!(p2[1], BlitterPhase::FetchC);

    // 4: B only
    let (p4, c4) = word_phases(false, true, false, false, false);
    assert_eq!(c4, 3);
    assert_eq!(p4[0], BlitterPhase::BusIdle);
    assert_eq!(p4[1], BlitterPhase::FetchB);
    assert_eq!(p4[2], BlitterPhase::BusIdle);

    // 8: A only
    let (p8, c8) = word_phases(true, false, false, false, false);
    assert_eq!(c8, 2);
    assert_eq!(p8[0], BlitterPhase::FetchA);
    assert_eq!(p8[1], BlitterPhase::BusIdle);
}

#[test]
fn test_table_6_2_word_phases_multi_channel_and_barrel_shift() {
    // 15: ABCD
    let (p15, c15) = word_phases(true, true, true, true, false);
    assert_eq!(c15, 4);
    assert_eq!(p15[0], BlitterPhase::FetchA);
    assert_eq!(p15[1], BlitterPhase::FetchB);
    assert_eq!(p15[2], BlitterPhase::FetchC);
    assert_eq!(p15[3], BlitterPhase::WriteD);

    // 7: BCD
    let (p7, c7) = word_phases(false, true, true, true, false);
    assert_eq!(c7, 4);
    assert_eq!(p7[0], BlitterPhase::BusIdle);
    assert_eq!(p7[1], BlitterPhase::FetchB);
    assert_eq!(p7[2], BlitterPhase::FetchC);
    assert_eq!(p7[3], BlitterPhase::WriteD);

    // 11: ACD
    let (p11, c11) = word_phases(true, false, true, true, false);
    assert_eq!(c11, 3);
    assert_eq!(p11[0], BlitterPhase::FetchA);
    assert_eq!(p11[1], BlitterPhase::FetchC);
    assert_eq!(p11[2], BlitterPhase::WriteD);

    // Barrel shifter verification
    let w0 = 0x1234u16;
    let w1 = 0x5678u16;

    // Ascending shift by 4: (0x1234_5678 >> 4) = 0x4567
    let shifted_asc = barrel_shift(w1, w0, 4, false);
    assert_eq!(shifted_asc, 0x4567);

    // Descending shift by 4: (0x5678_1234 >> 12) = 0x6781
    let shifted_desc = barrel_shift(w1, w0, 4, true);
    assert_eq!(shifted_desc, 0x6781);
}
