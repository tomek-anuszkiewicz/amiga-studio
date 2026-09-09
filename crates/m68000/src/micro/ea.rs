//! Effective Address (EA) Micro-Step Calculation Callbacks & Sequences
//!
//! Provides compile-time address calculation functions for all M68000
//! addressing modes without cascaded runtime matching.

use crate::state::CpuState;

/// Reads and sign-extends the index register (Xn) specified in a brief extension word
#[inline(always)]
pub fn read_index_reg(state: &CpuState, ext: u16) -> u32 {
    let is_address = (ext & 0x8000) != 0;
    let reg_idx = ((ext >> 12) & 7) as usize;
    let is_long = (ext & 0x0800) != 0;
    let raw = if is_address {
        state.read_a(reg_idx)
    } else {
        state.d_long(reg_idx)
    };
    if is_long {
        raw
    } else {
        (raw as i16 as i32) as u32
    }
}

// ============================================================================
// Source EA Calculations (reg_src: 0..7)
// ============================================================================

/// Address Register Indirect: (An) -> ea_addr = An
#[inline(always)]
pub fn ea_calc_src_ai(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.read_a(reg_src as usize);
}

/// Address Register Indirect with Postincrement (Byte): (An)+ -> ea_addr = An, An += (2 if A7 else 1)
#[inline(always)]
pub fn ea_calc_src_pi_b(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize);
    state.micro.ea_addr = an;
    let inc = if reg_src == 7 { 2 } else { 1 };
    state.write_a(reg_src as usize, an.wrapping_add(inc));
}

/// Address Register Indirect with Postincrement (Word): (An)+ -> ea_addr = An, An += 2
#[inline(always)]
pub fn ea_calc_src_pi_w(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize);
    state.micro.ea_addr = an;
    state.write_a(reg_src as usize, an.wrapping_add(2));
}

/// Address Register Indirect with Postincrement (Long): (An)+ -> ea_addr = An, An += 4
#[inline(always)]
pub fn ea_calc_src_pi_l(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize);
    state.micro.ea_addr = an;
    state.write_a(reg_src as usize, an.wrapping_add(4));
}

/// Address Register Indirect with Predecrement (Byte): -(An) -> An -= (2 if A7 else 1), ea_addr = An
#[inline(always)]
pub fn ea_calc_src_pd_b(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let dec = if reg_src == 7 { 2 } else { 1 };
    let an = state.read_a(reg_src as usize).wrapping_sub(dec);
    state.write_a(reg_src as usize, an);
    state.micro.ea_addr = an;
}

/// Address Register Indirect with Predecrement (Word): -(An) -> An -= 2, ea_addr = An
#[inline(always)]
pub fn ea_calc_src_pd_w(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize).wrapping_sub(2);
    state.write_a(reg_src as usize, an);
    state.micro.ea_addr = an;
}

/// Address Register Indirect with Predecrement (Long): -(An) -> An -= 4, ea_addr = An
#[inline(always)]
pub fn ea_calc_src_pd_l(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize).wrapping_sub(4);
    state.write_a(reg_src as usize, an);
    state.micro.ea_addr = an;
}

/// Address Register Indirect with Displacement: (d16, An) -> ea_addr = An + disp16
#[inline(always)]
pub fn ea_calc_src_d16_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    state.micro.ea_addr = state.read_a(reg_src as usize).wrapping_add(disp as u32);
}

/// Address Register Indirect with Index: (d8, An, Xn) -> ea_addr = An + Xn + disp8
#[inline(always)]
pub fn ea_calc_src_idx_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = read_index_reg(state, ext);
    let an = state.read_a(reg_src as usize);
    state.micro.ea_addr = an.wrapping_add(xn).wrapping_add(disp8 as u32);
}

/// Absolute Short: (xxx).W -> ea_addr = sign_extend(word)
#[inline(always)]
pub fn ea_calc_absw(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let word = state.prefetch[0] as i16 as i32 as u32;
    state.micro.ea_addr = word;
}

