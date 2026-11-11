//! Shared motherboard bank bounds for the supported A500 Old Gary map.

/// First 64 KB bank qualified for CIA selection (A23..A21 = 101).
pub const CIA_BANK_FIRST: u8 = 0xA0;
/// Last bank qualified for CIA selection.
pub const CIA_BANK_LAST: u8 = 0xBF;
/// First bank routed to the custom register window.
pub const CUSTOM_BANK_FIRST: u8 = 0xDE;
/// Last bank routed to the custom register window.
pub const CUSTOM_BANK_LAST: u8 = 0xDF;
