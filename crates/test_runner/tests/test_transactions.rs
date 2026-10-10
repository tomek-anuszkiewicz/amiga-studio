#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::transactions::{
    match_transactions, parse_transactions, BusAccessSize, RecordedTransaction,
};

#[test]
fn test_reject_shifted_internal_cycles() {
    let recorded = [
        RecordedTransaction {
            clock: 4,
            is_read: true,
            addr: 0x1000,
            size: BusAccessSize::Word,
            data: 0x1234,
        },
        RecordedTransaction {
            clock: 10,
            is_read: false,
            addr: 0x2000,
            size: BusAccessSize::Word,
            data: 0x5678,
        },
    ];
    let expected = parse_transactions(&[
        serde_json::json!(["n", 2]),
        serde_json::json!(["r", 4, 6, 0x1000, ".w", 0x1234]),
        serde_json::json!(["w", 4, 5, 0x2000, ".w", 0x5678]),
        serde_json::json!(["n", 2]),
    ])
    .unwrap();
    assert!(match_transactions(&recorded, &expected, true).is_ok());

    // Same direction, address, data and total clocks; the read moved two clocks earlier.
    let shifted = parse_transactions(&[
        serde_json::json!(["r", 4, 6, 0x1000, ".w", 0x1234]),
        serde_json::json!(["n", 2]),
        serde_json::json!(["w", 4, 5, 0x2000, ".w", 0x5678]),
        serde_json::json!(["n", 2]),
    ])
    .unwrap();
    assert!(
        match_transactions(&recorded, &shifted, true).is_err(),
        "The comparator accepted a shifted bus timeline with unchanged total clocks"
    );
}
