//! HRM chapter 4 horizontal sprite serializers and chapter 7 pair collisions.

use serde::{Deserialize, Serialize};

const BIT_VSTART_HIGH: u16 = 0x0004;
const BIT_VSTOP_HIGH: u16 = 0x0002;
const BIT_HSTART_LOW: u16 = 0x0001;
const BIT_ATTACH: u16 = 0x0080;
const SPRITE_WORD_PIXELS: u8 = 16;
const SPRITE_PAIRS: usize = config::SPRITE_COUNT / 2;
const SPRITE_PALETTE_BASE: usize = 16;
const COLORS_PER_PAIR: usize = 4;
const BIT_ODD_COLLISION_ENABLE: u16 = 0x1000;
const BIT_FIRST_PAIR_COLLISION: u16 = 0x0200;
const PAIR_COLLISIONS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpritePixel {
    pub color_index: usize,
    pub sprite_pair: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpriteChannel {
    pub pos: u16,
    pub ctl: u16,
    pub data_a: u16,
    pub data_b: u16,
    pub shift_a: u16,
    pub shift_b: u16,
    pub pixel_counter: u8,
    pub is_armed: bool,
}

impl SpriteChannel {
    /// Register inspection; the vertical DMA comparator belongs to Agnus.
    pub fn vstart(&self) -> u16 {
        (self.pos >> 8) | ((self.ctl & BIT_VSTART_HIGH) << 6)
    }

    pub fn vstop(&self) -> u16 {
        (self.ctl >> 8) | ((self.ctl & BIT_VSTOP_HIGH) << 7)
    }

    pub fn hstart(&self) -> u16 {
        ((self.pos & 0xff) << 1) | (self.ctl & BIT_HSTART_LOW)
    }

    pub fn is_attached(&self) -> bool {
        self.ctl & BIT_ATTACH != 0
    }

    pub fn shift_pixel(&mut self) -> u8 {
        if self.pixel_counter == 0 {
            return 0;
        }
        let pixel = ((self.shift_a >> 15) | ((self.shift_b >> 15) << 1)) as u8;
        self.shift_a <<= 1;
        self.shift_b <<= 1;
        self.pixel_counter = self.pixel_counter.wrapping_sub(1);
        pixel
    }

    fn sample(&mut self, horizontal: u16) -> u8 {
        if self.is_armed && horizontal == self.hstart() {
            self.shift_a = self.data_a;
            self.shift_b = self.data_b;
            self.pixel_counter = SPRITE_WORD_PIXELS;
        }
        self.shift_pixel()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sprites {
    pub channels: [SpriteChannel; config::SPRITE_COUNT],
    /// Selected pair presence at the current low-resolution pixel.
    pub collision_pairs: u8,
}

impl Sprites {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// A line boundary ends serialization but preserves the armed input latches.
    pub fn step_cck(&mut self, hpos: u16) {
        if hpos == 0 {
            for channel in &mut self.channels {
                channel.pixel_counter = 0;
            }
            self.collision_pairs = 0;
        }
    }

    /// Evaluate all channels before choosing a color; hidden pairs still collide.
    pub fn evaluate_pixel(
        &mut self,
        horizontal: u16,
        control: u16,
        collisions: &mut u16,
    ) -> Option<SpritePixel> {
        let mut pixels = [0u8; config::SPRITE_COUNT];
        for (channel, pixel) in self.channels.iter_mut().zip(&mut pixels) {
            *pixel = channel.sample(horizontal);
        }
        self.collision_pairs = 0;
        let mut winner = None;
        for pair in 0..SPRITE_PAIRS {
            let even = pair * 2;
            let odd = even + 1;
            if pixels[even] != 0
                || (control & (BIT_ODD_COLLISION_ENABLE << pair) != 0 && pixels[odd] != 0)
            {
                self.collision_pairs |= 1 << pair;
            }
            let color = if self.channels[odd].is_attached() {
                let code = pixels[even] | (pixels[odd] << 2);
                if code == 0 {
                    None
                } else {
                    Some(SPRITE_PALETTE_BASE + usize::from(code))
                }
            } else {
                let code = if pixels[even] != 0 {
                    pixels[even]
                } else {
                    pixels[odd]
                };
                if code == 0 {
                    None
                } else {
                    Some(SPRITE_PALETTE_BASE + pair * COLORS_PER_PAIR + usize::from(code))
                }
            };
            if winner.is_none() {
                winner = color.map(|color_index| SpritePixel {
                    color_index,
                    sprite_pair: pair,
                });
            }
        }
        for (index, (first, second)) in PAIR_COLLISIONS.into_iter().enumerate() {
            if self.collision_pairs & (1 << first) != 0 && self.collision_pairs & (1 << second) != 0
            {
                *collisions |= BIT_FIRST_PAIR_COLLISION << index;
            }
        }
        winner
    }
}
