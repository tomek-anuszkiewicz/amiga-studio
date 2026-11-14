use denise::{frame_builder::FrameBuilder, Denise, DeniseModel};
use std::ops::{Deref, DerefMut};

/// Drives the receiver with the same line events and CCK positions as the
/// storage tests, then injects identifiable samples at its output coordinates.
pub struct Video {
    pub denise: Denise,
}

impl Video {
    pub fn new() -> Self {
        Self {
            denise: Denise::new(DeniseModel::Ocs8362),
        }
    }

    pub fn begin_line(&mut self, interlaced: bool, odd: bool, completed: bool, blank: bool) {
        self.denise.bplcon0 = (if interlaced { 4 } else { 0 }).into();
        self.denise.sync.field_odd = if completed && interlaced { !odd } else { odd };
        self.denise.sync.field_locked = completed;
        self.denise.sync.equalization = false;
        if completed {
            assert!(blank);
            self.denise.strobe_strequ();
        } else if blank {
            self.denise.strobe_strvbl();
        } else {
            self.denise.strobe_strhor();
        }
    }

    pub fn step_cck(&mut self, hpos: u16, vpos: u16) {
        self.denise.sync.hpos = hpos;
        self.denise.sync.vpos = vpos;
        self.denise.step_cck();
    }

    pub fn write_pair(&mut self, half: usize, pixels: [u32; 2]) {
        let cursor = &self.denise.frame_cursor;
        if cursor.active {
            if let Some(row) = cursor.frame_y {
                let x = cursor.frame_x - 4 + half * 2;
                self.denise.frame_builder.write_pixel_pair(x, row, pixels);
            }
        }
    }

    pub fn reset(&mut self) {
        self.denise.reset();
    }
}

impl Clone for Video {
    fn clone(&self) -> Self {
        Self {
            denise: self.denise.clone(),
        }
    }
}

impl Deref for Video {
    type Target = FrameBuilder;
    fn deref(&self) -> &Self::Target {
        &self.denise.frame_builder
    }
}

impl DerefMut for Video {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.denise.frame_builder
    }
}
