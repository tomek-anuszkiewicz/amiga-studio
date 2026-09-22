//! Agnus 4-Channel DMA Blitter Emulation
//!
//! High-speed block image transfer, 256-minterm logic unit, barrel shifter,
//! and Bresenham line drawer.

pub mod line;
pub mod minterm;
pub mod phase;

pub use line::*;
pub use minterm::*;
pub use phase::*;

use serde::{Deserialize, Serialize};

#[inline(always)]
fn read_chip_ram_u16(ram: &[u8], addr: u32) -> u16 {
    if ram.is_empty() {
        return 0xFFFF;
    }
    let mask = ram.len() - 1;
    let offset = (addr as usize & mask) & !1;
    if offset + 1 < ram.len() {
        u16::from_be_bytes([ram[offset], ram[offset + 1]])
    } else {
        0xFFFF
    }
}

#[inline(always)]
fn write_chip_ram_u16(ram: &mut [u8], addr: u32, val: u16) {
    if ram.is_empty() {
        return;
    }
    let mask = ram.len() - 1;
    let offset = (addr as usize & mask) & !1;
    if offset + 1 < ram.len() {
        let bytes = val.to_be_bytes();
        ram[offset] = bytes[0];
        ram[offset + 1] = bytes[1];
    }
}

/// Agnus 4-channel DMA Blitter state and registers
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Blitter {
    /// Active phase index within the current word's cycle sequence (0..3)
    pub phase_index: u8,
    /// Blitter Control 0 (channel enables A-D, minterms LF0-LF7, shift A)
    pub bltcon0: u16,
    /// Blitter Control 1 (shift B, descending flag, line mode, fill mode)
    pub bltcon1: u16,
    /// Blitter First Word Mask for Channel A
    pub bltafwm: u16,
    /// Blitter Last Word Mask for Channel A
    pub bltalwm: u16,
    /// Channel A source pointer (18-bit Chip RAM address)
    pub bltapt: u32,
    /// Channel B source pointer (18-bit Chip RAM address)
    pub bltbpt: u32,
    /// Channel C source/destination pointer (18-bit Chip RAM address)
    pub bltcpt: u32,
    /// Channel D destination pointer (18-bit Chip RAM address)
    pub bltdpt: u32,
    /// Blit size register: Height in rows (bits 6-15) and Width in words (bits 0-5)
    pub bltsize: u16,
    /// Channel A modulo (signed 16-bit)
    pub bltamod: i16,
    /// Channel B modulo (signed 16-bit)
    pub bltbmod: i16,
    /// Channel C modulo (signed 16-bit)
    pub bltcmod: i16,
    /// Channel D modulo (signed 16-bit)
    pub bltdmod: i16,
    /// Channel A data holding latch
    pub bltadat: u16,
    /// Channel B data holding latch
    pub bltbdat: u16,
    /// Channel C data holding latch
    pub bltcdat: u16,

    /// Latched old A word for barrel shifter
    pub aold: u16,
    /// Latched old B word for barrel shifter
    pub bold: u16,
    /// Current A input word
    pub anew: u16,
    /// Current B input word
    pub bnew: u16,
    /// Shifted and masked A operand
    pub ahold: u16,
    /// Shifted B operand
    pub bhold: u16,
    /// C operand holding register
    pub chold: u16,
    /// Computed D output word
    pub dhold: u16,
    /// Current fill carry bit
    pub fill_carry: bool,

    /// True while a blit operation is active
    pub is_busy: bool,
    /// True if all output words of the blit were zero (for collision/cookie cut)
    pub is_zero: bool,
    /// Blitter DMA channel enabled via DMACON (BLTEN bit 6 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// Blitter Nasty / CPU priority mode (BLTPRI bit 10 in DMACON)
    pub bltpri: bool,
    /// Level 3 blitter interrupt request strobe (_BLITINT)
    pub blit_irq: bool,

    /// Current row index during area blit
    pub cur_row: usize,
    /// Current column index (in words) during area blit
    pub cur_col: usize,
    /// Total rows in active blit
    pub total_rows: usize,
    /// Total words per row in active blit
    pub total_cols: usize,
    /// Active phase for cycle-by-cycle stepping
    pub active_phase: BlitterPhase,
    /// Bresenham line drawer state
    pub line_drawer: LineDrawer,
    /// Remaining pixels in line mode
    pub line_pixels_left: usize,
    /// Hardware pipeline startup cycles before word execution begins (BLT_STRT1, BLT_STRT2)
    pub startup_cycles: u8,
}

