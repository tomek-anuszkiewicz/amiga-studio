//! Shared motherboard bank bounds for the supported A500 Old Gary map.

pub const MEMORY_BANK_COUNT: usize = 256;
pub const CHIP_BANK_FIRST: u8 = 0x00;
pub const CHIP_BANK_LAST: u8 = 0x07;
pub const FAST_BANK_FIRST: u8 = 0x20;
pub const FAST_BANK_LAST: u8 = 0x5F;
pub const SLOW_BANK_FIRST: u8 = 0xC0;
pub const SLOW_BANK_LAST: u8 = 0xC7;
pub const KICKSTART_BANK_FIRST: u8 = 0xF8;
pub const KICKSTART_BANK_LAST: u8 = 0xFF;
pub const KICKSTART_BASE: u32 = 0x00F8_0000;

/// First 64 KB bank qualified for CIA selection (A23..A21 = 101).
pub const CIA_BANK_FIRST: u8 = 0xA0;
/// Last bank qualified for CIA selection.
pub const CIA_BANK_LAST: u8 = 0xBF;
/// First bank routed to the custom register window.
pub const CUSTOM_BANK_FIRST: u8 = 0xDE;
/// Last bank routed to the custom register window.
pub const CUSTOM_BANK_LAST: u8 = 0xDF;
