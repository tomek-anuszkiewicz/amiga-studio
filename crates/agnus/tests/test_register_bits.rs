// Include the private definitions to retain direct hardware-encoding checks.
#[path = "../src/register_bits.rs"]
#[allow(dead_code)]
mod register_bits;

use register_bits::copcon;

#[test]
fn test_copcon_cdang_mask() {
    assert_eq!(copcon::BIT_CDANG, 0x0002);
}