impl Blitter {
    /// Creates a new uninitialized Blitter instance
    pub fn new() -> Self {
        let mut b = Self::default();
        b.reset();
        b
    }

    /// Resets all Blitter registers to power-on defaults
    pub fn reset(&mut self) {
        self.phase_index = 0;
        self.startup_cycles = 0;
        self.bltcon0 = 0;
        self.bltcon1 = 0;
        self.bltafwm = 0;
        self.bltalwm = 0;
        self.bltapt = 0;
        self.bltbpt = 0;
        self.bltcpt = 0;
        self.bltdpt = 0;
        self.bltsize = 0;
        self.bltamod = 0;
        self.bltbmod = 0;
        self.bltcmod = 0;
        self.bltdmod = 0;
        self.bltadat = 0xAAAA;
        self.bltbdat = 0xAAAA;
        self.bltcdat = 0x5555;
        self.aold = 0;
        self.bold = 0;
        self.anew = 0xAAAA;
        self.bnew = 0xAAAA;
        self.ahold = 0xAAAA;
        self.bhold = 0xAAAA;
        self.chold = 0x5555;
        self.dhold = 0;
        self.fill_carry = false;
        self.is_busy = false;
        self.is_zero = true;
        self.dma_enabled = false;
        self.bltpri = false;
        self.blit_irq = false;
        self.cur_row = 0;
        self.cur_col = 0;
        self.total_rows = 0;
        self.total_cols = 0;
        self.active_phase = BlitterPhase::Idle;
        self.line_drawer.reset();
        self.line_pixels_left = 0;
    }

