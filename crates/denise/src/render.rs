//! Per-pixel mixing, horizontal display window and collision accumulation.

use crate::{decode, Denise};
use frame_builder::{rgb444_to_argb32, HBLANK_START_CCK, HBLANK_STOP_CCK, LORES_PIXELS_PER_CCK};

// The receiver counter restarts at two lores periods (ARP Denise counter).
// Composition feeds the following output pair, so compare its output phase.
const RECEIVER_DIW_COUNT_ORIGIN: u16 = 2;
const HIRES_SAMPLES_PER_LORES_PIXEL: usize =
    frame_builder::HIRES_PIXELS_PER_CCK / LORES_PIXELS_PER_CCK;
const DIW_COUNT_LEAD: u16 =
    RECEIVER_DIW_COUNT_ORIGIN + (crate::PIPELINE_PIXELS / HIRES_SAMPLES_PER_LORES_PIXEL) as u16;
const MASK_DIW_HORIZONTAL: u16 = 0x00ff;
const MASK_PRIORITY: u16 = 0x0007;
const BIT_SECOND_PRIORITY_SHIFT: u16 = 3;

impl Denise {
    pub(super) fn render_cck(&mut self, horizontal: u16, vertical: u16) {
        if horizontal == 0 {
            self.last_ham_rgb = self.color[0];
            self.bpl_armed = false;
            self.shifters.fill(0);
            self.scroll_history.fill(0);
            self.pipeline_pixels_valid.fill(false);
        }
        self.sprites.step_cck(horizontal);
        self.advance_frame_cursor(horizontal, vertical);
        let blank =
            self.sync.vertical_blank || (HBLANK_START_CCK..=HBLANK_STOP_CCK).contains(&horizontal);
        let hires = self.is_hires();
        let border = rgb444_to_argb32(self.color[0]);
        if blank {
            for half in 0..LORES_PIXELS_PER_CCK as u16 {
                let position = horizontal * LORES_PIXELS_PER_CCK as u16 + half;
                self.horizontal_window(position + DIW_COUNT_LEAD);
                self.sprites
                    .evaluate_pixel(position, &self.clxcon, &mut self.clxdat);
                if self.bpl_armed {
                    self.shift_pixel();
                    if hires {
                        self.shift_pixel();
                    }
                }
            }
            // Keep the preceding border color for the first visible output pair.
            self.pipeline_pixels = [border; 2];
            self.pipeline_pixels_valid = [true; 2];
            return;
        }
        let frame_y = self
            .frame_cursor
            .frame_y
            .filter(|_| self.frame_cursor.active);
        let plane_mask = self.bplcon0.plane_mask;
        let mut samples = [border; frame_builder::HIRES_PIXELS_PER_CCK];
        for half in 0..LORES_PIXELS_PER_CCK as u16 {
            let position = horizontal * LORES_PIXELS_PER_CCK as u16 + half;
            let in_window = self.horizontal_window(position + DIW_COUNT_LEAD);
            let sprite = self
                .sprites
                .evaluate_pixel(position, &self.clxcon, &mut self.clxdat);
            let first = if self.bpl_armed {
                self.shift_pixel()
            } else {
                0
            };
            let compose = |this: &mut Self, planes: u8| {
                if in_window {
                    let planes = planes & plane_mask;
                    this.accumulate_plane_collisions(planes);
                    let playfield = this.decode_pixel(planes);
                    let rgb = match sprite {
                        Some(sprite) if this.sprite_is_visible(planes, sprite.sprite_pair) => {
                            this.color[sprite.color_index]
                        }
                        _ => playfield,
                    };
                    rgb444_to_argb32(rgb)
                } else {
                    border
                }
            };
            let color = compose(self, first);
            let next = if hires {
                let second = if self.bpl_armed {
                    self.shift_pixel()
                } else {
                    0
                };
                [color, compose(self, second)]
            } else {
                [color; 2]
            };
            // Emit the old pair before replacing it, even after a hires-to-lores change.
            let mut output = self.pipeline_pixels;
            if !self.pipeline_pixels_valid[0] {
                output[0] = border;
            }
            if !self.pipeline_pixels_valid[1] {
                output[1] = border;
            }
            self.pipeline_pixels = next;
            self.pipeline_pixels_valid = [true; 2];
            let offset = usize::from(half) * 2;
            samples[offset..offset + 2].copy_from_slice(&output);
        }
        if let Some(frame_y) = frame_y {
            self.frame_builder
                .write_cck(self.frame_cursor.frame_x, frame_y, samples);
        }
        self.frame_cursor.frame_x += frame_builder::HIRES_PIXELS_PER_CCK;
    }

    fn horizontal_window(&mut self, count: u16) -> bool {
        let start = self.diwstrt & MASK_DIW_HORIZONTAL;
        let stop = (self.diwstop & MASK_DIW_HORIZONTAL) | frame_builder::BIT_HSTOP_EXTENSION;
        if count == start {
            self.hflop = true;
        }
        if count == stop {
            self.hflop = false;
        }
        self.hflop
    }

    fn sprite_is_visible(&self, planes: u8, pair: usize) -> bool {
        let first_priority = self.bplcon2 & MASK_PRIORITY;
        let second_priority = (self.bplcon2 >> BIT_SECOND_PRIORITY_SHIFT) & MASK_PRIORITY;
        if self.is_dual_playfield() {
            let (first, second) = decode::playfield_codes(planes);
            (first == 0 || pair < usize::from(first_priority))
                && (second == 0 || pair < usize::from(second_priority))
        } else {
            planes == 0 || pair < usize::from(second_priority)
        }
    }

    fn accumulate_plane_collisions(&mut self, planes: u8) {
        self.clxdat |= self
            .clxcon
            .plane_collisions(planes, self.sprites.collision_bits());
    }
}
