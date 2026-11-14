//! Visible framebuffer coordinates derived from Denise's receiver timing.

use crate::Denise;
use frame_builder::{
    HBLANK_START_CCK, HBLANK_STOP_CCK, HIRES_PIXELS_PER_CCK, MAX_VISIBLE_FIELD_LINES,
};
use serde::{Deserialize, Serialize};

const VISIBLE_START_CCK: u16 = HBLANK_STOP_CCK + 1;
const HBLANK_CCKS: usize = (VISIBLE_START_CCK - HBLANK_START_CCK) as usize;

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
    first_visible_line: Option<u16>,
    complete_lines: usize,
    pending_line: Option<PendingLine>,
}

impl Denise {
    pub(super) fn begin_frame_line(&mut self, interlaced: bool, completed: bool) {
        let ccks = usize::from(self.frame_cursor.last_hpos) + 1;
        self.frame_builder.observe_line_length(ccks);
        let cursor = &mut self.frame_cursor;
        cursor.frame_x = ccks.saturating_sub(usize::from(VISIBLE_START_CCK)) * HIRES_PIXELS_PER_CCK;
        cursor.pending_line = Some(PendingLine {
            interlaced,
            odd: self.sync.field_odd,
            completed,
            blank: self.sync.vertical_blank,
            ccks,
        });
        cursor.active = !self.sync.vertical_blank
            && ccks >= usize::from(VISIBLE_START_CCK)
            && cursor.frame_y.is_some();
    }

    pub(super) fn advance_frame_cursor(&mut self, horizontal: u16, vertical: u16) {
        let cursor = &mut self.frame_cursor;
        if horizontal >= HBLANK_START_CCK {
            if let Some(line) = cursor.pending_line.take() {
                if let Some(row) = cursor.frame_y.filter(|_| !line.blank) {
                    cursor.complete_lines = cursor.complete_lines.max(row / 2 + 1);
                    self.frame_builder.clear_row_tail(
                        row,
                        line.ccks.saturating_sub(HBLANK_CCKS) * HIRES_PIXELS_PER_CCK,
                    );
                }
                if line.completed {
                    let painted_lines = cursor.frame_y.map_or(0, |row| row / 2 + 1);
                    self.frame_builder
                        .finish_field(cursor.complete_lines, painted_lines);
                }
                if vertical == 0 {
                    cursor.first_visible_line = None;
                    cursor.complete_lines = 0;
                }
                self.frame_builder.set_field_mode(line.interlaced, line.odd);
                cursor.frame_y = if line.blank {
                    None
                } else {
                    let first = *cursor.first_visible_line.get_or_insert(vertical);
                    let index = usize::from(vertical.saturating_sub(first));
                    (index < MAX_VISIBLE_FIELD_LINES)
                        .then_some(index * 2 + usize::from(line.interlaced && line.odd))
                };
                // A restored or explicitly positioned receiver can enter after HBLANK.
                cursor.frame_x = usize::from(horizontal.saturating_sub(VISIBLE_START_CCK))
                    * HIRES_PIXELS_PER_CCK;
            }
            if horizontal == VISIBLE_START_CCK {
                cursor.frame_x = 0;
            }
            cursor.active = horizontal >= VISIBLE_START_CCK && cursor.frame_y.is_some();
        } else {
            // A counter wrap without a line strobe has no preceding row prefix.
            cursor.active = cursor
                .pending_line
                .is_some_and(|line| !line.blank && line.ccks >= usize::from(VISIBLE_START_CCK))
                && cursor.frame_y.is_some();
        }
        cursor.active &= !self.sync.vertical_blank;
        cursor.last_hpos = horizontal;
    }
}
