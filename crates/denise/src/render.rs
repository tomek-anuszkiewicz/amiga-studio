//! Per-pixel mixing, horizontal display window and collision accumulation.

use crate::{decode, Denise};
use frame_builder::{rgb444_to_argb32, HBLANK_START_CCK, HBLANK_STOP_CCK, LORES_PIXELS_PER_CCK};

const BIT_DIW_HORIZONTAL_LOW: u16 = 1;
const DIW_COUNT_LEAD: u16 = 2;
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
        self.frame_builder.step_cck(horizontal, vertical);
        let blank =
            self.sync.vertical_blank || (HBLANK_START_CCK..=HBLANK_STOP_CCK).contains(&horizontal);
        let hires = self.is_hires();
        let count = self.bitplane_count().min(config::OCS_BITPLANE_COUNT as u8);
        let plane_mask = (1u8 << count) - 1;
        let border = rgb444_to_argb32(self.color[0]);
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
                let planes = planes & plane_mask;
                if in_window && !blank {
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
                    // Blanking gates storage below. Keep border color in flight
                    // so blanking cannot inject black into the first visible pair.
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
            if !blank {
                self.frame_builder
                    .write_pixel_pair(usize::from(position), output);
            }
        }
    }

    fn horizontal_window(&mut self, count: u16) -> bool {
        let start = (self.diwstrt & 0xff) | BIT_DIW_HORIZONTAL_LOW;
        let stop =
            (self.diwstop & 0xff) | BIT_DIW_HORIZONTAL_LOW | frame_builder::BIT_HSTOP_EXTENSION;
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
