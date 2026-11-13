//! HRM chapter 4 sprite output and chapter 7 collisions, cached per raster line.

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
const BIT_ODD_PIXEL_SHIFT: u8 = 2;
const MASK_CHANNEL_PIXEL: u16 = 3;
const MASK_PAIR_PIXEL: u16 = MASK_CHANNEL_PIXEL | (MASK_CHANNEL_PIXEL << BIT_ODD_PIXEL_SHIFT);
const MASK_PAIR_PRESENCE: u8 = 0x0f;
const MASK_PAIR_COLLISIONS: u16 = 0x7e00;
const MASK_ODD_PLANES: u8 = 0x15;
const MASK_EVEN_PLANES: u8 = 0x2a;
const BIT_COLLISION_ENABLE_SHIFT: u16 = 6;
const BIT_FIRST_ODD_COLLISION: u16 = 0x0002;
const BIT_FIRST_EVEN_COLLISION: u16 = 0x0020;
const BIT_PLAYFIELD_COLLISION: u16 = 0x0001;
const PLANE_PIXEL_CODES: usize = 1 << config::OCS_BITPLANE_COUNT;
const LINE_PIXELS: usize =
    (crate::MASK_HORIZONTAL_CURSOR as usize + 1) * frame_builder::LORES_PIXELS_PER_CCK;
const MASK_LINE_POSITION: usize = LINE_PIXELS - 1;
const NO_OUTPUT_POSITION: u32 = u32::MAX;
const BIT_COLOR_SHIFT: u32 = 16;
const BIT_PAIR_SHIFT: u32 = 21;
const MASK_COLOR: u32 = 0x1f;
const MASK_PAIR: u32 = 3;
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

/// CLXCON and decoded collision conditions, rebuilt on writes and deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u16", into = "u16")]
pub struct CollisionControl {
    value: u16,
    odd_pairs: u8,
    plane_masks: [u16; PLANE_PIXEL_CODES],
}

impl CollisionControl {
    #[inline]
    pub fn value(&self) -> u16 {
        self.value
    }

    #[inline]
    pub(crate) fn plane_collisions(&self, planes: u8, sprite_collisions: u16) -> u16 {
        self.plane_masks[usize::from(planes)] & (sprite_collisions | BIT_PLAYFIELD_COLLISION)
    }
}

impl Default for CollisionControl {
    fn default() -> Self {
        Self::from(0)
    }
}

impl From<u16> for CollisionControl {
    fn from(value: u16) -> Self {
        let mut plane_masks = [0; PLANE_PIXEL_CODES];
        for (planes, mask) in plane_masks.iter_mut().enumerate() {
            let mismatch =
                (planes as u8 ^ value as u8) & (value >> BIT_COLLISION_ENABLE_SHIFT) as u8;
            let odd = mismatch & MASK_ODD_PLANES == 0;
            let even = mismatch & MASK_EVEN_PLANES == 0;
            if odd {
                *mask |= u16::from(MASK_PAIR_PRESENCE) * BIT_FIRST_ODD_COLLISION;
            }
            if even {
                *mask |= u16::from(MASK_PAIR_PRESENCE) * BIT_FIRST_EVEN_COLLISION;
            }
            if odd && even {
                *mask |= BIT_PLAYFIELD_COLLISION;
            }
        }
        Self {
            value,
            odd_pairs: (value >> BIT_ODD_COLLISION_SHIFT) as u8,
            plane_masks,
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

    /// Project only the pixels already consumed since the stored channel state.
    fn advance(&mut self, first: u32, next: u32, hstart: u16) {
        let hstart = u32::from(hstart);
        let elapsed = if self.is_armed && (first..next).contains(&hstart) {
            self.shift_a = self.data_a;
            self.shift_b = self.data_b;
            self.pixel_counter = SPRITE_WORD_PIXELS;
            next - hstart
        } else {
            next - first
        };
        let shifted = elapsed.min(u32::from(self.pixel_counter));
        self.shift_a = self.shift_a.checked_shl(shifted).unwrap_or(0);
        self.shift_b = self.shift_b.checked_shl(shifted).unwrap_or(0);
        self.pixel_counter = self.pixel_counter.wrapping_sub(shifted as u8);
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "SpriteState", into = "SpriteState")]
pub struct Sprites {
    channels: [SpriteChannel; config::SPRITE_COUNT],
    /// Selected pair presence at the current low-resolution pixel.
    pub collision_pairs: u8,
    hstart: [u16; config::SPRITE_COUNT],
    attached: [bool; SPRITE_PAIRS],
    /// Channel state is anchored here; inspection projects it to next_horizontal.
    first_horizontal: u32,
    next_horizontal: u32,
    first_output: u32,
    inactive: bool,
    line_valid: bool,
    line_odd_pairs: u8,
    /// Packed palette index, winning pair and potential collision bits by H position.
    line: [u32; LINE_PIXELS],
    collision_bits: u16,
}

impl Default for Sprites {
    fn default() -> Self {
        Self {
            channels: [SpriteChannel::default(); config::SPRITE_COUNT],
            collision_pairs: 0,
            hstart: [0; config::SPRITE_COUNT],
            attached: [false; SPRITE_PAIRS],
            first_horizontal: 0,
            next_horizontal: 0,
            first_output: NO_OUTPUT_POSITION,
            inactive: true,
            line_valid: false,
            line_odd_pairs: 0,
            line: [0; LINE_PIXELS],
            collision_bits: 0,
        }
    }
}

impl PartialEq for Sprites {
    fn eq(&self, other: &Self) -> bool {
        self.collision_pairs == other.collision_pairs && self.channels() == other.channels()
    }
}

impl Eq for Sprites {}

impl std::fmt::Debug for Sprites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sprites")
            .field("channels", &self.channels())
            .field("collision_pairs", &self.collision_pairs)
            .finish()
    }
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
            collision_bits: collision_bits(state.collision_pairs & MASK_PAIR_PRESENCE),
            ..Self::default()
        };
        for (index, channel) in sprites.channels.iter().enumerate() {
            sprites.hstart[index] = channel.hstart();
        }
        for pair in 0..SPRITE_PAIRS {
            sprites.attached[pair] = sprites.channels[pair * 2 + 1].is_attached();
        }
        sprites.invalidate_line();
        sprites
    }
}

