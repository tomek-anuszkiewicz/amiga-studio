//! Per-pixel mixing, horizontal display window and collision accumulation.

use crate::{decode, Denise};
use frame_builder::{
    rgb444_to_argb32, HBLANK_START_CCK, HBLANK_STOP_CCK, LORES_PIXELS_PER_CCK, OPAQUE_BLACK,
};

const BIT_DIW_HORIZONTAL_LOW: u16 = 1;
const DIW_COUNT_LEAD: u16 = 2;
const MASK_PRIORITY: u16 = 0x0007;
const BIT_SECOND_PRIORITY_SHIFT: u16 = 3;
const MASK_ODD_PLANES: u8 = 0x15;
const MASK_EVEN_PLANES: u8 = 0x2a;
const BIT_COLLISION_ENABLE_SHIFT: u16 = 6;
const BIT_FIRST_ODD_COLLISION: u16 = 0x0002;
const BIT_FIRST_EVEN_COLLISION: u16 = 0x0020;
const BIT_PLAYFIELD_COLLISION: u16 = 0x0001;

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
        for half in 0..LORES_PIXELS_PER_CCK as u16 {
            let position = horizontal * LORES_PIXELS_PER_CCK as u16 + half;
            let in_window = self.horizontal_window(position + DIW_COUNT_LEAD);
            let sprite = self
                .sprites
                .evaluate_pixel(position, self.clxcon, &mut self.clxdat);
            let first = if self.bpl_armed {
                self.shift_pixel()
            } else {
                0
            };
            let mut output = [OPAQUE_BLACK; 2];
            for (subpixel, output_pixel) in output.iter_mut().enumerate() {
                let planes = if hires && subpixel != 0 && self.bpl_armed {
                    self.shift_pixel()
                } else {
                    first
                };
                let count = self.bitplane_count().min(config::OCS_BITPLANE_COUNT as u8);
                let planes = planes & ((1u8 << count) - 1);
                let color = if blank {
                    OPAQUE_BLACK
                } else if in_window {
                    self.accumulate_plane_collisions(planes);
                    let playfield = self.decode_pixel(planes);
                    let rgb = match &sprite {
                        Some(sprite) if self.sprite_is_visible(planes, sprite.sprite_pair) => {
                            self.color[sprite.color_index]
                        }
                        _ => playfield,
                    };
                    rgb444_to_argb32(rgb)
                } else {
                    rgb444_to_argb32(self.color[0])
                };
                let pixel = if self.pipeline_pixels_valid[0] {
                    self.pipeline_pixels[0]
                } else {
                    rgb444_to_argb32(self.color[0])
                };
                self.pipeline_pixels[0] = self.pipeline_pixels[1];
                self.pipeline_pixels[1] = color;
                self.pipeline_pixels_valid[0] = self.pipeline_pixels_valid[1];
                self.pipeline_pixels_valid[1] = true;
                *output_pixel = pixel;
            }
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
        let control = self.clxcon.value();
        let mismatch = (planes ^ control as u8) & (control >> BIT_COLLISION_ENABLE_SHIFT) as u8;
        let odd = mismatch & MASK_ODD_PLANES == 0;
        let even = mismatch & MASK_EVEN_PLANES == 0;
        if odd && even {
            self.clxdat |= BIT_PLAYFIELD_COLLISION;
        }
        for pair in 0..config::SPRITE_COUNT / 2 {
            if self.sprites.collision_pairs & (1 << pair) == 0 {
                continue;
            }
            if odd {
                self.clxdat |= BIT_FIRST_ODD_COLLISION << pair;
            }
            if even {
                self.clxdat |= BIT_FIRST_EVEN_COLLISION << pair;
            }
        }
    }
}
