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
pub const SR_MASK: u16 = SR_T | SR_S | SR_I_MASK | CCR_ALL;

/// Default Status Register on CPU Reset (Supervisor bit set, Interrupt Priority Mask Level 7)
pub const SR_RESET_DEFAULT: u16 = SR_S | SR_I_MASK; // 0x2700

/// M68000 Function Code lines (FC0-FC2)
pub mod function_code {
    pub const USER_DATA: u8 = 1;
    pub const USER_PROGRAM: u8 = 2;
    pub const SUPERVISOR_DATA: u8 = 5;
    pub const SUPERVISOR_PROGRAM: u8 = 6;
    pub const CPU_SPACE: u8 = 7;
}

/// Standard Motorola 68000 exception vector numbers (each vector occupies 4 bytes at vector * 4)
pub mod vector {
    pub const RESET_SSP: u32 = 0;
    pub const RESET_PC: u32 = 1;
    pub const BUS_ERROR: u32 = 2;
    pub const ADDRESS_ERROR: u32 = 3;
    pub const ILLEGAL_INSTRUCTION: u32 = 4;
    pub const ZERO_DIVIDE: u32 = 5;
    pub const CHK: u32 = 6;
    pub const TRAPV: u32 = 7;
    pub const PRIVILEGE_VIOLATION: u32 = 8;
    pub const TRACE: u32 = 9;
    pub const LINE_1010: u32 = 10;
    pub const LINE_1111: u32 = 11;
    pub const UNINITIALIZED_INTERRUPT: u32 = 15;
    pub const SPURIOUS_INTERRUPT: u32 = 24;
    pub const AUTOVECTOR_BASE: u32 = 24;
    pub const TRAP_BASE: u32 = 32;

    /// Converts an exception vector number to its 24-bit physical vector table address
    #[inline(always)]
    pub const fn addr(vec: u32) -> u32 {
        vec * 4
    }
}

/// Complete register set and state snapshot for the Motorola 68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuState {
    /// Data Registers D0-D7 (32-bit each, private)
    d: [u32; 8],

    /// Address Registers A0-A7 (32-bit each, private). A7 holds the active stack pointer (USP or SSP).
    a: [u32; 8],

    /// User Stack Pointer (stored A7 when Supervisor bit S = 0)
    pub usp: u32,

    /// Supervisor Stack Pointer (stored A7 when Supervisor bit S = 1)
    pub ssp: u32,

    /// Program Counter (24-bit physical addressing on MC68000)
    pub pc: u32,

    /// Status Register (16-bit):
    /// - System Byte (Bits 8-15): Trace (T, bit 15), Supervisor (S, bit 13), Interrupt Mask (I2-I0, bits 10-8)
    /// - User Byte / CCR (Bits 0-7): Extend (X, bit 4), Negative (N, bit 3), Zero (Z, bit 2), Overflow (V, bit 1), Carry (C, bit 0)
    pub sr: u16,

    /// Lookahead Prefetch Queue Register (models 16-bit hardware IR holding extension word or next opcode)
    pub prefetch: u16,

    /// Instruction Register (active 16-bit instruction being executed)
    pub ir: u16,

    /// Interrupt Priority Level pending from motherboard/custom chips (IPL 1-6)
    pub ipl: u8,

    /// Program counter at the start of the current instruction (used for PC-relative EA & exceptions)
    #[serde(default)]
    pub instruction_pc: u32,

    /// CPU Stopped state (halted until higher interrupt arrives)
    pub stopped: bool,

    /// CPU Halted state (fatal double bus fault or physical HALT line)
    pub halted: bool,

    /// Hardware RESET pin active state
    #[serde(default)]
    pub reset_line_asserted: bool,

    /// Cycle-exact micro-operation engine state (not saved in basic snapshot)
    #[serde(default)]
    pub micro: crate::micro::CpuMicroState,

    /// Master cycle counter
    #[serde(default)]
    pub cycle_counter: u64,
}

impl Default for CpuState {
    fn default() -> Self {
        Self {
            d: [0; 8],
            a: [0; 8],
            usp: 0,
            ssp: 0,
            pc: 0,
            sr: SR_RESET_DEFAULT,
            prefetch: 0,
            ir: 0,
            ipl: 0,
            instruction_pc: 0,
            stopped: false,
            halted: false,
            reset_line_asserted: false,
            micro: crate::micro::CpuMicroState::default(),
            cycle_counter: 0,
        }
    }
}

impl CpuState {
    // --- Data Register (Dn) Accessors ---

    /// Reads lowest 8 bits of data register Dn
    #[inline(always)]
    pub fn d_byte(&self, reg: usize) -> u8 {
        self.d[reg] as u8
    }

    /// Writes lowest 8 bits of data register Dn, preserving upper 24 bits
    #[inline(always)]
    pub fn set_d_byte(&mut self, reg: usize, val: u8) {
        self.d[reg] = (self.d[reg] & 0xFFFF_FF00) | (val as u32);
    }

    /// Reads lowest 16 bits of data register Dn
    #[inline(always)]
    pub fn d_word(&self, reg: usize) -> u16 {
        self.d[reg] as u16
    }

