//! Motorola 68000 CPU State, Registers, and Status Register / CCR flags

use serde::{Deserialize, Serialize};

pub const SR_T: u16 = 0x8000;
pub const SR_S: u16 = 0x2000;
pub const SR_I_MASK: u16 = 0x0700;
pub const CCR_X: u16 = 0x0010;
pub const CCR_N: u16 = 0x0008;
pub const CCR_Z: u16 = 0x0004;
pub const CCR_V: u16 = 0x0002;
pub const CCR_C: u16 = 0x0001;
pub const CCR_ALL: u16 = 0x001F;

/// Complete register set and state snapshot for the Motorola 68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuState {
    /// Data Registers D0-D7 (32-bit each)
    pub d: [u32; 8],

    /// Address Registers A0-A6 (32-bit each)
    pub a: [u32; 7],

    /// User Stack Pointer (active A7 when Supervisor bit S = 0)
    pub usp: u32,

    /// Supervisor Stack Pointer (active A7 when Supervisor bit S = 1)
    pub ssp: u32,

    /// Program Counter (24-bit physical addressing on MC68000)
    pub pc: u32,

    /// Status Register (16-bit):
    /// - System Byte (Bits 8-15): Trace (T, bit 15), Supervisor (S, bit 13), Interrupt Mask (I2-I0, bits 10-8)
    /// - User Byte / CCR (Bits 0-7): Extend (X, bit 4), Negative (N, bit 3), Zero (Z, bit 2), Overflow (V, bit 1), Carry (C, bit 0)
    pub sr: u16,

    /// Internal Prefetch Queue [IRC (Capture), IRD (Decode)]
    pub prefetch: [u16; 2],

    /// Current Instruction Register (holds opcode being decoded/executed)
    pub ir: u16,

    /// Sub-cycle execution phase / step index within current instruction (CCK1/CCK2)
    pub step: u16,

    /// Sampled Interrupt Priority Level (0..7) driven from outside
    pub ipl: u8,

    /// Execution control flags
    pub stopped: bool,
    pub halted: bool,
}

impl Default for CpuState {
    fn default() -> Self {
        Self {
            d: [0; 8],
            a: [0; 7],
            usp: 0,
            ssp: 0,
            pc: 0,
            sr: 0x2700, // Supervisor mode, Interrupt mask 7
            prefetch: [0; 2],
            ir: 0,
            step: 0,
            ipl: 0,
            stopped: false,
            halted: false,
        }
    }
}

impl CpuState {
    /// Returns the currently active stack pointer (A7) based on the Supervisor flag
    #[inline]
    pub fn a7(&self) -> u32 {
        if (self.sr & SR_S) != 0 {
            self.ssp
        } else {
            self.usp
        }
    }

    /// Sets the currently active stack pointer (A7) based on the Supervisor flag
    #[inline]
    pub fn set_a7(&mut self, val: u32) {
        if (self.sr & SR_S) != 0 {
            self.ssp = val;
        } else {
            self.usp = val;
        }
    }

    /// Read address register by index (0-6 returns A0-A6, 7 returns active A7)
    #[inline]
    pub fn read_a(&self, idx: usize) -> u32 {
        if idx < 7 {
            self.a[idx]
        } else {
            self.a7()
        }
    }

    /// Write address register by index (0-6 writes A0-A6, 7 writes active A7)
    #[inline]
    pub fn write_a(&mut self, idx: usize, val: u32) {
        if idx < 7 {
            self.a[idx] = val;
        } else {
            self.set_a7(val);
        }
    }

    /// Returns true if CPU is running in Supervisor mode
    #[inline]
    pub fn is_supervisor(&self) -> bool {
        (self.sr & SR_S) != 0
    }

    // --- Condition Code Helpers ---

    #[inline]
    pub fn get_x(&self) -> bool {
        (self.sr & CCR_X) != 0
    }

    #[inline]
    pub fn set_x(&mut self, val: bool) {
        if val {
            self.sr |= CCR_X;
        } else {
            self.sr &= !CCR_X;
        }
    }

    #[inline]
    pub fn get_n(&self) -> bool {
        (self.sr & CCR_N) != 0
    }

    #[inline]
    pub fn set_n(&mut self, val: bool) {
        if val {
            self.sr |= CCR_N;
        } else {
            self.sr &= !CCR_N;
        }
    }

    #[inline]
    pub fn get_z(&self) -> bool {
        (self.sr & CCR_Z) != 0
    }

    #[inline]
    pub fn set_z(&mut self, val: bool) {
        if val {
            self.sr |= CCR_Z;
        } else {
            self.sr &= !CCR_Z;
        }
    }

    #[inline]
    pub fn get_v(&self) -> bool {
        (self.sr & CCR_V) != 0
    }

    #[inline]
    pub fn set_v(&mut self, val: bool) {
        if val {
            self.sr |= CCR_V;
        } else {
            self.sr &= !CCR_V;
        }
    }

    #[inline]
    pub fn get_c(&self) -> bool {
        (self.sr & CCR_C) != 0
    }

    #[inline]
    pub fn set_c(&mut self, val: bool) {
        if val {
            self.sr |= CCR_C;
        } else {
            self.sr &= !CCR_C;
        }
    }

    /// Evaluates M68000 branch/conditional tests (conditions 0000..1111)
    pub fn eval_condition(&self, cond: u8) -> bool {
        let c = self.get_c();
        let v = self.get_v();
        let z = self.get_z();
        let n = self.get_n();

        match cond & 0x0F {
            0x00 => true,               // True (T) / BRA
            0x01 => false,              // False (F)
            0x02 => !c && !z,           // High (HI)
            0x03 => c || z,             // Low or Same (LS)
            0x04 => !c,                 // Carry Clear (CC / HS)
            0x05 => c,                  // Carry Set (CS / LO)
            0x06 => !z,                 // Not Equal (NE)
            0x07 => z,                  // Equal (EQ)
            0x08 => !v,                 // Overflow Clear (VC)
            0x09 => v,                  // Overflow Set (VS)
            0x0A => !n,                 // Plus (PL)
            0x0B => n,                  // Minus (MI)
            0x0C => (n && v) || (!n && !v), // Greater or Equal (GE)
            0x0D => (n && !v) || (!n && v), // Less Than (LT)
            0x0E => (n && v && !z) || (!n && !v && !z), // Greater Than (GT)
            0x0F => z || (n && !v) || (!n && v),        // Less or Equal (LE)
            _ => unreachable!(),
        }
    }
}
