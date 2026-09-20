//! ORI / ANDI / EORI to CCR and SR Handlers
//!
//! Implements cycle-exact 20-clock (10 CCK) micro-step sequences for immediate
//! bitwise operations targeting CCR (unprivileged) and SR (privileged).

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Privilege Check Callback
// ============================================================================

pub fn alu_check_privilege(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_CHECK_PRIVILEGE: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_check_privilege),
    base_clocks: 0,
};

// ============================================================================
// ORI to CCR / SR Handlers (20 Clocks / 10 CCKs)
// ============================================================================

pub fn alu_ori_to_ccr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let imm = (state.micro.source & 0x1F) as u8;
    state.set_ccr(state.ccr() | imm);
}

pub static ALU_ORI_TO_CCR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_ori_to_ccr),
    base_clocks: 8,
};

pub static STEPS_ORI_TO_CCR: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_ORI_TO_CCR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub fn alu_ori_to_sr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_sr(state.sr | (state.micro.source as u16));
}

pub static ALU_ORI_TO_SR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_ori_to_sr),
    base_clocks: 8,
};

pub static STEPS_ORI_TO_SR: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_ORI_TO_SR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// ANDI to CCR / SR Handlers (20 Clocks / 10 CCKs)
// ============================================================================

pub fn alu_andi_to_ccr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let imm = (state.micro.source & 0x1F) as u8;
    state.set_ccr(state.ccr() & imm);
}

pub static ALU_ANDI_TO_CCR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_andi_to_ccr),
    base_clocks: 8,
};

pub static STEPS_ANDI_TO_CCR: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_ANDI_TO_CCR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub fn alu_andi_to_sr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_sr(state.sr & (state.micro.source as u16));
}

pub static ALU_ANDI_TO_SR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_andi_to_sr),
    base_clocks: 8,
};

pub static STEPS_ANDI_TO_SR: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_ANDI_TO_SR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// EORI to CCR / SR Handlers (20 Clocks / 10 CCKs)
// ============================================================================

pub fn alu_eori_to_ccr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let imm = (state.micro.source & 0x1F) as u8;
    state.set_ccr(state.ccr() ^ imm);
}

pub static ALU_EORI_TO_CCR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_eori_to_ccr),
    base_clocks: 8,
};

pub static STEPS_EORI_TO_CCR: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_EORI_TO_CCR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub fn alu_eori_to_sr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_sr(state.sr ^ (state.micro.source as u16));
}

pub static ALU_EORI_TO_SR_8CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_eori_to_sr),
    base_clocks: 8,
};

pub static STEPS_EORI_TO_SR: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_EORI_TO_SR_8CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];