    /// Writes lowest 16 bits of data register Dn, preserving upper 16 bits
    #[inline(always)]
    pub fn set_d_word(&mut self, reg: usize, val: u16) {
        self.d[reg] = (self.d[reg] & 0xFFFF_0000) | (val as u32);
    }

    /// Reads full 32-bit value of data register Dn
    #[inline(always)]
    pub fn d_long(&self, reg: usize) -> u32 {
        self.d[reg]
    }

    /// Writes full 32-bit value of data register Dn
    #[inline(always)]
    pub fn set_d_long(&mut self, reg: usize, val: u32) {
        self.d[reg] = val;
    }

    // --- Address Register (An) Accessors ---
    // Note: Byte accessors are intentionally omitted because byte operations on An are illegal in the M68000 ISA.

    /// Reads lowest 16 bits of address register An
    #[inline(always)]
    pub fn a_word(&self, reg: usize) -> u16 {
        self.read_a(reg) as u16
    }

    /// Reads full 32-bit value of address register An
    #[inline(always)]
    pub fn a_long(&self, reg: usize) -> u32 {
        self.read_a(reg)
    }

    /// Writes full 32-bit value of address register An
    #[inline(always)]
    pub fn set_a_long(&mut self, reg: usize, val: u32) {
        self.write_a(reg, val);
    }

    // --- Full Array Accessors (Test Harness & State Snapshots) ---

    /// Read-only slice view of all 8 data registers D0-D7 (used in test runners, debugger, and state comparison)
    #[inline(always)]
    pub fn d_regs(&self) -> &[u32; 8] {
        &self.d
    }

    /// Read-only slice view of all 8 address registers A0-A7 (used in test runners, debugger, and state comparison)
    #[inline(always)]
    pub fn a_regs(&self) -> &[u32; 8] {
        &self.a
    }

    /// Clears data registers D0-D7, address registers A0-A7, and USP to zero
    #[inline]
    pub fn clear_registers(&mut self) {
        self.d.fill(0);
        self.a.fill(0);
        self.usp = 0;
    }

    /// Read address register by index (0-7 returns A0-A7)
    #[inline(always)]
    pub fn read_a(&self, idx: usize) -> u32 {
        self.a[idx]
    }

    /// Write address register by index (0-7 writes A0-A7)
    #[inline(always)]
    pub fn write_a(&mut self, idx: usize, val: u32) {
        self.a[idx] = val;
    }

    /// Transitions or sets supervisor mode, swapping active A7 with stored USP/SSP if privilege changes
    #[inline]
    pub fn set_supervisor(&mut self, supervisor: bool) {
        let is_super = (self.sr & SR_S) != 0;
        if is_super == supervisor {
            return;
        }
        if supervisor {
            self.sr |= SR_S;
            self.usp = self.a[7];
            self.a[7] = self.ssp;
        } else {
            self.sr &= !SR_S;
            self.ssp = self.a[7];
            self.a[7] = self.usp;
        }
    }

    /// Updates Status Register (SR) and swaps active A7 with stored USP/SSP if the Supervisor bit changes
    #[inline]
    pub fn set_sr(&mut self, new_sr: u16) {
        let masked_sr = new_sr & SR_MASK;
        let old_s = (self.sr & SR_S) != 0;
        let new_s = (masked_sr & SR_S) != 0;
        self.sr = masked_sr;
        if old_s != new_s {
            if new_s {
                self.usp = self.a[7];
                self.a[7] = self.ssp;
            } else {
                self.ssp = self.a[7];
                self.a[7] = self.usp;
            }
        }
    }

    /// Flushes the live active stack pointer (`a[7]`) into `ssp` (if supervisor) or `usp` (if user)
    #[inline]
    pub fn sync_stack_pointers(&mut self) {
        if (self.sr & SR_S) != 0 {
            self.ssp = self.a[7];
        } else {
            self.usp = self.a[7];
        }
    }

    /// Returns true if CPU is running in Supervisor mode
    #[inline]
    pub fn is_supervisor(&self) -> bool {
        (self.sr & SR_S) != 0
    }

    /// Returns the 3-bit interrupt priority mask from SR (bits 8..=10, levels 0..=7)
    #[inline(always)]
    pub fn interrupt_mask(&self) -> u8 {
        ((self.sr >> 8) & 0x07) as u8
    }

    /// Sets the 3-bit interrupt priority mask in SR (bits 8..=10)
    #[inline(always)]
    pub fn set_interrupt_mask(&mut self, mask: u8) {
        self.sr = (self.sr & !0x0700) | (((mask as u16) & 0x07) << 8);
    }

    /// Checks whether the currently sampled `ipl` qualifies as a pending interrupt
    /// according to M68000 priority rules (ipl > mask || ipl == 7 for NMI).
    #[inline(always)]
    pub fn is_interrupt_pending(&self) -> Option<u8> {
        if (self.ipl > self.interrupt_mask() && self.ipl > 0) || self.ipl == 7 {
            Some(self.ipl)
        } else {
            None
        }
    }

