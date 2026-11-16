//! Visible framebuffer coordinates derived from Denise's receiver timing.

use crate::Denise;
use frame_builder::{FRAME_START_CCK, HBLANK_START_CCK, HIRES_PIXELS_PER_CCK, ROWS_PER_FIELD_LINE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PendingLine {
    interlaced: bool,
    odd: bool,
    completed: bool,
    blank: bool,
    ccks: usize,
}

/// Output position on the fixed hires grid, independent of hardware counter wrap.
/// X advances by two per pair; Y selects the first row of a progressive pair
/// or the current interlaced parity. Blanking and partial prefixes are inactive.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameCursor {
    pub frame_x: usize,
    pub frame_y: Option<usize>,
    pub active: bool,
    last_hpos: u16,
    complete_lines: usize,
    pending_line: Option<PendingLine>,
}

impl Denise {
    pub(super) fn begin_frame_line(&mut self, interlaced: bool, completed: bool) {
        let ccks = usize::from(self.frame_cursor.last_hpos) + 1;
        let cursor = &mut self.frame_cursor;
        cursor.frame_x = ccks.saturating_sub(usize::from(FRAME_START_CCK)) * HIRES_PIXELS_PER_CCK;
        cursor.pending_line = Some(PendingLine {
            interlaced,
            odd: self.sync.field_odd,
            completed,
            blank: self.sync.vertical_blank,
            ccks,
        });
        cursor.active = !self.sync.vertical_blank
            && ccks >= usize::from(FRAME_START_CCK)
            && cursor.frame_y.is_some();
    }

    pub(super) fn advance_frame_cursor(&mut self, horizontal: u16, vertical: u16) {
        let cursor = &mut self.frame_cursor;
        if horizontal >= HBLANK_START_CCK {
            if let Some(line) = cursor.pending_line.take() {
                if let Some(row) = cursor.frame_y {
                    cursor.complete_lines =
                        cursor.complete_lines.max(row / ROWS_PER_FIELD_LINE + 1);
                    self.frame_builder.clear_row_tail(
                        row,
                        if line.blank {
                            cursor.frame_x
                        } else {
                            (line.ccks + usize::from(horizontal))
                                .saturating_sub(usize::from(FRAME_START_CCK))
                                * HIRES_PIXELS_PER_CCK
                        },
                    );
                }
                if line.completed {
                    self.frame_builder.finish_field(cursor.complete_lines);
                }
                if vertical == 0 {
                    cursor.complete_lines = 0;
                }
                self.frame_builder.set_field_mode(line.interlaced, line.odd);
                cursor.frame_y = vertical
                    .checked_sub(self.frame_builder.first_line())
                    .and_then(|index| {
                        let row = usize::from(index) * ROWS_PER_FIELD_LINE
                            + usize::from(line.interlaced && line.odd);
                        (row < self.frame_builder.height as usize).then_some(row)
                    });
                // A blank line keeps its fixed row position; remove old pixels
                // only after the preceding row and field have finished.
                if line.blank {
                    if let Some(row) = cursor.frame_y.take() {
                        self.frame_builder.clear_row_tail(row, 0);
                    }
                }
                // A restored or explicitly positioned receiver can enter after HBLANK.
                cursor.frame_x =
                    usize::from(horizontal.saturating_sub(FRAME_START_CCK)) * HIRES_PIXELS_PER_CCK;
            }
            if horizontal == FRAME_START_CCK {
                cursor.frame_x = 0;
            }
            cursor.active = horizontal >= FRAME_START_CCK && cursor.frame_y.is_some();
        } else {
            // A counter wrap without a line strobe has no preceding row prefix.
            cursor.active = cursor
                .pending_line
                .is_some_and(|line| !line.blank && line.ccks >= usize::from(FRAME_START_CCK))
                && cursor.frame_y.is_some();
        }
        cursor.active &= !self.sync.vertical_blank;
        cursor.last_hpos = horizontal;
    }
}
