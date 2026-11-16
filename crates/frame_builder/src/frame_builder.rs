//! Video frame storage and field assembly for Denise's completed pixels.
//!
//! Stores 32-bit ARGB pixels, combines interlaced fields and exposes the buffer
//! to host display frontends. Denise owns pixel composition and display gating.

use config::VideoStandard;
use serde::{Deserialize, Serialize};

pub const MASK_RGB444_COMPONENT: u16 = 0xF;
pub const BIT_RGB444_RED_SHIFT: u16 = 8;
pub const BIT_RGB444_GREEN_SHIFT: u16 = 4;
pub const MASK_RGB444_COLOR: u16 = 0x0FFF;
const RGB444_COMPONENT_SCALE: u32 = 17;
const OPAQUE_ALPHA: u32 = 0xFF00_0000;
pub const OPAQUE_BLACK: u32 = OPAQUE_ALPHA;
pub const HIRES_PIXELS_PER_CCK: usize = 4;
pub const LORES_PIXELS_PER_CCK: usize = 2;
pub const ROWS_PER_FIELD_LINE: usize = 2;
/// Retained receiver blanking phase; counters continue through this interval.
pub const HBLANK_START_CCK: u16 = 18;
pub const HBLANK_STOP_CCK: u16 = 35;
/// Initial receiver timing before a complete field has been measured.
pub const PAL_MAX_RASTER_LINES: usize = 313;
/// Fixed presentation width on the high-resolution sample grid.
pub const MAX_FRAME_WIDTH: usize = 720;
pub const MAX_FRAME_HEIGHT: usize = 568;
const NTSC_FRAME_HEIGHT: usize = 480;
/// Receiver output phase at the first stored sample, independent of DIW.
pub const FRAME_START_CCK: u16 = 49;
const PAL_FRAME_START_LINE: u16 = 28;
const NTSC_FRAME_START_LINE: u16 = 21;
const FRAME_BUFFER_CAPACITY: usize = MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT;
pub const BIT_HSTOP_EXTENSION: u16 = 0x100;

const fn expand_rgb444(rgb: u16) -> u32 {
    let r = ((rgb >> BIT_RGB444_RED_SHIFT) & MASK_RGB444_COMPONENT) as u32;
    let g = ((rgb >> BIT_RGB444_GREEN_SHIFT) & MASK_RGB444_COMPONENT) as u32;
    let b = (rgb & MASK_RGB444_COMPONENT) as u32;
    let r8 = r * RGB444_COMPONENT_SCALE;
    let g8 = g * RGB444_COMPONENT_SCALE;
    let b8 = b * RGB444_COMPONENT_SCALE;
    OPAQUE_ALPHA | (r8 << 16) | (g8 << 8) | b8
}

const RGB444_COLOR_COUNT: usize = MASK_RGB444_COLOR as usize + 1;
static RGB444_TO_ARGB32: [u32; RGB444_COLOR_COUNT] = build_color_table();
const fn build_color_table() -> [u32; RGB444_COLOR_COUNT] {
    let mut table = [0; RGB444_COLOR_COUNT];
    let mut rgb = 0;
    while rgb < RGB444_COLOR_COUNT {
        table[rgb] = expand_rgb444(rgb as u16);
        rgb += 1;
    }
    table
}

