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
const BIT_ODD_COLLISION_SHIFT: u16 = 12;
const MASK_PAIR_CHANNELS: u8 = 3;
const BIT_ODD_PIXEL_SHIFT: u8 = 2;
// Indexed by the four collision-group presence bits; CLXDAT bits 9..14.
const PAIR_COLLISIONS: [u16; 16] = [
    0, 0, 0, 0x0200, 0, 0x0400, 0x1000, 0x1600, 0, 0x0800, 0x2000, 0x2a00, 0x4000, 0x4c00, 0x7000,
    0x7e00,
];
// Packed even/odd pixels: independent priority, then attached four-bit color.
const PAIR_COLORS: [[u8; 16]; 2] = [
    [0, 1, 2, 3, 1, 1, 2, 3, 2, 1, 2, 3, 3, 1, 2, 3],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
];

/// CLXCON and its decoded sprite enables, rebuilt on writes and deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "u16", into = "u16")]
pub struct CollisionControl {
    value: u16,
    odd_pairs: u8,
}

impl CollisionControl {
    #[inline]
    pub fn value(self) -> u16 {
        self.value
    }
}

impl From<u16> for CollisionControl {
    fn from(value: u16) -> Self {
        Self {
            value,
            odd_pairs: (value >> BIT_ODD_COLLISION_SHIFT) as u8,
        }
    }
}

impl From<CollisionControl> for u16 {
    fn from(control: CollisionControl) -> Self {
        control.value
    }
}

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
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "SpriteState", into = "SpriteState")]
pub struct Sprites {
    channels: [SpriteChannel; config::SPRITE_COUNT],
    /// Selected pair presence at the current low-resolution pixel.
    pub collision_pairs: u8,
    hstart: [u16; config::SPRITE_COUNT],
    attached: [bool; SPRITE_PAIRS],
    /// Armed channels and disarmed channels whose loaded word is still shifting.
    active_channels: u8,
}

// Only hardware state is persisted. Derived caches cannot survive stale restores.
#[derive(Serialize, Deserialize)]
struct SpriteState {
    channels: [SpriteChannel; config::SPRITE_COUNT],
    collision_pairs: u8,
}

impl From<SpriteState> for Sprites {
    fn from(state: SpriteState) -> Self {
        let mut sprites = Self {
            channels: state.channels,
            collision_pairs: state.collision_pairs,
            ..Self::default()
        };
        for (index, channel) in sprites.channels.iter().enumerate() {
            sprites.hstart[index] = channel.hstart();
            if channel.is_armed || channel.pixel_counter != 0 {
                sprites.active_channels |= 1 << index;
            }
        }
        for pair in 0..SPRITE_PAIRS {
            sprites.attached[pair] = sprites.channels[pair * 2 + 1].is_attached();
        }
        sprites
    }
}

impl From<Sprites> for SpriteState {
    fn from(sprites: Sprites) -> Self {
        Self {
            channels: sprites.channels,
            collision_pairs: sprites.collision_pairs,
        }
    }
}

impl Sprites {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    #[inline]
    pub fn channels(&self) -> &[SpriteChannel; config::SPRITE_COUNT] {
        &self.channels
    }

    /// Apply a delivered POS write; pending writes must not change the comparator.
    pub fn write_pos(&mut self, index: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.pos = value;
            self.hstart[index] = channel.hstart();
        }
    }

    /// CTL disarms reloads without stopping the word already in the shifters.
    pub fn write_ctl(&mut self, index: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.ctl = value;
            channel.is_armed = false;
            self.hstart[index] = channel.hstart();
            if index % 2 != 0 {
                self.attached[index / 2] = channel.is_attached();
            }
            if channel.pixel_counter == 0 {
                self.active_channels &= !(1 << index);
            }
        }
    }

    pub fn write_data(&mut self, index: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.data_a = value;
            channel.is_armed = true;
            self.active_channels |= 1 << index;
        }
    }

    pub fn write_datb(&mut self, index: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.data_b = value;
        }
    }

    /// A line boundary ends serialization but preserves the armed input latches.
    pub fn step_cck(&mut self, hpos: u16) {
        if hpos == 0 {
            self.active_channels = 0;
            for (index, channel) in self.channels.iter_mut().enumerate() {
                channel.pixel_counter = 0;
                if channel.is_armed {
                    self.active_channels |= 1 << index;
                }
            }
            self.collision_pairs = 0;
        }
    }

    fn sample_channel(&mut self, index: usize, horizontal: u16) -> u8 {
        let channel = &mut self.channels[index];
        if channel.is_armed && horizontal == self.hstart[index] {
            channel.shift_a = channel.data_a;
            channel.shift_b = channel.data_b;
            channel.pixel_counter = SPRITE_WORD_PIXELS;
        }
        if channel.pixel_counter == 0 {
            return 0;
        }
        let pixel = channel.shift_pixel();
        if !channel.is_armed && channel.pixel_counter == 0 {
            self.active_channels &= !(1 << index);
        }
        pixel
    }

    /// Hidden pairs still advance and collide after the first color wins.
    pub fn evaluate_pixel(
        &mut self,
        horizontal: u16,
        control: CollisionControl,
        collisions: &mut u16,
    ) -> Option<SpritePixel> {
        self.collision_pairs = 0;
        if self.active_channels == 0 {
            return None;
        }
        let mut winner = None;
        for pair in 0..SPRITE_PAIRS {
            let even = pair * 2;
            if self.active_channels & (MASK_PAIR_CHANNELS << even) == 0 {
                continue;
            }
            let even_pixel = self.sample_channel(even, horizontal);
            let odd_pixel = self.sample_channel(even + 1, horizontal);
            if even_pixel != 0 || (control.odd_pairs & (1 << pair) != 0 && odd_pixel != 0) {
                self.collision_pairs |= 1 << pair;
            }
            let packed = even_pixel | (odd_pixel << BIT_ODD_PIXEL_SHIFT);
            if winner.is_none() && packed != 0 {
                let attached = self.attached[pair];
                let code = PAIR_COLORS[usize::from(attached)][usize::from(packed)];
                let base = SPRITE_PALETTE_BASE + if attached { 0 } else { pair * COLORS_PER_PAIR };
                winner = Some(SpritePixel {
                    color_index: base + usize::from(code),
                    sprite_pair: pair,
                });
            }
        }
        *collisions |= PAIR_COLLISIONS[usize::from(self.collision_pairs)];
        winner
    }
}