/// Absolute Long Cycle 1: High word -> latch in ea_high
#[inline(always)]
pub fn ea_calc_absl_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_high = (state.prefetch[0] as u32) << 16;
}

/// Absolute Long Cycle 2: Low word -> assemble into ea_addr
#[inline(always)]
pub fn ea_calc_absl_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.micro.ea_high | (state.prefetch[0] as u32);
}

/// Immediate Long Cycle 1: High word -> latch in source
#[inline(always)]
pub fn ea_calc_imm_l_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let hi = (state.prefetch[0] as u32) << 16;
    state.micro.source = hi;
}

/// Immediate Long Cycle 2: Low word -> assemble into source
#[inline(always)]
pub fn ea_calc_imm_l_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = state.micro.source | (state.prefetch[0] as u32);
    state.micro.source = val;
}

/// Program Counter with Displacement: (d16, PC) -> ea_addr = PC + disp16
#[inline(always)]
pub fn ea_calc_d16_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    // PC points to extension word address (PC - 2 from the current advancing PC)
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
}

/// Program Counter with Index: (d8, PC, Xn) -> ea_addr = PC + Xn + disp8
#[inline(always)]
pub fn ea_calc_idx_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = read_index_reg(state, ext);
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(xn).wrapping_add(disp8 as u32);
}

// ============================================================================
// Destination EA Calculations (reg_dst: 0..7)
// ============================================================================

/// Destination Address Register Indirect: (An) -> ea_addr = An
#[inline(always)]
pub fn ea_calc_dst_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.ea_addr = state.read_a(reg_dst as usize);
}

/// Destination Address Register Indirect with Postincrement (Byte): (An)+ -> ea_addr = An, An += (2 if A7 else 1)
#[inline(always)]
pub fn ea_calc_dst_pi_b(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    let inc = if reg_dst == 7 { 2 } else { 1 };
    state.write_a(reg_dst as usize, an.wrapping_add(inc));
}

/// Destination Address Register Indirect with Postincrement (Word): (An)+ -> ea_addr = An, An += 2
#[inline(always)]
pub fn ea_calc_dst_pi_w(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    state.write_a(reg_dst as usize, an.wrapping_add(2));
}

/// Destination Address Register Indirect with Postincrement (Long): (An)+ -> ea_addr = An, An += 4
#[inline(always)]
pub fn ea_calc_dst_pi_l(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    state.write_a(reg_dst as usize, an.wrapping_add(4));
}

/// MOVE Destination Address Register Indirect with Postincrement (Word):
/// On 68000 silicon, post-increment on a pure WRITE operation is suppressed if an Address Error occurs.
#[inline(always)]
pub fn ea_calc_move_dst_pi_w(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    if (an & 1) == 0 {
        state.write_a(reg_dst as usize, an.wrapping_add(2));
    }
}

/// Destination Address Register Indirect with Predecrement (Byte): -(An) -> An -= (2 if A7 else 1), ea_addr = An
#[inline(always)]
pub fn ea_calc_dst_pd_b(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let dec = if reg_dst == 7 { 2 } else { 1 };
    let an = state.read_a(reg_dst as usize).wrapping_sub(dec);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

/// Destination Address Register Indirect with Predecrement (Word): -(An) -> An -= 2, ea_addr = An
#[inline(always)]
pub fn ea_calc_dst_pd_w(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize).wrapping_sub(2);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

/// Destination Address Register Indirect with Predecrement (Long): -(An) -> An -= 4, ea_addr = An
#[inline(always)]
pub fn ea_calc_dst_pd_l(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = state.read_a(reg_dst as usize).wrapping_sub(4);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

/// Destination Address Register Indirect with Displacement: (d16, An) -> ea_addr = An + disp16
#[inline(always)]
pub fn ea_calc_dst_d16_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    state.micro.ea_addr = state.read_a(reg_dst as usize).wrapping_add(disp as u32);
}

/// Destination Address Register Indirect with Index: (d8, An, Xn) -> ea_addr = An + Xn + disp8
#[inline(always)]
pub fn ea_calc_dst_idx_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = read_index_reg(state, ext);
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an.wrapping_add(xn).wrapping_add(disp8 as u32);
}

