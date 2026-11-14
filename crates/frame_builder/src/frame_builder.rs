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
pub const HIRES_PIXELS_PER_CCK: usize = 4;
pub const LORES_PIXELS_PER_CCK: usize = 2;
/// Retained receiver blanking phase; counters continue through this interval.
pub const HBLANK_START_CCK: u16 = 18;
pub const HBLANK_STOP_CCK: u16 = 35;
const HBLANK_CCKS: usize = (HBLANK_STOP_CCK - HBLANK_START_CCK + 1) as usize;
const VISIBLE_START: usize = (HBLANK_STOP_CCK as usize + 1) * LORES_PIXELS_PER_CCK;
const COUNTER_PREFIX_END: usize = HBLANK_START_CCK as usize * LORES_PIXELS_PER_CCK;
const PAL_LINE_CCKS: usize = 227;
const MAX_LINE_CCKS: usize = 228;
/// Initial receiver timing before a complete field has been measured.
pub const PAL_MAX_RASTER_LINES: usize = 313;
/// Maximum complete visible lines per field in the retained OCS receiver model.
pub const MAX_VISIBLE_FIELD_LINES: usize = 286;
/// Maximum visible width, including the border, at high resolution.
pub const MAX_FRAME_WIDTH: usize = (MAX_LINE_CCKS - HBLANK_CCKS) * HIRES_PIXELS_PER_CCK;
pub const MAX_FRAME_HEIGHT: usize = MAX_VISIBLE_FIELD_LINES * 2;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PreviousLine {
    row_start: usize,
    ccks: usize,
    prefix_column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PendingField {
    interlaced: bool,
    odd: bool,
    completed: bool,
    blank: bool,
}

/// Complete visible image on a fixed high-resolution, interlaced spatial grid.
/// Spare capacity is reserved once; blanking samples are never stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuilder {
    pub width: u32,
    pub height: u32,
    pub hpos: u16,
    pub vpos: u16,
    pub frame_ready: bool,
    pub frame_number: u64,
    pub interlaced: bool,
    pub field_odd: bool,
    buffer: Vec<u32>,
    line_ccks: usize,
    first_visible_line: Option<u16>,
    active_row_start: Option<usize>,
    previous_line: Option<PreviousLine>,
    pending_field: Option<PendingField>,
    complete_lines: usize,
    field_lines: [usize; 2],
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            width: ((PAL_LINE_CCKS - HBLANK_CCKS) * HIRES_PIXELS_PER_CCK) as u32,
            height: MAX_FRAME_HEIGHT as u32,
            hpos: 0,
            vpos: 0,
            frame_ready: false,
            frame_number: 0,
            interlaced: false,
            field_odd: false,
            buffer: vec![OPAQUE_BLACK; FRAME_BUFFER_CAPACITY],
            line_ccks: PAL_LINE_CCKS,
            first_visible_line: None,
            active_row_start: None,
            previous_line: None,
            pending_field: None,
            complete_lines: 0,
            field_lines: [0; 2],
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(OPAQUE_BLACK);
        self.width = ((PAL_LINE_CCKS - HBLANK_CCKS) * HIRES_PIXELS_PER_CCK) as u32;
        self.height = MAX_FRAME_HEIGHT as u32;
        self.hpos = 0;
        self.vpos = 0;
        self.frame_ready = false;
        self.frame_number = 0;
        self.interlaced = false;
        self.field_odd = false;
        self.line_ccks = PAL_LINE_CCKS;
        self.first_visible_line = None;
        self.active_row_start = None;
        self.previous_line = None;
        self.pending_field = None;
        self.complete_lines = 0;
        self.field_lines = [0; 2];
    }

    /// Receives a line strobe. The counter prefix still belongs to the old row.
    pub fn begin_line(&mut self, interlaced: bool, odd: bool, completed: bool, blank: bool) {
        let ccks = usize::from(self.hpos) + 1;
        if ccks > self.line_ccks && ccks <= MAX_LINE_CCKS {
            self.line_ccks = ccks;
            self.width = ((ccks - HBLANK_CCKS) * HIRES_PIXELS_PER_CCK) as u32;
        }
        self.previous_line = if blank {
            None
        } else {
            self.active_row_start.map(|row_start| PreviousLine {
                row_start,
                ccks,
                prefix_column: (ccks * LORES_PIXELS_PER_CCK).checked_sub(VISIBLE_START),
            })
        };
        self.pending_field = Some(PendingField {
            interlaced,
            odd,
            completed,
            blank,
        });
    }

    pub fn step_cck(&mut self, hpos: u16, vpos: u16) {
        if hpos >= HBLANK_START_CCK {
            if let Some(field) = self.pending_field.take() {
                if let Some(previous) = self.previous_line.take() {
                    let row = previous.row_start / MAX_FRAME_WIDTH;
                    self.complete_lines = self.complete_lines.max(row / 2 + 1);
                    let pixels = previous.ccks.saturating_sub(HBLANK_CCKS) * HIRES_PIXELS_PER_CCK;
                    self.clear_row_tail(row, pixels);
                }
                if field.completed {
                    let parity = usize::from(self.field_odd);
                    let previous_lines = self.field_lines[parity];
                    let painted_lines = self
                        .active_row_start
                        .map_or(0, |start| start / (MAX_FRAME_WIDTH * 2) + 1);
                    // Only the unfinished row and rows retained from a longer
                    // field can contain pixels outside the new visible extent.
                    for line in self.complete_lines..previous_lines.max(painted_lines) {
                        let row = line * 2 + if self.interlaced { parity } else { 0 };
                        self.clear_row_tail(row, 0);
                    }
                    self.field_lines[parity] = self.complete_lines;
                    let rows = if self.interlaced {
                        self.field_lines[0]
                            .saturating_mul(2)
                            .saturating_sub(1)
                            .max(self.field_lines[1] * 2)
                    } else {
                        self.complete_lines * 2
                    };
                    self.height = rows.clamp(1, MAX_FRAME_HEIGHT) as u32;
                    self.frame_number = self.frame_number.wrapping_add(1);
                    self.frame_ready = true;
                }
                if vpos == 0 {
                    self.first_visible_line = None;
                    self.complete_lines = 0;
                }
                if field.interlaced != self.interlaced {
                    self.buffer.fill(OPAQUE_BLACK);
                    self.field_lines = [0; 2];
                    self.height = MAX_FRAME_HEIGHT as u32;
                }
                self.interlaced = field.interlaced;
                self.field_odd = field.odd;
                self.active_row_start = if field.blank {
                    None
                } else {
                    let first = *self.first_visible_line.get_or_insert(vpos);
                    let line = usize::from(vpos.saturating_sub(first));
                    if line < MAX_VISIBLE_FIELD_LINES {
                        let row = line * 2 + usize::from(self.interlaced && self.field_odd);
                        Some(row * MAX_FRAME_WIDTH)
                    } else {
                        None
                    }
                };
            }
        }
        self.hpos = hpos;
        self.vpos = vpos;
    }

    /// Stores one low-resolution interval, with both high-resolution samples.
    pub fn write_pixel_pair(&mut self, x: usize, pixels: [u32; 2]) {
        let (column, row_start) = if x >= VISIBLE_START {
            let Some(row_start) = self.active_row_start else {
                return;
            };
            (x - VISIBLE_START, row_start)
        } else if x < COUNTER_PREFIX_END {
            let Some(previous) = self.previous_line else {
                return;
            };
            let Some(tail) = previous.prefix_column else {
                return;
            };
            (tail + x, previous.row_start)
        } else {
            return;
        };
        let column = column * 2;
        let width = self.width as usize;
        if column + 1 < width {
            let start = row_start + column;
            self.buffer[start..start + 2].copy_from_slice(&pixels);
            if !self.interlaced {
                let start = row_start + MAX_FRAME_WIDTH + column;
                self.buffer[start..start + 2].copy_from_slice(&pixels);
            }
        }
    }

    fn clear_row_tail(&mut self, row: usize, first: usize) {
        let width = self.width as usize;
        if row < MAX_FRAME_HEIGHT && first < width {
            self.buffer[row * MAX_FRAME_WIDTH + first..row * MAX_FRAME_WIDTH + width]
                .fill(OPAQUE_BLACK);
            if !self.interlaced && row + 1 < MAX_FRAME_HEIGHT {
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

    /// Fixed storage stride; each row starts with width visible ARGB pixels.
    #[inline]
    pub fn row_stride(&self) -> usize {
        MAX_FRAME_WIDTH
    }

    /// Returns height rows at row_stride; only each row's first width pixels are visible.
    pub fn frame_buffer(&self) -> &[u32] {
        &self.buffer[..MAX_FRAME_WIDTH * self.height as usize]
    }

    /// Mutable storage with the same row stride and visible bounds as frame_buffer.
    pub fn frame_buffer_mut(&mut self) -> &mut [u32] {
        let len = MAX_FRAME_WIDTH * self.height as usize;
        &mut self.buffer[..len]
    }
}
