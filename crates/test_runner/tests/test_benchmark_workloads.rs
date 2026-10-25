#![allow(dead_code)]

#[path = "../benches/bench_instructions/inventory.rs"]
mod inventory;

#[test]
fn decoder_inventory_separates_legal_encodings_from_decode_exceptions() {
    let counts = inventory::decoder_inventory();
    assert_eq!(counts.legal_encodings + counts.decode_exceptions, 65_536);
    assert!(counts.legal_encodings > 0);
    assert!(!std::ptr::eq(
        cpu::OPCODE_DESCRIPTOR_TABLE[0x4E71].steps,
        &cpu::micro::STEPS_DECODE_EXCEPTION[..]
    ));
    assert!(std::ptr::eq(
        cpu::OPCODE_DESCRIPTOR_TABLE[0xFFFF].steps,
        &cpu::micro::STEPS_DECODE_EXCEPTION[..]
    ));
}