    /// Returns the 8-bit Condition Code Register (lower byte of SR, bits 0..=4)
    #[inline(always)]
    pub fn ccr(&self) -> u8 {
        (self.sr & 0x001F) as u8
    }

    /// Sets the Condition Code Register (lower byte of SR, bits 0..=4)
    #[inline(always)]
    pub fn set_ccr(&mut self, ccr: u8) {
        self.sr = (self.sr & !0x001F) | ((ccr as u16) & 0x001F);
    }

    // --- Branchless Multi-Flag CCR Setters (Host Hardware Efficiency) ---

    /// Sets all 5 flags (X, N, Z, V, C) simultaneously in a single 16-bit operation without branching.
    /// Used by: ADD, ADDI, ADDQ, SUB, SUBI, SUBQ, NEG, arithmetic shifts, etc.
    #[inline(always)]
    pub fn set_ccr_xnzvc(&mut self, x: bool, n: bool, z: bool, v: bool, c: bool) {
        let flags = ((x as u16) << 4)
            | ((n as u16) << 3)
            | ((z as u16) << 2)
            | ((v as u16) << 1)
            | (c as u16);
        self.sr = (self.sr & !CCR_ALL) | flags;
    }

    /// Sets N, Z, V, C simultaneously without branching, strictly preserving Extend (X).
    /// Used by: CMP, CMPI, CMPA, CMPM, CHK.
    #[inline(always)]
    pub fn set_ccr_nzvc(&mut self, n: bool, z: bool, v: bool, c: bool) {
        let flags = ((n as u16) << 3) | ((z as u16) << 2) | ((v as u16) << 1) | (c as u16);
        self.sr = (self.sr & !0x000F) | flags;
    }

    /// Sets N and Z, clears V=0 and C=0, and strictly preserves Extend (X).
    /// Used by: ORI, ANDI, EORI, OR, AND, EOR, NOT, MOVE, MOVEQ, CLR, EXT, TST, TAS, SWAP.
    #[inline(always)]
    pub fn set_ccr_nz_clear_vc(&mut self, n: bool, z: bool) {
        let flags = ((n as u16) << 3) | ((z as u16) << 2);
        self.sr = (self.sr & !0x000F) | flags;
    }

    /// Sets Zero (Z) flag branchlessly, strictly preserving X, N, V, C.
    /// Used by: BTST, BSET, BCLR, BCHG.
    #[inline(always)]
    pub fn set_ccr_z_only(&mut self, z: bool) {
        let flag = (z as u16) << 2;
        self.sr = (self.sr & !CCR_Z) | flag;
    }

    /// Sets Overflow (V=1), clears Carry (C=0), and strictly preserves X, N, Z.
    /// Used by: DIVU and DIVS on division overflow (68000 hardware behavior).
    #[inline(always)]
    pub fn set_ccr_v_clear_c(&mut self) {
        self.sr = (self.sr & !0x0003) | 0x0002;
    }

    // --- Condition Code Helpers ---

    #[inline(always)]
    pub fn is_x(&self) -> bool {
        (self.sr & CCR_X) != 0
    }

    #[inline(always)]
    pub fn is_n(&self) -> bool {
        (self.sr & CCR_N) != 0
    }

    #[inline(always)]
    pub fn is_z(&self) -> bool {
        (self.sr & CCR_Z) != 0
    }

    #[inline(always)]
    pub fn is_v(&self) -> bool {
        (self.sr & CCR_V) != 0
    }

    #[inline(always)]
    pub fn is_c(&self) -> bool {
        (self.sr & CCR_C) != 0
    }

    /// Evaluates M68000 branch/conditional tests (conditions 0000..1111)
    #[inline(always)]
    pub fn eval_condition(&self, cond: u8) -> bool {
        let c = self.is_c();
        let v = self.is_v();
        let z = self.is_z();
        let n = self.is_n();

        match cond & 0x0F {
            0x00 => true,                        // True (T) / BRA
            0x01 => false,                       // False (F)
            0x02 => !c && !z,                    // High (HI)
            0x03 => c || z,                      // Low or Same (LS)
            0x04 => !c,                          // Carry Clear (CC / HS)
            0x05 => c,                           // Carry Set (CS / LO)
            0x06 => !z,                          // Not Equal (NE)
            0x07 => z,                           // Equal (EQ)
            0x08 => !v,                          // Overflow Clear (VC)
            0x09 => v,                           // Overflow Set (VS)
            0x0A => !n,                          // Plus (PL)
            0x0B => n,                           // Minus (MI)
            0x0C => (n && v) || (!n && !v),      // Greater or Equal (GE)
            0x0D => (n && !v) || (!n && v),      // Less Than (LT)
            0x0E => (n == v) && !z,              // Greater Than (GT)
            0x0F => z || (n && !v) || (!n && v), // Less or Equal (LE)
            _ => false,
        }
    }

    /// Returns total elapsed CPU clock cycles since reset
    #[inline(always)]
    pub fn cycle_counter(&self) -> u64 {
        self.cycle_counter
    }

    /// Resets the cycle counter to zero
    #[inline(always)]
    pub fn reset_cycle_counter(&mut self) {
        self.cycle_counter = 0;
    }
}