/// Converts RGB444 to opaque ARGB through the full-range host color table.
/// Each channel expands by 17; upper input bits are ignored, without gamma correction.
#[inline(always)]
pub fn rgb444_to_argb32(rgb: u16) -> u32 {
    RGB444_TO_ARGB32[usize::from(rgb & MASK_RGB444_COLOR)]
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

/// Complete visible image on a fixed high-resolution, interlaced spatial grid.
/// Spare capacity is reserved once; blanking samples are never stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuilder {
    pub width: u32,
    pub height: u32,
    pub frame_ready: bool,
    pub frame_number: u64,
    pub interlaced: bool,
    pub field_odd: bool,
    buffer: Vec<u32>,
    standard: VideoStandard,
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            width: MAX_FRAME_WIDTH as u32,
            height: MAX_FRAME_HEIGHT as u32,
            frame_ready: false,
            frame_number: 0,
            interlaced: false,
            field_odd: false,
            buffer: Self::new_buffer(),
            standard: VideoStandard::Pal,
        }
    }

    /// Allocates one reusable host image buffer at the fixed storage capacity.
    pub fn new_buffer() -> Vec<u32> {
        vec![OPAQUE_BLACK; FRAME_BUFFER_CAPACITY]
    }

    /// Full allocation length, including rows outside the current visible image.
    #[inline]
    pub fn storage_len(&self) -> usize {
        self.buffer.len()
    }

    /// Hands the current allocation to the caller and continues in its replacement.
    ///
    /// Seed the replacement before exchanging ownership so opposite-field and
    /// unfinished-field pixels remain current. The caller must exclusively own
    /// the replacement. An incorrect length leaves both buffers unchanged.
    pub fn exchange_buffer(&mut self, replacement: &mut Vec<u32>) -> bool {
        if replacement.len() != FRAME_BUFFER_CAPACITY {
            return false;
        }
        replacement.copy_from_slice(&self.buffer);
        std::mem::swap(&mut self.buffer, replacement);
        true
    }

    pub fn reset(&mut self) {
        self.buffer.fill(OPAQUE_BLACK);
        self.width = MAX_FRAME_WIDTH as u32;
        self.height = self.standard_height() as u32;
        self.frame_ready = false;
        self.frame_number = 0;
        self.interlaced = false;
        self.field_odd = false;
    }

    /// Selects the machine's presentation profile before receiver execution.
    pub fn set_video_standard(&mut self, standard: VideoStandard) {
        if self.standard != standard {
            self.standard = standard;
            self.reset();
        }
    }

    fn standard_height(&self) -> usize {
        match self.standard {
            VideoStandard::Pal => MAX_FRAME_HEIGHT,
            VideoStandard::Ntsc => NTSC_FRAME_HEIGHT,
        }
    }

    /// First stored receiver line; field parity selects alternate image rows.
    pub fn first_line(&self) -> u16 {
        match self.standard {
            VideoStandard::Pal => PAL_FRAME_START_LINE,
            VideoStandard::Ntsc => NTSC_FRAME_START_LINE,
        }
    }

    /// Publishes fixed geometry and clears rows absent from the completed field.
    pub fn finish_field(&mut self, complete_lines: usize) {
        for line in complete_lines..self.height as usize / ROWS_PER_FIELD_LINE {
            let row = line * ROWS_PER_FIELD_LINE + usize::from(self.interlaced && self.field_odd);
            self.clear_row_tail(row, 0);
        }
        self.frame_number = self.frame_number.wrapping_add(1);
        self.frame_ready = true;
    }

    /// Applies the incoming field layout after the preceding row has finished.
    pub fn set_field_mode(&mut self, interlaced: bool, odd: bool) {
        if interlaced != self.interlaced {
            self.buffer.fill(OPAQUE_BLACK);
        }
        self.interlaced = interlaced;
        self.field_odd = odd;
    }

    /// Stores one color clock of hires samples at their final visible coordinates.
    /// Partial edge pairs retain the same clipping as individual pair writes.
    #[inline]
    pub fn write_cck(
        &mut self,
        frame_x: usize,
        frame_y: usize,
        pixels: [u32; HIRES_PIXELS_PER_CCK],
    ) {
        if frame_x + HIRES_PIXELS_PER_CCK <= self.width as usize && frame_y < self.height as usize {
            let start = frame_y * MAX_FRAME_WIDTH + frame_x;
            self.buffer[start..start + HIRES_PIXELS_PER_CCK].copy_from_slice(&pixels);
            if !self.interlaced && frame_y + 1 < self.height as usize {
                let start = start + MAX_FRAME_WIDTH;
                self.buffer[start..start + HIRES_PIXELS_PER_CCK].copy_from_slice(&pixels);
            }
        } else {
            self.write_pixel_pair(frame_x, frame_y, [pixels[0], pixels[1]]);
            self.write_pixel_pair(frame_x + 2, frame_y, [pixels[2], pixels[3]]);
        }
    }

    /// Stores two hires samples at their final visible coordinates.
    #[inline]
    pub fn write_pixel_pair(&mut self, frame_x: usize, frame_y: usize, pixels: [u32; 2]) {
        if frame_x < self.width as usize - 1 && frame_y < self.height as usize {
            let start = frame_y * MAX_FRAME_WIDTH + frame_x;
            self.buffer[start..start + 2].copy_from_slice(&pixels);
            if !self.interlaced && frame_y + 1 < self.height as usize {
                let start = start + MAX_FRAME_WIDTH;
                self.buffer[start..start + 2].copy_from_slice(&pixels);
            }
        }
    }

    pub fn clear_row_tail(&mut self, row: usize, first: usize) {
        let width = self.width as usize;
        if row < self.height as usize && first < width {
            self.buffer[row * MAX_FRAME_WIDTH + first..row * MAX_FRAME_WIDTH + width]
                .fill(OPAQUE_BLACK);
            if !self.interlaced && row + 1 < self.height as usize {
                self.buffer
                    [(row + 1) * MAX_FRAME_WIDTH + first..(row + 1) * MAX_FRAME_WIDTH + width]
                    .fill(OPAQUE_BLACK);
            }
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width as usize && y < self.height as usize {
            self.buffer[y * MAX_FRAME_WIDTH + x]
        } else {
            OPAQUE_BLACK
        }
    }

    /// Fixed storage stride, equal to the published image width.
    #[inline]
    pub fn row_stride(&self) -> usize {
        MAX_FRAME_WIDTH
    }

    /// Returns exactly the published rectangle, without row padding.
    pub fn frame_buffer(&self) -> &[u32] {
        &self.buffer[..MAX_FRAME_WIDTH * self.height as usize]
    }

    /// Mutable storage with the same row stride and visible bounds as frame_buffer.
    pub fn frame_buffer_mut(&mut self) -> &mut [u32] {
        let len = MAX_FRAME_WIDTH * self.height as usize;
        &mut self.buffer[..len]
    }
}
