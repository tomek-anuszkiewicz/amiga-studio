//! Video frame storage and field assembly for Denise's completed pixels.
//!
//! Stores 32-bit ARGB pixels, combines interlaced fields and exposes the buffer
//! to host display frontends. Denise owns pixel composition and display gating.

use serde::{Deserialize, Serialize};

pub const MASK_RGB444_COMPONENT: u16 = 0xF;
pub const BIT_RGB444_RED_SHIFT: u16 = 8;
pub const BIT_RGB444_GREEN_SHIFT: u16 = 4;
pub const MASK_RGB444_COLOR: u16 = 0x0FFF;
const RGB444_COMPONENT_SCALE: u32 = 17;
const OPAQUE_ALPHA: u32 = 0xFF00_0000;
pub const OPAQUE_BLACK: u32 = OPAQUE_ALPHA;
pub const PIXELS_PER_CCK: usize = 4;
pub const BIT_HSTOP_EXTENSION: u16 = 0x100;

/// Maximum overscan width in high-resolution pixels (4 pixels per CCK across 228 CCKs)
pub const MAX_FRAME_WIDTH: usize = 912;
/// Maximum overscan height in PAL scanlines
pub const PAL_FIELD_HEIGHT: usize = 313;
pub const NTSC_FIELD_HEIGHT: usize = 263;
pub const MAX_FRAME_HEIGHT: usize = PAL_FIELD_HEIGHT * 2;
/// Total pixel count of the uncompressed frame buffer
pub const FRAME_BUFFER_PIXELS: usize = MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT;

/// Converts a 12-bit Amiga RGB444 color to a 32-bit host ARGB color (0xAARRGGBB).
///
/// Expands each channel from 0..15 to 0..255 by multiplying by 17.
/// Preserves channel order, ignores the upper four input bits and sets opaque alpha.
/// This is full-range host normalization, without gamma or CRT correction.
#[inline(always)]
pub fn rgb444_to_argb32(rgb: u16) -> u32 {
    let r = ((rgb >> BIT_RGB444_RED_SHIFT) & MASK_RGB444_COMPONENT) as u32;
    let g = ((rgb >> BIT_RGB444_GREEN_SHIFT) & MASK_RGB444_COMPONENT) as u32;
    let b = (rgb & MASK_RGB444_COMPONENT) as u32;
    let r8 = r * RGB444_COMPONENT_SCALE;
    let g8 = g * RGB444_COMPONENT_SCALE;
    let b8 = b * RGB444_COMPONENT_SCALE;
    OPAQUE_ALPHA | (r8 << 16) | (g8 << 8) | b8
}

/// Checks whether a given raster coordinate is within the active Display Window (DIW)
#[inline]
pub fn is_in_display_window(hcoord: u16, vcoord: u16, diwstrt: u16, diwstop: u16) -> bool {
    let vstart = (diwstrt >> 8) & 0xFF;
    let vstop = ((diwstop >> 8) & 0xFF)
        | (if (diwstop & config::display_window::BIT_VSTOP_EXTENSION_SOURCE) != 0 {
            0
        } else {
            config::display_window::BIT_VSTOP_EXTENSION
        });

    let hstart = diwstrt & 0xFF;
    let hstop = (diwstop & 0xFF) | BIT_HSTOP_EXTENSION;

    vcoord >= vstart && vcoord < vstop && hcoord >= hstart && hcoord < hstop
}

/// Raster video frame builder and display buffer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuilder {
    /// Active display width in pixels
    pub width: u32,
    /// Active display height in scanlines
    pub height: u32,
    /// Current vertical scanline counter
    pub vpos: u16,
    /// Current horizontal Color Clock position
    pub hpos: u16,
    /// True when a complete video frame has been rasterized (VBlank reached)
    pub frame_ready: bool,
    /// Full 32-bit ARGB pixel buffer (0xAARRGGBB)
    buffer: Vec<u32>,
    pub frame_number: u64,
    pub interlaced: bool,
    pub field_odd: bool,
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder {
    /// Creates a new frame builder with a pre-allocated fixed frame buffer
    pub fn new() -> Self {
        Self {
            width: MAX_FRAME_WIDTH as u32,
            height: PAL_FIELD_HEIGHT as u32,
            vpos: 0,
            hpos: 0,
            frame_ready: false,
            buffer: vec![OPAQUE_BLACK; FRAME_BUFFER_PIXELS],
            frame_number: 0,
            interlaced: false,
            field_odd: false,
        }
    }

    /// Resets frame builder state and clears the frame buffer to solid black
    pub fn reset(&mut self) {
        self.vpos = 0;
        self.hpos = 0;
        self.frame_ready = false;
        self.buffer.fill(OPAQUE_BLACK);
        self.frame_number = 0;
        self.interlaced = false;
        self.field_odd = false;
        self.height = PAL_FIELD_HEIGHT as u32;
    }

    /// Advances frame builder by 1 Color Clock observing current beam coordinates
    #[inline]
    pub fn step_cck(&mut self, hpos: u16, vpos: u16) {
        self.hpos = hpos;
        self.vpos = vpos;
    }

    /// Sets an individual pixel color (0xAARRGGBB) with boundary checking
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, argb: u32) {
        let row = if self.interlaced {
            y * 2 + usize::from(self.field_odd)
        } else {
            y
        };
        if x < MAX_FRAME_WIDTH && row < MAX_FRAME_HEIGHT {
            self.buffer[row * MAX_FRAME_WIDTH + x] = argb;
        }
    }

    /// Retrieves an individual pixel color (0xAARRGGBB) with boundary checking
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < MAX_FRAME_WIDTH && y < MAX_FRAME_HEIGHT {
            let idx = y * MAX_FRAME_WIDTH + x;
            self.buffer[idx]
        } else {
            OPAQUE_BLACK
        }
    }

    /// Returns a slice of the 32-bit ARGB frame buffer
    #[inline]
    pub fn frame_buffer(&self) -> &[u32] {
        &self.buffer
    }

    /// Returns a mutable slice of the 32-bit ARGB frame buffer
    #[inline]
    pub fn frame_buffer_mut(&mut self) -> &mut [u32] {
        &mut self.buffer
    }

    /// Receiver-derived field geometry; the opposite interlaced field is retained.
    pub fn begin_field(&mut self, interlaced: bool, odd: bool, lines: u16, completed: bool) {
        if interlaced != self.interlaced {
            self.buffer.fill(OPAQUE_BLACK);
        }
        self.interlaced = interlaced;
        self.field_odd = odd;
        let height = if usize::from(lines) <= NTSC_FIELD_HEIGHT {
            NTSC_FIELD_HEIGHT
        } else {
            PAL_FIELD_HEIGHT
        };
        self.height = (height * if interlaced { 2 } else { 1 }) as u32;
        if completed {
            self.frame_number = self.frame_number.wrapping_add(1);
            self.frame_ready = true;
        }
    }
}
