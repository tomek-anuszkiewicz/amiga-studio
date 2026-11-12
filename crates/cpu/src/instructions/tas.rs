//! TAS (Test and Set an Operand) Instruction Handler
//!
//! Tests a destination byte, sets Condition Codes (N, Z, V=0, C=0, X unaffected),
//! and sets bit 7 (MSB) of the byte (`byte | BIT_BYTE_SIGN`).
//!
//! Valid EA modes: Data alterable (Dn, (An), (An)+, -(An), (d16,An), (d8,An,Xn), (xxx).W, (xxx).L).
//! Execution time: 4 clocks (Dn), 14..22 clocks (memory).

use super::BIT_BYTE_SIGN;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;
use physical_memory::{AddressBus, BusResult};

impl Cpu {
    /// CCK2: identifies the TAS write so the platform can apply its RMW erratum.
    pub fn step_bus_write_tas_byte(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        bus.write_tas_byte(
            self.state.micro().ea_addr & physical_memory::MASK_PHYSICAL_ADDRESS,
            self.state.micro().destination as u8,
        )
    }
}

const BIT_TAS_SET: u8 = 0x80;

const WRITE_TAS_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_write_tas_byte);

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_tas_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let orig = state.d_long(reg_dst as usize);
    let val = (orig & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val & BIT_BYTE_SIGN) != 0, val == 0);
    let new_byte = val | BIT_TAS_SET;
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (new_byte as u32));
}

pub static ALU_TAS_DN: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_tas_dn),
    base_clocks: 0,
};

pub fn alu_tas_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.micro().source & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val & BIT_BYTE_SIGN) != 0, val == 0);
    state.micro_mut().destination = (val | BIT_TAS_SET) as u32;
}

pub static ALU_TAS_MEM_2CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_tas_mem),
    base_clocks: MicroStep::CCK_CLOCKS,
};

// ============================================================================
// Static Micro-Step Slices for TAS
// ============================================================================

pub static STEPS_TAS_DN: [MicroStep; 3] = [
    ALU_TAS_DN,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AI: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_PI: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pi_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_PD: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_D16: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_IDX: [MicroStep; 10] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AW: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AL: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    WRITE_TAS_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Opcode Decoding Helper
// ============================================================================

pub const fn decode_tas_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::DATA_REGISTER => Some(&STEPS_TAS_DN),
        mode::INDIRECT => Some(&STEPS_TAS_AI),
        mode::POSTINCREMENT => Some(&STEPS_TAS_PI),
        mode::PREDECREMENT => Some(&STEPS_TAS_PD),
        mode::DISPLACEMENT => Some(&STEPS_TAS_D16),
        mode::INDEXED => Some(&STEPS_TAS_IDX),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_TAS_AW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_TAS_AL),
            _ => None,
        },
        _ => None,
    }
}
