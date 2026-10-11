#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::transactions::{
    match_transactions, parse_transactions, BusAccessSize, RecordedTransaction,
};

#[test]
fn test_tas_interval_checks_both_transfer_phases() {
    // TAS.json #0: one ten-clock interval contains a byte read and byte write.
    let expected = parse_transactions(
        &[
            serde_json::json!(["t", 10, 5, 0x2b5781, ".b", 0xb5]),
            serde_json::json!(["r", 4, 6, 0xc04, ".w", 0x024e]),
        ],
        &[[0x2b5781, 0x35]],
    )
    .unwrap();
    let recorded = [
        RecordedTransaction {
            clock: 2,
            is_read: true,
            addr: 0x2b5781,
            size: BusAccessSize::Byte,
            data: 0x35,
        },
        RecordedTransaction {
            clock: 10,
            is_read: false,
            addr: 0x2b5781,
            size: BusAccessSize::Byte,
            data: 0xb5,
        },
        RecordedTransaction {
            clock: 12,
            is_read: true,
            addr: 0xc04,
            size: BusAccessSize::Word,
            data: 0x024e,
        },
    ];
    assert!(match_transactions(&recorded, &expected).is_ok());
    for i in 0..recorded.len() {
        let mut shifted = recorded.clone();
        shifted[i].clock += 2;
        assert!(match_transactions(&shifted, &expected).is_err());
        let mut wrong_data = recorded.clone();
        wrong_data[i].data ^= 0x80;
        assert!(match_transactions(&wrong_data, &expected).is_err());
    }
    assert!(match_transactions(&recorded[..2], &expected).is_err());
}

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
    let expected = parse_transactions(
        &[
            serde_json::json!(["n", 2]),
            serde_json::json!(["r", 4, 6, 0x1000, ".w", 0x1234]),
            serde_json::json!(["w", 4, 5, 0x2000, ".w", 0x5678]),
            serde_json::json!(["n", 2]),
        ],
        &[],
    )
    .unwrap();
    assert!(match_transactions(&recorded, &expected).is_ok());

    // Same direction, address, data and total clocks; the read moved two clocks earlier.
    let shifted = parse_transactions(
        &[
            serde_json::json!(["r", 4, 6, 0x1000, ".w", 0x1234]),
            serde_json::json!(["n", 2]),
            serde_json::json!(["w", 4, 5, 0x2000, ".w", 0x5678]),
            serde_json::json!(["n", 2]),
        ],
        &[],
    )
    .unwrap();
    assert!(
        match_transactions(&recorded, &shifted).is_err(),
        "The comparator accepted a shifted bus timeline with unchanged total clocks"
    );
}
