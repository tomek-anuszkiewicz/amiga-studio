//! M68000 Shifts and Rotates (LSL, LSR, ASL, ASR)

use crate::addressing::Size;
use crate::state::CpuState;

pub fn execute_lsl(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let count = count % 64;
    let width = size.byte_count() * 8;
    let mask = match size {
        Size::Byte => 0xFF,
        Size::Word => 0xFFFF,
        Size::Long => 0xFFFF_FFFF,
    };
    let mut v = val & mask;

    if count == 0 {
        state.set_c(false);
        state.set_v(false);
        update_nz(state, v, size);
        return (val & !mask) | v;
    }

    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & (1 << (width - 1))) != 0;
        v = (v << 1) & mask;
    }

    state.set_x(last_out);
    state.set_c(last_out);
    state.set_v(false);
    update_nz(state, v, size);

    (val & !mask) | v
}

pub fn execute_lsr(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let count = count % 64;
    let mask = match size {
        Size::Byte => 0xFF,
        Size::Word => 0xFFFF,
        Size::Long => 0xFFFF_FFFF,
    };
    let mut v = val & mask;

    if count == 0 {
        state.set_c(false);
        state.set_v(false);
        update_nz(state, v, size);
        return (val & !mask) | v;
    }

    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }

    state.set_x(last_out);
    state.set_c(last_out);
    state.set_v(false);
    update_nz(state, v, size);

    (val & !mask) | v
}

pub fn execute_asl(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let count = count % 64;
    let width = size.byte_count() * 8;
    let msb_mask = 1 << (width - 1);
    let mask = match size {
        Size::Byte => 0xFF,
        Size::Word => 0xFFFF,
        Size::Long => 0xFFFF_FFFF,
    };
    let mut v = val & mask;

    if count == 0 {
        state.set_c(false);
        state.set_v(false);
        update_nz(state, v, size);
        return (val & !mask) | v;
    }

    let mut last_out = false;
    let mut overflow = false;
    for _ in 0..count {
        let old_msb = (v & msb_mask) != 0;
        last_out = old_msb;
        v = (v << 1) & mask;
        let new_msb = (v & msb_mask) != 0;
        if old_msb != new_msb {
            overflow = true;
        }
    }

    state.set_x(last_out);
    state.set_c(last_out);
    state.set_v(overflow);
    update_nz(state, v, size);

    (val & !mask) | v
}

pub fn execute_asr(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let count = count % 64;
    let width = size.byte_count() * 8;
    let msb_mask = 1 << (width - 1);
    let mask = match size {
        Size::Byte => 0xFF,
        Size::Word => 0xFFFF,
        Size::Long => 0xFFFF_FFFF,
    };
    let mut v = val & mask;

    if count == 0 {
        state.set_c(false);
        state.set_v(false);
        update_nz(state, v, size);
        return (val & !mask) | v;
    }

    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        let sign_bit = v & msb_mask;
        v = (v >> 1) | sign_bit;
    }

    state.set_x(last_out);
    state.set_c(last_out);
    state.set_v(false);
    update_nz(state, v, size);

    (val & !mask) | v
}

#[inline]
fn update_nz(state: &mut CpuState, v: u32, size: Size) {
    let (n, z) = match size {
        Size::Byte => (((v as u8) & 0x80) != 0, (v as u8) == 0),
        Size::Word => (((v as u16) & 0x8000) != 0, (v as u16) == 0),
        Size::Long => ((v & 0x8000_0000) != 0, v == 0),
    };
    state.set_n(n);
    state.set_z(z);
}