    /// Sets Blitter DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if !enabled {
            self.is_busy = false;
            self.active_phase = BlitterPhase::Idle;
        }
    }

    /// Sets Blitter Nasty priority mode from DMACON (BLTPRI, bit 10)
    #[inline]
    pub fn set_bltpri(&mut self, enabled: bool) {
        self.bltpri = enabled;
    }

    /// Action method: triggers blit execution when BLTSIZE matures
    pub fn trigger_blit(&mut self, bltsize: u16) {
        self.bltsize = bltsize;
        self.is_busy = true;
        self.is_zero = true;
        self.startup_cycles = 1;

        let width = match bltsize & 0x003F {
            0 => 64,
            w => w as usize,
        };
        let height = match bltsize >> 6 {
            0 => 1024,
            h => h as usize,
        };

        self.total_cols = width;
        self.total_rows = height;
        self.cur_row = 0;
        self.cur_col = 0;
        self.aold = 0;
        self.bold = 0;
        self.fill_carry = (self.bltcon1 & 0x0004) != 0;

        if (self.bltcon1 & 0x0001) != 0 {
            // Line mode
            self.line_pixels_left = height;
            self.line_drawer.reset();
            self.anew = self.bltadat;
            self.bnew = self.bltbdat;
            self.active_phase = BlitterPhase::LinePixel;
        } else {
            // Area mode
            self.phase_index = 0;
            self.active_phase = self.active_word_phases().0[0];
        }
    }

    /// Action method: finishes the active blit, resets busy, and asserts Level 3 `_BLITINT`
    #[inline]
    pub fn finish_blit(&mut self) {
        self.is_busy = false;
        self.blit_irq = true;
        self.active_phase = BlitterPhase::Idle;
    }

    /// Polls and clears the blitter interrupt request strobe (_BLITINT)
    #[inline]
    pub fn poll_blit_irq(&mut self) -> bool {
        let irq = self.blit_irq;
        self.blit_irq = false;
        irq
    }

    /// Advances the Blitter state by 1 Color Clock (no RAM attached)
    #[inline]
    pub fn step_cck(&mut self) {
        self.step_cck_ram(&mut []);
    }

    /// Advances the Blitter state by 1 Color Clock with Chip RAM access
    pub fn step_cck_ram(&mut self, chip_ram: &mut [u8]) {
        if !self.is_busy || !self.dma_enabled {
            return;
        }

        if self.startup_cycles > 0 {
            self.startup_cycles -= 1;
            return;
        }

        if (self.bltcon1 & 0x0001) != 0 {
            // Line mode pixel step
            self.step_line_cycle(chip_ram);
        } else {
            // Area mode cycle step
            self.step_area_cycle(chip_ram);
        }
    }

    /// Returns the active micro-phase sequence for a word in Area Mode according to HRM Table 6.2
    #[inline]
    fn active_word_phases(&self) -> ([BlitterPhase; 4], u8) {
        let use_a = (self.bltcon0 & 0x0800) != 0;
        let use_b = (self.bltcon0 & 0x0400) != 0;
        let use_c = (self.bltcon0 & 0x0200) != 0;
        let use_d = (self.bltcon0 & 0x0100) != 0;
        let fill = (self.bltcon1 & 0x0018) != 0;
        word_phases(use_a, use_b, use_c, use_d, fill)
    }

    /// Evaluates barrel shift, minterm logic, fill, and optionally writes to channel D
    fn perform_word_computation_and_write(
        &mut self,
        chip_ram: &mut [u8],
        use_a: bool,
        use_b: bool,
        use_c: bool,
        use_d: bool,
        step: i32,
        desc: bool,
    ) {
        let mut mask = 0xFFFFu16;
        if self.cur_col == 0 {
            mask &= self.bltafwm;
        }
        if self.cur_col == self.total_cols - 1 {
            mask &= self.bltalwm;
        }

        if !use_a {
            self.anew = self.bltadat;
        }
        if !use_b {
            self.bnew = self.bltbdat;
        }
        if !use_c {
            self.chold = self.bltcdat;
        }

        let ash = (self.bltcon0 >> 12) & 0xF;
        let bsh = (self.bltcon1 >> 12) & 0xF;

        self.ahold = barrel_shift(self.anew & mask, self.aold, ash, desc);
        self.aold = self.anew & mask;

        if use_b {
            self.bhold = barrel_shift(self.bnew, self.bold, bsh, desc);
            self.bold = self.bnew;
        } else {
            self.bhold = self.bnew;
        }

        let minterm = (self.bltcon0 & 0xFF) as u8;
        self.dhold = eval_minterm(self.ahold, self.bhold, self.chold, minterm);

        if (self.bltcon1 & 0x0018) != 0 {
            let exclusive = (self.bltcon1 & 0x0010) != 0;
            self.dhold = apply_fill(self.dhold, &mut self.fill_carry, exclusive);
        }

        if self.dhold != 0 {
            self.is_zero = false;
        }

        if use_d {
            write_chip_ram_u16(chip_ram, self.bltdpt, self.dhold);
            self.bltdpt = self.bltdpt.wrapping_add(step as u32);
        }
    }

    /// Steps a single cycle in Area Blit mode following Table 6.2
    fn step_area_cycle(&mut self, chip_ram: &mut [u8]) {
        let (phases, count) = self.active_word_phases();
        let phase = phases[(self.phase_index as usize).min(count as usize - 1)];
        self.active_phase = phase;

        let use_a = (self.bltcon0 & 0x0800) != 0;
        let use_b = (self.bltcon0 & 0x0400) != 0;
        let use_c = (self.bltcon0 & 0x0200) != 0;
        let use_d = (self.bltcon0 & 0x0100) != 0;

        let desc = (self.bltcon1 & 0x0002) != 0;
        let step: i32 = if desc { -2 } else { 2 };
        let amod: i32 = if desc {
            -(self.bltamod as i32)
        } else {
            self.bltamod as i32
        };
        let bmod: i32 = if desc {
            -(self.bltbmod as i32)
        } else {
            self.bltbmod as i32
        };
        let cmod: i32 = if desc {
            -(self.bltcmod as i32)
        } else {
            self.bltcmod as i32
        };
        let dmod: i32 = if desc {
            -(self.bltdmod as i32)
        } else {
            self.bltdmod as i32
        };

        match phase {
            BlitterPhase::FetchA => {
                if use_a {
                    self.anew = read_chip_ram_u16(chip_ram, self.bltapt);
                    self.bltapt = self.bltapt.wrapping_add(step as u32);
                }
            }
            BlitterPhase::FetchB => {
                if use_b {
                    self.bnew = read_chip_ram_u16(chip_ram, self.bltbpt);
                    self.bltbpt = self.bltbpt.wrapping_add(step as u32);
                }
            }
            BlitterPhase::FetchC => {
                if use_c {
                    self.chold = read_chip_ram_u16(chip_ram, self.bltcpt);
                    self.bltcpt = self.bltcpt.wrapping_add(step as u32);
                }
            }
            BlitterPhase::WriteD => {
                self.perform_word_computation_and_write(
                    chip_ram, use_a, use_b, use_c, use_d, step, desc,
                );
            }
            BlitterPhase::BusIdle => {
                // If D is not used, perform minterm/zero check on the first cycle of the word
                if !use_d && self.phase_index == 0 {
                    self.perform_word_computation_and_write(
                        chip_ram, use_a, use_b, use_c, false, step, desc,
                    );
                }
            }
            _ => {}
        }

        self.phase_index += 1;
        if self.phase_index >= count {
            self.phase_index = 0;
            self.cur_col += 1;
            if self.cur_col >= self.total_cols {
                if use_a {
                    self.bltapt = self.bltapt.wrapping_add(amod as u32);
                }
                if use_b {
                    self.bltbpt = self.bltbpt.wrapping_add(bmod as u32);
                }
                if use_c {
                    self.bltcpt = self.bltcpt.wrapping_add(cmod as u32);
                }
                if use_d {
                    self.bltdpt = self.bltdpt.wrapping_add(dmod as u32);
                }

                self.cur_col = 0;
                self.cur_row += 1;
                if (self.bltcon1 & 0x0018) != 0 {
                    self.fill_carry = (self.bltcon1 & 0x0004) != 0;
                }
                if self.cur_row >= self.total_rows {
                    self.finish_blit();
                }
            }
        }
    }

    /// Steps a single pixel in Line mode
    fn step_line_cycle(&mut self, chip_ram: &mut [u8]) {
        let use_b = (self.bltcon0 & 0x0400) != 0;
        let use_c = (self.bltcon0 & 0x0200) != 0;
        let sing = (self.bltcon1 & 0x0002) != 0;
        let minterm = (self.bltcon0 & 0xFF) as u8;

        if use_b {
            self.bnew = read_chip_ram_u16(chip_ram, self.bltbpt);
            self.bltbpt = self.bltbpt.wrapping_add(self.bltbmod as i32 as u32);
        }

        if use_c {
            self.chold = read_chip_ram_u16(chip_ram, self.bltcpt);
        }

        let ash = (self.bltcon0 >> 12) & 0xF;
        let mut bsh = (self.bltcon1 >> 12) & 0xF;

        self.ahold = (self.anew & self.bltafwm) >> ash;

        let b_rot = ((((self.bnew as u32) << 16) | (self.bnew as u32)) >> bsh) as u16;
        let texture_mask = if (b_rot & 1) != 0 { 0xFFFF } else { 0x0000 };

        bsh = bsh.wrapping_sub(1) & 0xF;
        self.bltcon1 = (self.bltcon1 & 0x0FFF) | (bsh << 12);

        self.dhold = eval_minterm(self.ahold, texture_mask, self.chold, minterm);

        let write_enable = (!sing || self.line_drawer.first_pixel_on_line) && use_c;

        self.line_drawer.step_pixel(
            &mut self.bltcon0,
            &mut self.bltcon1,
            &mut self.bltapt,
            self.bltamod,
            self.bltbmod,
            self.bltcmod,
            &mut self.bltcpt,
        );

        if self.dhold != 0 {
            self.is_zero = false;
        }

        if write_enable {
            write_chip_ram_u16(chip_ram, self.bltdpt, self.dhold);
        }

        self.bltdpt = self.bltcpt;

        if self.line_pixels_left > 0 {
            self.line_pixels_left -= 1;
        }
        if self.line_pixels_left == 0 {
            self.finish_blit();
        }
    }

    /// Executes the active blit to completion immediately (synchronous one-shot)
    pub fn execute_blit(&mut self, chip_ram: &mut [u8]) {
        if !self.is_busy {
            if self.bltsize == 0 {
                return;
            }
            self.trigger_blit(self.bltsize);
        }
        let line_mode = (self.bltcon1 & 0x0001) != 0;
        while self.is_busy {
            if line_mode {
                self.step_line_cycle(chip_ram);
            } else {
                self.step_area_cycle(chip_ram);
            }
        }
    }
}