// ============================================================================
// Control Addressing Helpers
// ============================================================================

/// Address Register Indirect with Index helper
#[inline(always)]
pub fn ea_calc_src_idx_an_pure(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    ea_calc_src_idx_an(state, reg_src, reg_dst);
}

/// PC Indirect with Index helper
#[inline(always)]
pub fn ea_calc_idx_pc_pure(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    ea_calc_idx_pc(state, reg_src, reg_dst);
}

// ============================================================================
// PEA (Push Effective Address) Helpers (destination = EA)
// ============================================================================

/// PEA (An): destination = An
#[inline(always)]
pub fn ea_calc_pea_ai(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let ea = state.read_a(reg_src as usize);
    state.micro.destination = ea;
}

/// PEA (d16, An): destination = An + disp16
#[inline(always)]
pub fn ea_calc_pea_d16_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    let ea = state.read_a(reg_src as usize).wrapping_add(disp as u32);
    state.micro.destination = ea;
}

/// PEA (d8, An, Xn): destination = An + Xn + disp8
#[inline(always)]
pub fn ea_calc_pea_idx_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = read_index_reg(state, ext);
    let an = state.read_a(reg_src as usize);
    let ea = an.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.destination = ea;
}

/// PEA (xxx).W: destination = sign_extend(word)
#[inline(always)]
pub fn ea_calc_pea_absw(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ea = state.prefetch[0] as i16 as i32 as u32;
    state.micro.destination = ea;
}

/// PEA (xxx).L: assemble low word into destination
#[inline(always)]
pub fn ea_calc_pea_absl_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ea = state.micro.ea_high | (state.prefetch[0] as u32);
    state.micro.destination = ea;
}

/// PEA (d16, PC): destination = PC + disp16
#[inline(always)]
pub fn ea_calc_pea_d16_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    let base_pc = state.pc.wrapping_sub(2);
    let ea = base_pc.wrapping_add(disp as u32);
    state.micro.destination = ea;
}

/// PEA (d8, PC, Xn): destination = PC + Xn + disp8
#[inline(always)]
pub fn ea_calc_pea_idx_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = read_index_reg(state, ext);
    let base_pc = state.pc.wrapping_sub(2);
    let ea = base_pc.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.destination = ea;
}

// ============================================================================
// Compound Dual-Memory Operand Helpers (ADDX, SUBX, CMPM)
// ============================================================================

/// Long predecrement split for source -(Ay): decrements by 2, saves Ay - 4 in ea_high, ea_addr = low word
#[inline(always)]
pub fn ea_calc_src_pd_l_split(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize);
    let low_addr = an.wrapping_sub(2);
    state.write_a(reg_src as usize, low_addr);
    state.micro.ea_high = an.wrapping_sub(4);
    state.micro.ea_addr = low_addr;
}

/// Prepares ea_addr for source high word
#[inline(always)]
pub fn latch_src_lo_and_read_src_hi(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.write_a(reg_src as usize, state.micro.ea_high);
    state.micro.ea_addr = state.micro.ea_high;
}

/// Prepares destination -(Ax) split predecrement
#[inline(always)]
pub fn calc_dst_pd_l(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let ax = state.read_a(reg_dst as usize);
    let low_addr = ax.wrapping_sub(2);
    state.write_a(reg_dst as usize, low_addr);
    state.micro.ea_high = ax.wrapping_sub(4);
    state.micro.ea_addr = low_addr;
}

/// Prepares ea_addr for destination high word
#[inline(always)]
pub fn latch_dst_lo_and_read_dst_hi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.write_a(reg_dst as usize, state.micro.ea_high);
    state.micro.ea_addr = state.micro.ea_high;
}

/// Sets ea_addr to high word and shifts destination right by 16 for high word write-back
#[inline(always)]
pub fn set_write_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.micro.ea_high;
    state.micro.destination >>= 16;
}
