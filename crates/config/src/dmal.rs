//! Positions on the serialized Paula request frame; no addresses cross DMAL.

pub const FRAME_BITS: u16 = 16;
pub const FIRST_DISK_BIT: u16 = 2;
pub const FIRST_DISK_TRANSFER: u16 = 7;
pub const DISK_SLOT_COUNT: usize = 3;
pub const FIRST_AUDIO_BIT: u16 = 8;
pub const PAIR_BITS: u16 = 2;
pub const REQUEST_BIT: u16 = 1;
/// Line strobes are driven at HPOS 1 and received at HPOS 2 in the CCK model.
pub const HORIZONTAL_CAPTURE_POSITION: u16 = 2;
