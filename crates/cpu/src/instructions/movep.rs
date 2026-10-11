//! MOVEP word/long transfers through alternate bytes at a displacement address.

use crate::micro::{common, ea, types::MicroStep};
use crate::{Cpu, CpuState};
use physical_memory::{AddressBus, BusResult};

pub fn alu_movep_w_dn_disp(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    state.micro.destination = (state.d_word(reg_src as usize) as u32) << 16;
    ea::ea_calc_dst_d16_an(state, reg_src, reg_dst);
}

pub fn alu_movep_l_dn_disp(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    state.micro.destination = state.d_long(reg_src as usize);
    ea::ea_calc_dst_d16_an(state, reg_src, reg_dst);
}

pub fn alu_movep_w_disp_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.set_d_word(reg_dst as usize, state.micro.source as u16);
}

pub fn alu_movep_l_disp_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.set_d_long(reg_dst as usize, state.micro.source);
}

/// CCK1: Appends one byte and advances the spaced address only after a successful read.
fn step_movep_read_byte(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
    let addr = cpu.state.micro.ea_addr;
    match bus.read_byte(addr & 0x00FF_FFFF) {
        BusResult::WaitState => BusResult::WaitState,
        BusResult::Ready(data) => {
            cpu.state.micro.source = (cpu.state.micro.source << 8) | data as u32;
            cpu.state.micro.ea_addr = addr.wrapping_add(2);
            BusResult::Ready(())
        }
    }
}

/// CCK2: Transfers the high staged byte, retaining address and data while waiting.
fn step_movep_write_byte(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
    let addr = cpu.state.micro.ea_addr;
    let data = (cpu.state.micro.destination >> 24) as u8;
    match bus.write_byte(addr & 0x00FF_FFFF, data) {
        BusResult::WaitState => BusResult::WaitState,
        BusResult::Ready(()) => {
            cpu.state.micro.destination <<= 8;
            cpu.state.micro.ea_addr = addr.wrapping_add(2);
            BusResult::Ready(())
        }
    }
}

const READ_BYTE: MicroStep = MicroStep::cck(step_movep_read_byte);
const WRITE_BYTE: MicroStep = MicroStep::cck(step_movep_write_byte);
const FETCH_DISP_READ: MicroStep = MicroStep {
    bus_fn: Some(Cpu::step_fetch_extension_read),
    alu_fn: Some(ea::ea_calc_src_d16_an),
    base_clocks: 2,
};
const READ_IDLE_COMMIT_W: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_movep_w_disp_dn),
    base_clocks: 2,
};
const READ_IDLE_COMMIT_L: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_movep_l_disp_dn),
    base_clocks: 2,
};

pub static STEPS_MOVEP_W_DISP_DN: [MicroStep; 8] = [
    FETCH_DISP_READ,
    common::FETCH_EXT_FINISH,
    READ_BYTE,
    common::BUS_READ_IDLE,
    READ_BYTE,
    READ_IDLE_COMMIT_W,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEP_L_DISP_DN: [MicroStep; 12] = [
    FETCH_DISP_READ,
    common::FETCH_EXT_FINISH,
    READ_BYTE,
    common::BUS_READ_IDLE,
    READ_BYTE,
    common::BUS_READ_IDLE,
    READ_BYTE,
    common::BUS_READ_IDLE,
    READ_BYTE,
    READ_IDLE_COMMIT_L,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEP_W_DN_DISP: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_movep_w_dn_disp),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEP_L_DN_DISP: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_movep_l_dn_disp),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::BUS_WRITE_IDLE,
    WRITE_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub const fn decode_movep_steps(opmode: u8) -> &'static [MicroStep] {
    match opmode {
        0 => &STEPS_MOVEP_W_DISP_DN,
        1 => &STEPS_MOVEP_L_DISP_DN,
        2 => &STEPS_MOVEP_W_DN_DISP,
        _ => &STEPS_MOVEP_L_DN_DISP,
    }
}
