//! Denise Video Frame Builder & Raster Compositor
//!
//! Assembles scanline pixel data, applies display window clipping,
//! and generates 32-bit ARGB frame buffers for host display frontends.

use config::BeamPosition;
use serde::{Deserialize, Serialize};

/// Maximum overscan width in high-resolution pixels
pub const MAX_FRAME_WIDTH: usize = 720;
/// Maximum overscan height in PAL scanlines
pub const MAX_FRAME_HEIGHT: usize = 576;
/// Total pixel count of the uncompressed frame buffer
pub const FRAME_BUFFER_PIXELS: usize = MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT;

/// Converts a 12-bit Amiga RGB444 color to a 32-bit host ARGB color (0xAARRGGBB)
#[inline(always)]
pub fn rgb444_to_argb32(rgb: u16) -> u32 {
    let r = ((rgb >> 8) & 0xF) as u32;
    let g = ((rgb >> 4) & 0xF) as u32;
    let b = (rgb & 0xF) as u32;
    let r8 = (r << 4) | r;
    let g8 = (g << 4) | g;
    let b8 = (b << 4) | b;
    0xFF00_0000 | (r8 << 16) | (g8 << 8) | b8
}

/// Checks whether a given raster coordinate is within the active Display Window (DIW)
#[inline]
pub fn is_in_display_window(hcoord: u16, vcoord: u16, diwstrt: u16, diwstop: u16) -> bool {
    let vstart = (diwstrt >> 8) & 0xFF;
    let vstop = ((diwstop >> 8) & 0xFF) | (if (diwstop & 0x8000) != 0 { 0 } else { 0x100 });

    let hstart = diwstrt & 0xFF;
    let hstop = (diwstop & 0xFF) | 0x100;

    vcoord >= vstart && vcoord < vstop && hcoord >= hstart && hcoord < hstop
}

/// Raster video frame builder and display buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameBuilder {
    /// Active display width in pixels
    pub width: u32,
    /// Active display height in scanlines
    pub height: u32,
    /// Current vertical scanline counter
    pub vpos: u16,
    /// Current horizontal Color Clock position
    pub hpos: u16,
    /// Bitplane DMA channel enabled via DMACON (BPLEN bit 8 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// True when a complete video frame has been rasterized (VBlank reached)
    pub frame_ready: bool,
    /// Full 32-bit ARGB pixel buffer (0xAARRGGBB)
    #[serde(default = "default_frame_buffer", skip)]
    buffer: Vec<u32>,
}

#[inline]
fn default_frame_buffer() -> Vec<u32> {
    vec![0xFF000000; FRAME_BUFFER_PIXELS]
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for FrameBuilder {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.vpos == other.vpos
            && self.hpos == other.hpos
            && self.frame_ready == other.frame_ready
    }
}

impl Eq for FrameBuilder {}

impl FrameBuilder {
    /// Creates a new frame builder with a pre-allocated fixed frame buffer
    pub fn new() -> Self {
        Self {
            width: MAX_FRAME_WIDTH as u32,
            height: MAX_FRAME_HEIGHT as u32,
            vpos: 0,
            hpos: 0,
            dma_enabled: false,
            frame_ready: false,
            buffer: vec![0xFF000000; FRAME_BUFFER_PIXELS],
        }
    }

    /// Resets frame builder state and clears the frame buffer to solid black
    pub fn reset(&mut self) {
        self.vpos = 0;
        self.hpos = 0;
        self.dma_enabled = false;
        self.frame_ready = false;
        self.buffer.fill(0xFF000000);
    }

    /// Sets Bitplane DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
    }

    /// Advances frame builder by 1 Color Clock observing current beam coordinates
    #[inline]
    pub fn step_cck(&mut self, beam: BeamPosition) {
        self.hpos = beam.hpos;
        self.vpos = beam.vpos;
        if beam.hpos == 0 && beam.vpos == 0 {
            self.frame_ready = true;
        }
    }

    /// Sets an individual pixel color (0xAARRGGBB) with boundary checking
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, argb: u32) {
        if x < MAX_FRAME_WIDTH && y < MAX_FRAME_HEIGHT {
            let idx = y * MAX_FRAME_WIDTH + x;
            self.buffer[idx] = argb;
        }
    }

    /// Retrieves an individual pixel color (0xAARRGGBB) with boundary checking
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < MAX_FRAME_WIDTH && y < MAX_FRAME_HEIGHT {
            let idx = y * MAX_FRAME_WIDTH + x;
            self.buffer[idx]
        } else {
            0xFF000000
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

    /// Marks the start of a new video frame
    #[inline]
    pub fn begin_frame(&mut self) {
        self.frame_ready = false;
        self.vpos = 0;
        self.hpos = 0;
    }

    /// Signals VBlank and marks the current frame as ready for presentation
    #[inline]
    pub fn end_frame(&mut self) {
        self.frame_ready = true;
    }
}
