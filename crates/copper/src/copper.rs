//! Agnus Copper (Coprocessor) Emulation
//!
//! Synchronized coprocessor executing MOVE, WAIT, and SKIP instructions
//! driven in lockstep with the raster beam position.

use config::BeamPosition;
use serde::{Deserialize, Serialize};
/// Execution state of the Copper instruction pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CopperState {
    /// Halted / not executing instructions
    #[default]
    Idle,
    /// Fetching first instruction word (IR1) - remaining CCK cycles (2..0)
    FetchIR1(u8),
    /// Fetching second instruction word (IR2) - remaining CCK cycles (2..0)
    FetchIR2(u8),
    /// Waiting for beam coordinates or Blitter completion
    Waiting,
    /// 2-CCK wake-up latency before fetching next instruction
    Wakeup(u8),
}

/// Agnus Copper coprocessor state and execution engine
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Copper {
    /// First Copper list pointer (COP1LCH/COP1LCL, 18-bit Chip RAM address)
    pub cop1lc: u32,
    /// Second Copper list pointer (COP2LCH/COP2LCL, 18-bit Chip RAM address)
    pub cop2lc: u32,
    /// Active program counter
    pub cop_pc: u32,
    /// Instruction register 1 (IR1 / cop1ins)
    pub ir1: u16,
    /// Instruction register 2 (IR2 / cop2ins)
    pub ir2: u16,
    /// Current instruction latch (legacy alias for ir1)
    pub copins: u16,
    /// Copper control register ($02E)
    pub copcon: u16,
    /// Copper Danger mode flag (COPCON bit 1: allows writes to $DFF000..$DFF07E)
    pub cdang: bool,
    /// Copper DMA channel enabled via DMACON (COPEN bit 7 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// True if Copper is actively executing instructions
    pub is_running: bool,
    /// True if Copper is halted waiting for a beam position comparison
    pub is_waiting: bool,
    /// Active execution pipeline state
    pub state: CopperState,
}

impl Copper {
    /// Creates a new uninitialized Copper instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets Copper registers to power-on defaults
    pub fn reset(&mut self) {
        self.cop1lc = 0;
        self.cop2lc = 0;
        self.cop_pc = 0;
        self.ir1 = 0;
        self.ir2 = 0;
        self.copins = 0;
        self.copcon = 0;
        self.cdang = false;
        self.dma_enabled = false;
        self.is_running = false;
        self.is_waiting = false;
        self.state = CopperState::Idle;
    }

