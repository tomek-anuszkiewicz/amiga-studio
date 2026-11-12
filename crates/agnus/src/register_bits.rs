//! Internal Agnus register fields and display-fetch parameters.

pub mod blitter_fields {
    pub const BIT_SHIFT: u16 = 12;
    pub const MASK_SHIFT_VALUE: u16 = 0xF;
    pub const MASK_CONTROL_PRESERVE: u16 = 0x0FFF;
    pub const MASK_FILL: u16 = 0x0018;
    pub const BIT_FILL_CARRY_IN: u16 = 0x0004;
    pub const BIT_LINE_SINGLE_DOT: u16 = 0x0002;
    pub const BIT_LINE_SUD: u16 = 0x0010;
    pub const BIT_LINE_SUL: u16 = 0x0008;
    pub const BIT_LINE_AUL: u16 = 0x0004;
    pub const BIT_LINE_SIGN: u16 = 0x0040;
}

pub mod bltcon {
    /// BLTCON0 Bit 11: Channel A DMA enable
    pub const BIT_USEA: u16 = 0x0800;
    /// BLTCON0 Bit 10: Channel B DMA enable
    pub const BIT_USEB: u16 = 0x0400;
    /// BLTCON0 Bit 9: Channel C DMA enable
    pub const BIT_USEC: u16 = 0x0200;
    /// BLTCON0 Bit 8: Channel D DMA enable
    pub const BIT_USED: u16 = 0x0100;
    /// BLTCON1 Bit 0: Line mode enable (0 = area fill/copy, 1 = line draw)
    pub const BIT_LINE: u16 = 0x0001;
    /// BLTCON1 Bit 1: Descending addressing mode (decrement pointers)
    pub const BIT_DESC: u16 = 0x0002;
    /// BLTCON1 Bit 4: Exclusive-OR fill mode
    pub const BIT_EFE: u16 = 0x0010;
}

pub mod copcon {
    /// Bit 1: Copper Danger bit (allows Copper to write registers below $080, e.g. Blitter)
    pub const BIT_CDANG: u16 = 0x0002;
}

pub mod vposr {
    /// Bit 15: Long Frame toggle bit (0 = short field, 1 = long field in interlace)
    pub const BIT_LOF: u16 = 0x8000;
    /// OCS NTSC 8370 chip ID value
    pub const CHIP_ID_NTSC_8370: u16 = 0x1000;
    /// Bits 2..0: Vertical position high bits (V10..V8)
    pub const MASK_VPOS_HI: u16 = 0x0007;
}

pub mod display_fetch {
    pub const HIRES_FETCH_PERIOD_CCK: u16 = 4;
    pub const LORES_FETCH_PERIOD_CCK: u16 = 8;
    pub const MASK_DDF_POSITION: u16 = 0x00FC;
}
