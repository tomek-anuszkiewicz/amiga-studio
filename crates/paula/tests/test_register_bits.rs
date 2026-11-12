// Include the private definitions to retain direct hardware-encoding checks.
#[path = "../src/register_bits.rs"]
#[allow(dead_code)]
mod register_bits;

use register_bits::{dskbytr, intreq};

#[test]
fn test_intreq_masks() {
    assert_eq!(intreq::BIT_SET_CLR, 0x8000);
    assert_eq!(intreq::BIT_INTEN, 0x4000);
    assert_eq!(intreq::BIT_BLIT, 0x0040);
    assert_eq!(intreq::BIT_VERTB, 0x0020);
    assert_eq!(intreq::BIT_COPER, 0x0010);
}

#[test]
fn test_dskbytr_masks() {
    assert_eq!(dskbytr::BIT_DSKBYTR_DMAON, 0x4000);
    assert_eq!(dskbytr::BIT_DSKBYTR_DISKWRITE, 0x2000);
    assert_eq!(dskbytr::MASK_DSKBYTR_DATA, 0x90FF);
}