    /// Sets Copper DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if !enabled {
            self.is_running = false;
            self.is_waiting = false;
            self.state = CopperState::Idle;
        }
    }

    /// Sets COP1LC address latch
    #[inline]
    pub fn set_cop1lc(&mut self, addr: u32) {
        self.cop1lc = addr;
    }

    /// Sets COP2LC address latch
    #[inline]
    pub fn set_cop2lc(&mut self, addr: u32) {
        self.cop2lc = addr;
    }

    /// Restarts execution using Copper list 1 (COPJMP1 strobe)
    #[inline]
    pub fn restart_list1(&mut self) {
        self.cop_pc = self.cop1lc & 0x0007_FFFE;
        self.is_running = true;
        self.is_waiting = false;
        self.state = CopperState::FetchIR1(2);
    }

    /// Restarts execution using Copper list 2 (COPJMP2 strobe)
    #[inline]
    pub fn restart_list2(&mut self) {
        self.cop_pc = self.cop2lc & 0x0007_FFFE;
        self.is_running = true;
        self.is_waiting = false;
        self.state = CopperState::FetchIR1(2);
    }

    /// Returns true if the Copper is actively fetching instructions from Chip RAM (not idle or waiting)
    #[inline]
    pub fn is_active_fetching(&self) -> bool {
        self.dma_enabled && self.is_running && !self.is_waiting
    }

    /// Action method: triggers Copper restart on COP1LC address
    #[inline]
    pub fn strobe_jump1(&mut self, addr: u32) {
        self.cop1lc = addr;
        self.restart_list1();
    }

    /// Action method: triggers Copper restart on COP2LC address
    #[inline]
    pub fn strobe_jump2(&mut self, addr: u32) {
        self.cop2lc = addr;
        self.restart_list2();
    }

    /// Writes COPCON control register
    #[inline]
    pub fn set_copcon(&mut self, val: u16) {
        self.copcon = val;
        self.cdang = (val & 0x0002) != 0;
    }

    /// Evaluates the WAIT / SKIP beam position comparator
    #[inline]
    pub fn eval_comparator(&self, beam: BeamPosition, blitter_busy: bool) -> bool {
        let vpos_target = ((self.ir1 >> 8) & 0xFF) as u16;
        let vpos_mask = (((self.ir2 >> 8) & 0x7F) | 0x80) as u16;

        let cur_v = (beam.vpos & 0xFF) & vpos_mask;
        let tgt_v = vpos_target & vpos_mask;

        if cur_v < tgt_v {
            return false;
        }

        let bfd = (self.ir2 & 0x8000) == 0;
        let blitter_ok = !bfd || !blitter_busy;

        if cur_v > tgt_v {
            return blitter_ok;
        }

        // Vertical coordinates match: compare horizontal beam position
        let hpos_target = (self.ir1 & 0x00FE) as u16;
        let hpos_mask = (self.ir2 & 0x00FE) as u16;

        let cur_h = if beam.hpos < 0xE0 {
            beam.hpos.wrapping_add(2) & 0x00FE
        } else {
            beam.hpos.wrapping_sub(0xE0) & 0x00FE
        } & hpos_mask;
        let tgt_h = hpos_target & hpos_mask;

        (cur_h >= tgt_h) && blitter_ok
    }

    /// Decodes and executes the active two-word instruction pair (IR1, IR2)
    fn execute_instruction(
        &mut self,
        beam: BeamPosition,
        blitter_busy: bool,
    ) -> Option<(u16, u16)> {
        if (self.ir1 & 0x0001) == 0 {
            // MOVE instruction:
            let reg = self.ir1 & 0x01FE;
            let data = self.ir2;

            // Copper Danger mode / illegal register check:
            // When CDANG is 0, writes to registers below $080 halt Copper.
            // On OCS, writes below $040 halt Copper even if CDANG is set.
            let is_illegal = if self.cdang { reg < 0x040 } else { reg < 0x080 };

            if is_illegal {
                self.is_waiting = true;
                self.state = CopperState::Idle;
                None
            } else {
                self.state = CopperState::FetchIR1(2);
                self.is_waiting = false;
                Some((reg, data))
            }
        } else if (self.ir2 & 0x0001) == 0 {
            // WAIT instruction:
            if self.ir1 == 0xFFFF && (self.ir2 & 0xFFFE) == 0xFFFE {
                // Terminator WAIT $FFFF, $FFFE: halt until next VBlank
                self.is_waiting = true;
                self.state = CopperState::Idle;
                None
            } else {
                self.is_waiting = true;
                if beam.hpos % 2 == 0 && self.eval_comparator(beam, blitter_busy) {
                    self.is_waiting = false;
                    self.state = CopperState::FetchIR1(2);
                } else {
                    self.state = CopperState::Waiting;
                }
                None
            }
        } else {
            // SKIP instruction:
            self.is_waiting = false;
            if self.eval_comparator(beam, blitter_busy) {
                // Condition met: skip subsequent 32-bit instruction word pair
                self.cop_pc = self.cop_pc.wrapping_add(4) & 0x0007_FFFE;
            }
            self.state = CopperState::FetchIR1(2);
            None
        }
    }

    /// Advances Copper coprocessor state by 1 Color Clock observing current beam coordinates,
    /// blitter busy status, and Chip RAM contents.
    ///
    /// Returns `Some((register_offset, value))` if a MOVE instruction committed on this cycle.
    pub fn step_cck(
        &mut self,
        beam: BeamPosition,
        blitter_busy: bool,
        chip_ram: &[u8],
    ) -> Option<(u16, u16)> {
        // Automatic restart on Vertical Blank line 0 / HPOS 0
        if beam.vpos == 0 && beam.hpos == 0 && self.dma_enabled {
            self.restart_list1();
        }

        if !self.dma_enabled || !self.is_running {
            return None;
        }

        match self.state {
            CopperState::Idle => None,
            CopperState::FetchIR1(cck_left) => {
                if cck_left <= 1 {
                    self.ir1 = read_chip_ram_word(chip_ram, self.cop_pc);
                    self.copins = self.ir1;
                    self.cop_pc = self.cop_pc.wrapping_add(2) & 0x0007_FFFE;
                    self.state = CopperState::FetchIR2(2);
                } else {
                    self.state = CopperState::FetchIR1(cck_left.wrapping_sub(1));
                }
                None
            }
            CopperState::FetchIR2(cck_left) => {
                if cck_left <= 1 {
                    self.ir2 = read_chip_ram_word(chip_ram, self.cop_pc);
                    self.cop_pc = self.cop_pc.wrapping_add(2) & 0x0007_FFFE;
                    self.execute_instruction(beam, blitter_busy)
                } else {
                    self.state = CopperState::FetchIR2(cck_left.wrapping_sub(1));
                    None
                }
            }
            CopperState::Waiting => {
                self.is_waiting = true;
                if beam.hpos % 2 == 0 && self.eval_comparator(beam, blitter_busy) {
                    self.is_waiting = false;
                    self.state = CopperState::FetchIR1(2);
                }
                None
            }
            CopperState::Wakeup(cck_left) => {
                if cck_left <= 1 {
                    self.state = CopperState::FetchIR1(2);
                } else {
                    self.state = CopperState::Wakeup(cck_left.wrapping_sub(1));
                }
                None
            }
        }
    }
}

/// Reads a big-endian 16-bit word from Chip RAM with wrapping and bounds safety
#[inline]
fn read_chip_ram_word(chip_ram: &[u8], addr: u32) -> u16 {
    if chip_ram.is_empty() {
        return 0xFFFF;
    }
    let offset = (addr as usize) & (chip_ram.len().wrapping_sub(1));
    if offset + 1 < chip_ram.len() {
        u16::from_be_bytes([chip_ram[offset], chip_ram[offset + 1]])
    } else {
        0xFFFF
    }
}