impl From<Sprites> for SpriteState {
    fn from(sprites: Sprites) -> Self {
        Self {
            channels: sprites.channels(),
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

    /// Inspect current serializers without exposing or advancing future cached output.
    pub fn channels(&self) -> [SpriteChannel; config::SPRITE_COUNT] {
        let mut channels = self.channels;
        for (index, channel) in channels.iter_mut().enumerate() {
            channel.advance(
                self.first_horizontal,
                self.next_horizontal,
                self.hstart[index],
            );
        }
        channels
    }

    fn materialize_channels(&mut self) {
        if self.first_horizontal != self.next_horizontal {
            self.channels = self.channels();
            self.first_horizontal = self.next_horizontal;
        }
    }

    fn invalidate_line(&mut self) {
        self.line_valid = false;
        self.first_output = NO_OUTPUT_POSITION;
        self.inactive = true;
        for (index, channel) in self.channels.iter().enumerate() {
            if channel.is_armed || channel.pixel_counter != 0 {
                self.inactive = false;
            }
            if channel.pixel_counter != 0 {
                self.first_output = self.next_horizontal;
                break;
            }
            let hstart = u32::from(self.hstart[index]);
            if channel.is_armed && hstart >= self.next_horizontal {
                self.first_output = self.first_output.min(hstart);
            }
        }
    }

    /// Apply a delivered POS write; pending writes must not change the comparator.
    pub fn write_pos(&mut self, index: usize, value: u16) {
        self.materialize_channels();
        if let Some(channel) = self.channels.get_mut(index) {
            channel.pos = value;
            self.hstart[index] = channel.hstart();
        }
        self.invalidate_line();
    }

    /// CTL disarms reloads without stopping the word already in the shifters.
    pub fn write_ctl(&mut self, index: usize, value: u16) {
        self.materialize_channels();
        if let Some(channel) = self.channels.get_mut(index) {
            channel.ctl = value;
            channel.is_armed = false;
            self.hstart[index] = channel.hstart();
            if index % 2 != 0 {
                self.attached[index / 2] = channel.is_attached();
            }
        }
        self.invalidate_line();
    }

    pub fn write_data(&mut self, index: usize, value: u16) {
        self.materialize_channels();
        if let Some(channel) = self.channels.get_mut(index) {
            channel.data_a = value;
            channel.is_armed = true;
        }
        self.invalidate_line();
    }

    pub fn write_datb(&mut self, index: usize, value: u16) {
        self.materialize_channels();
        if let Some(channel) = self.channels.get_mut(index) {
            channel.data_b = value;
        }
        self.invalidate_line();
    }

    /// A line boundary ends serialization but preserves the armed input latches.
    pub fn step_cck(&mut self, hpos: u16) {
        if hpos == 0 {
            self.materialize_channels();
            for channel in &mut self.channels {
                channel.pixel_counter = 0;
            }
            self.first_horizontal = 0;
            self.next_horizontal = 0;
            self.collision_pairs = 0;
            self.collision_bits = 0;
            self.invalidate_line();
        }
    }

    #[inline]
    pub(crate) fn collision_bits(&self) -> u16 {
        self.collision_bits
    }

    /// Consume current output only; future cached collisions never enter CLXDAT early.
    #[inline]
    pub fn evaluate_pixel(
        &mut self,
        horizontal: u16,
        control: &CollisionControl,
        collisions: &mut u16,
    ) -> Option<SpritePixel> {
        if self.inactive {
            self.next_horizontal = u32::from(horizontal) + 1;
            self.first_horizontal = self.next_horizontal;
            self.collision_pairs = 0;
            self.collision_bits = 0;
            None
        } else {
            self.evaluate_line(horizontal, control, collisions)
        }
    }

    fn evaluate_line(
        &mut self,
        horizontal: u16,
        control: &CollisionControl,
        collisions: &mut u16,
    ) -> Option<SpritePixel> {
        let position = u32::from(horizontal);
        if position != self.next_horizontal
            || position - self.first_horizontal >= LINE_PIXELS as u32
            || (self.line_valid && control.odd_pairs != self.line_odd_pairs)
        {
            self.materialize_channels();
            self.first_horizontal = position;
            self.next_horizontal = position;
            self.invalidate_line();
        }
        if !self.line_valid && position >= self.first_output {
            self.materialize_channels();
            self.build_line(control.odd_pairs);
        }
        let pixel = if self.line_valid {
            self.line[usize::from(horizontal) & MASK_LINE_POSITION]
        } else {
            0
        };
        self.next_horizontal = position + 1;
        self.collision_bits = pixel as u16;
        self.collision_pairs =
            (self.collision_bits / BIT_FIRST_ODD_COLLISION) as u8 & MASK_PAIR_PRESENCE;
        *collisions |= self.collision_bits & MASK_PAIR_COLLISIONS;
        let color = (pixel >> BIT_COLOR_SHIFT) & MASK_COLOR;
        if color == 0 {
            None
        } else {
            Some(SpritePixel {
                color_index: color as usize,
                sprite_pair: ((pixel >> BIT_PAIR_SHIFT) & MASK_PAIR) as usize,
            })
        }
    }

    fn build_line(&mut self, odd_pairs: u8) {
        self.line.fill(0);
        let mut pixels = [0u16; LINE_PIXELS];
        let mut occupied = [0u64; LINE_PIXELS / u64::BITS as usize];
        let start = self.first_horizontal;
        let end = start + LINE_PIXELS as u32;
        for (index, channel) in self.channels.iter().enumerate() {
            let hstart = u32::from(self.hstart[index]);
            let reload = channel.is_armed && hstart >= start;
            let prefix = if reload {
                u32::from(channel.pixel_counter).min(hstart - start)
            } else {
                u32::from(channel.pixel_counter)
            };
            add_word(
                &mut pixels,
                &mut occupied,
                index,
                start,
                prefix,
                channel.shift_a,
                channel.shift_b,
            );
            if reload {
                add_word(
                    &mut pixels,
                    &mut occupied,
                    index,
                    hstart,
                    // Later pixels belong to the next cache window, not its start.
                    u32::from(SPRITE_WORD_PIXELS).min(end - hstart),
                    channel.data_a,
                    channel.data_b,
                );
            }
        }
        for (word, mut mask) in occupied.into_iter().enumerate() {
            while mask != 0 {
                let position = word * u64::BITS as usize + mask.trailing_zeros() as usize;
                mask &= mask - 1;
                self.line[position] = self.decode_pixel(pixels[position], odd_pairs);
            }
        }
        self.line_odd_pairs = odd_pairs;
        self.line_valid = true;
    }

    fn decode_pixel(&self, pixels: u16, odd_pairs: u8) -> u32 {
        let mut result = 0;
        let mut presence = 0;
        for pair in 0..SPRITE_PAIRS {
            let packed =
                ((pixels >> (pair * 2 * usize::from(BIT_ODD_PIXEL_SHIFT))) & MASK_PAIR_PIXEL) as u8;
            let even = u16::from(packed) & MASK_CHANNEL_PIXEL;
            let odd = packed >> BIT_ODD_PIXEL_SHIFT;
            if even != 0 || (odd_pairs & (1 << pair) != 0 && odd != 0) {
                presence |= 1 << pair;
            }
            if result == 0 && packed != 0 {
                let attached = self.attached[pair];
                let code = PAIR_COLORS[usize::from(attached)][usize::from(packed)];
                let base = SPRITE_PALETTE_BASE + if attached { 0 } else { pair * COLORS_PER_PAIR };
                result = ((base + usize::from(code)) as u32) << BIT_COLOR_SHIFT
                    | (pair as u32) << BIT_PAIR_SHIFT;
            }
        }
        result | u32::from(collision_bits(presence))
    }
}

fn collision_bits(presence: u8) -> u16 {
    PAIR_COLLISIONS[usize::from(presence)]
        | u16::from(presence) * BIT_FIRST_ODD_COLLISION
        | u16::from(presence) * BIT_FIRST_EVEN_COLLISION
}

/// Insert only actual word spans; the rest of the cached line stays transparent.
fn add_word(
    pixels: &mut [u16; LINE_PIXELS],
    occupied: &mut [u64; LINE_PIXELS / u64::BITS as usize],
    channel: usize,
    start: u32,
    count: u32,
    mut data_a: u16,
    mut data_b: u16,
) {
    for offset in 0..count.min(u32::from(SPRITE_WORD_PIXELS)) {
        let code =
            (data_a >> (SPRITE_WORD_PIXELS - 1)) | ((data_b >> (SPRITE_WORD_PIXELS - 1)) << 1);
        if code != 0 {
            let position = (start + offset) as usize & MASK_LINE_POSITION;
            pixels[position] |= code << (channel * usize::from(BIT_ODD_PIXEL_SHIFT));
            occupied[position / u64::BITS as usize] |= 1 << (position % u64::BITS as usize);
        }
        data_a <<= 1;
        data_b <<= 1;
    }
}
