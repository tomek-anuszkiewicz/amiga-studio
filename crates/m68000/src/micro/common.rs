//! Reusable Common Micro-Step Building Blocks
//!
//! Centralizes universal atomic bus steps and retirement sequences shared
//! across all Motorola 68000 instructions.

use super::types::MicroStep;
use crate::core::Cpu;
use crate::state::CpuState;

// ============================================================================
// 2-Clock Micro-Step Building Blocks (1 CCK = 2 CPU Clocks)
// ============================================================================

/// CCK1: Prefetch next instruction opcode from PC
pub const PREFETCH_NEXT_READ: MicroStep = MicroStep::cck(Cpu::step_prefetch_next_read);

/// CCK2: Prefetch next instruction opcode finish & standard retirement (idle CCK alias)
pub const PREFETCH_NEXT_RETIRE: MicroStep = BUS_READ_IDLE;

/// CCK1: Instruction extension word read from PC
pub const FETCH_EXT_READ: MicroStep = MicroStep::cck(Cpu::step_fetch_extension_read);

/// CCK2: Instruction extension word finish (latches into prefetch[0], PC += 2)
pub const FETCH_EXT_FINISH: MicroStep = MicroStep::cck(Cpu::step_fetch_extension_finish);

/// CCK1: Prefetch next opcode directly to IRC
pub const PREFETCH_IRC_READ: MicroStep = PREFETCH_NEXT_READ;

/// CCK2: Prefetch next opcode to IRC finish (latches into irc and advances prefetch)
pub const PREFETCH_IRC_FINISH: MicroStep = MicroStep::cck(Cpu::step_prefetch_irc_finish);

/// CCK1: Bus write idle step (internal address/pin setup, bus free for Agnus DMA)
pub const BUS_WRITE_IDLE: MicroStep = MicroStep::cck_idle();

/// CCK2: Bus read idle step (physical bus free for Agnus DMA after operand or opcode read initiation)
pub const BUS_READ_IDLE: MicroStep = MicroStep::cck_idle();

/// CCK2: Finishes bus read word cycle (idle CCK alias)
pub const READ_WORD_FINISH: MicroStep = BUS_READ_IDLE;

/// CCK2: Finishes bus read byte cycle (idle CCK alias)
pub const READ_BYTE_FINISH: MicroStep = BUS_READ_IDLE;

/// CCK1: Reads source word from memory into `source`
pub const READ_SRC_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_word);

/// CCK1: Reads source byte from memory into `source`
pub const READ_SRC_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_byte);

/// CCK1: Reads destination word from memory into `destination`
pub const READ_DST_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_word);

/// CCK1: Reads destination byte from memory into `destination`
pub const READ_DST_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_byte);

/// CCK2: Writes word from `destination` to memory
pub const WRITE_DST_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_word);

/// CCK2: Writes word from `destination` to memory (deprecated alias, use `WRITE_DST_WORD`)
pub const WRITE_DST_WORD_RETIRE: MicroStep = WRITE_DST_WORD;

/// CCK2: Writes byte from `destination` to memory
pub const WRITE_DST_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_byte);

/// CCK2: Writes byte from `destination` to memory (deprecated alias, use `WRITE_DST_BYTE`)
pub const WRITE_DST_BYTE_RETIRE: MicroStep = WRITE_DST_BYTE;

/// CCK1: Reads source long high word from memory into `source`
pub const READ_SRC_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_long_high);

/// CCK1: Reads source long low word from memory into `source`
pub const READ_SRC_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_long_low);

/// CCK1: Reads destination long high word from memory into `destination`
pub const READ_DST_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_long_high);

/// CCK1: Reads destination long low word from memory into `destination`
pub const READ_DST_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_long_low);

/// CCK1: Reads split high word of 32-bit source operand into bits 16..31 of `source`
pub const READ_SRC_SPLIT_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_split_high);

/// CCK1: Reads split high word of 32-bit destination operand into bits 16..31 of `destination`
pub const READ_DST_SPLIT_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_split_high);

/// CCK2: Writes high word of 32-bit destination to memory
pub const WRITE_DST_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_long_high);

/// CCK2: Writes low word of 32-bit destination to memory + 2
pub const WRITE_DST_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_long_low);

/// CCK2: Writes low word of 32-bit destination to memory + 2 (deprecated alias, use `WRITE_DST_LONG_LOW`)
pub const WRITE_DST_LONG_LOW_RETIRE: MicroStep = WRITE_DST_LONG_LOW;

/// CCK2: Writes low word of 32-bit destination to memory at predecrement address (An - 2)
pub const WRITE_DST_PD_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_pd_long_low);

/// CCK2: Writes high word of 32-bit destination to memory at predecrement address (An - 4)
pub const WRITE_DST_PD_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_pd_long_high);

/// CCK1: Reads source byte from staged `addr1` into `source`
pub const READ_ADDR1_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr1_byte);

/// CCK1: Reads source word from staged `addr1` into `source`
pub const READ_ADDR1_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr1_word);

/// CCK1: Reads destination byte from staged `addr2` into `destination`
pub const READ_ADDR2_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr2_byte);

/// CCK1: Reads destination word from staged `addr2` into `destination`
pub const READ_ADDR2_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr2_word);

/// CCK1: Reads source long high word from staged `addr1` into `source`
pub const READ_ADDR1_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr1_long_high);

/// CCK1: Reads source long low word from staged `addr1 + 2` into `source`
pub const READ_ADDR1_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr1_long_low);

/// CCK1: Reads destination long high word from staged `addr2` into `destination`
pub const READ_ADDR2_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr2_long_high);

/// CCK1: Reads destination long low word from staged `addr2 + 2` into `destination`
pub const READ_ADDR2_LONG_LOW: MicroStep = MicroStep::cck(Cpu::step_bus_read_addr2_long_low);

// ============================================================================
// 2-Clock Control Flow, Stack & Target Refill Building Blocks
// ============================================================================

/// CCK1: Reads first word of target instruction from `ea_addr`
pub const READ_TARGET_OPCODE_READ: MicroStep =
    MicroStep::cck(Cpu::step_bus_read_target_opcode_read);

/// CCK2: Target opcode read idle step (bus free for Agnus DMA)
pub const READ_TARGET_OPCODE_FINISH: MicroStep = BUS_READ_IDLE;

/// CCK1: Reads second word of target pipeline from `ea_addr + 2`
pub const PREFETCH_TARGET_READ: MicroStep = MicroStep::cck(Cpu::step_prefetch_target_read);

/// CCK2: Completes target pipeline refill retirement
pub const PREFETCH_TARGET_FINISH: MicroStep = MicroStep::cck(Cpu::step_prefetch_target_finish);

/// CCK1: Stack push high word setup: SP -= 4, address check, idle bus
pub const PUSH_STACK_HIGH_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_high_idle);

/// CCK2: Stack push high word write to SP
pub const PUSH_STACK_HIGH_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_high_write);

/// CCK2: Stack push low word write to SP + 2
pub const PUSH_STACK_LOW_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_low_write);

/// CCK2: Stack push low word write to SP + 2 (deprecated alias, use `PUSH_STACK_LOW_WRITE`)
pub const PUSH_STACK_LOW_WRITE_RETIRE: MicroStep = PUSH_STACK_LOW_WRITE;

/// CCK1: Stack pop high word read from (SP)
pub const POP_STACK_HIGH_READ: MicroStep = MicroStep::cck(Cpu::step_bus_pop_stack_high_read);

/// CCK2: Stack pop high word finish: SP += 2, latch scratch[0], log transaction
pub const POP_STACK_HIGH_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_pop_stack_high_finish);

/// CCK1: Stack pop low word read from (SP)
pub const POP_STACK_LOW_READ: MicroStep = MicroStep::cck(Cpu::step_bus_pop_stack_low_read);

/// CCK2: Stack pop low word finish: SP += 2, assemble ea_addr, log transaction
pub const POP_STACK_LOW_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_pop_stack_low_finish);

// ============================================================================
// 2-Clock Internal Execution Building Blocks
// ============================================================================

/// 2-clock internal ALU/idle cycle (1 CCK, no external bus activity)
pub const ALU_IDLE: MicroStep = MicroStep::cck_idle();

/// 2-clock internal processing cycle alias (1 CCK, no external bus activity)
pub const ALU_INTERNAL_2CLK: MicroStep = ALU_IDLE;

// ============================================================================
// 2-Clock Exception Processing Building Blocks
// ============================================================================

/// CCK1: Exception stack push low word of return PC setup to SP - 2 (idle bus, address check)
pub const EXCEPTION_PUSH_PCLO_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pclo_idle);

/// CCK2: Exception stack push low word of return PC write to SP - 2
pub const EXCEPTION_PUSH_PCLO_WRITE: MicroStep =
    MicroStep::cck(Cpu::step_bus_write_trap_pclo_write);

/// CCK1: Exception stack push SR setup to SP - 6 (idle bus, address check)
pub const EXCEPTION_PUSH_SR_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_sr_idle);

/// CCK2: Exception stack push SR write to SP - 6
pub const EXCEPTION_PUSH_SR_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_sr_write);

/// CCK1: Exception stack push high word of return PC setup to SP - 4 (idle bus, address check)
pub const EXCEPTION_PUSH_PCHI_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pchi_idle);

/// CCK2: Exception stack push high word of return PC write to SP - 4 and commit SP = SP - 6
pub const EXCEPTION_PUSH_PCHI_WRITE: MicroStep =
    MicroStep::cck(Cpu::step_bus_write_trap_pchi_write);

/// CCK1: Reads exception vector high word from ea_addr into ea_high
pub const READ_VECTOR_HIGH_READ: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_high_read);

/// CCK2: Vector high word read idle step (bus free for Agnus DMA)
pub const READ_VECTOR_HIGH_FINISH: MicroStep = BUS_READ_IDLE;

/// CCK1: Reads exception vector low word from ea_addr + 2 into source
pub const READ_VECTOR_LOW_READ: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_low_read);

/// CCK2: Logs exception vector low word read transaction, checks target alignment, and updates ea_addr
pub const READ_VECTOR_LOW_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_low_finish);

// ============================================================================
// Group 0 Address Error Exception Building Blocks & Pipeline (50 Clocks / 25 CCKs)
// ============================================================================

/// Exception Vector 3 address ($00000C) for Group 0 Address Error
pub const VECTOR_ADDRESS_ERROR: u32 = 0x0000_000C;

/// Initial setup for Address Error exception:
/// Sets supervisor mode (S=1, T=0), checks for double-bus fault,
/// snapshots SSP, return PC, old SR, and sets vector address ($00000C).
pub fn alu_aerr_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let old_sr = state.sr;
    state.set_supervisor(true);
    state.sr &= !0x8000;

    let ssp = state.read_a(7);
    if (ssp & 1) != 0 {
        state.halted = true;
    }
    state.micro.ssp_base = ssp;
    state.micro.source = state.instruction_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = VECTOR_ADDRESS_ERROR;
}

/// CCK1: Address Error initial ALU step (S=1, T=0, SSP check, vector setup)
pub static ALU_AERR_INIT: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_aerr_init),
    base_clocks: 2,
};

/// CCK1: Address Error stack push low word of return PC setup to SSP - 2
pub const AERR_PUSH_PCLO_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_pclo_idle);

/// CCK2: Address Error stack push low word of return PC write to SSP - 2
pub const AERR_PUSH_PCLO_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_pclo_write);

/// CCK1: Address Error stack push SR setup to SSP - 6
pub const AERR_PUSH_SR_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_sr_idle);

/// CCK2: Address Error stack push SR write to SSP - 6
pub const AERR_PUSH_SR_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_sr_write);

/// CCK1: Address Error stack push high word of return PC setup to SSP - 4
pub const AERR_PUSH_PCHI_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_pchi_idle);

/// CCK2: Address Error stack push high word of return PC write to SSP - 4
pub const AERR_PUSH_PCHI_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_pchi_write);

/// CCK1: Address Error stack push IR setup to SSP - 8
pub const AERR_PUSH_IR_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_ir_idle);

/// CCK2: Address Error stack push IR write to SSP - 8
pub const AERR_PUSH_IR_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_ir_write);

/// CCK1: Address Error stack push access address low word setup to SSP - 10
pub const AERR_PUSH_ADDR_LO_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_addr_lo_idle);

/// CCK2: Address Error stack push access address low word write to SSP - 10
pub const AERR_PUSH_ADDR_LO_WRITE: MicroStep =
    MicroStep::cck(Cpu::step_bus_write_aerr_addr_lo_write);

/// CCK1: Address Error stack push info word setup to SSP - 14
pub const AERR_PUSH_INFO_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_info_idle);

/// CCK2: Address Error stack push info word write to SSP - 14
pub const AERR_PUSH_INFO_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_info_write);

/// CCK1: Address Error stack push access address high word setup to SSP - 12
pub const AERR_PUSH_ADDR_HI_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_aerr_addr_hi_idle);

/// CCK2: Address Error stack push access address high word write to SSP - 12 and commit SSP = SSP - 14
pub const AERR_PUSH_ADDR_HI_WRITE: MicroStep =
    MicroStep::cck(Cpu::step_bus_write_aerr_addr_hi_write);

/// Microcode pipeline for Group 0 Address Error exception processing (50 CPU clocks / 25 CCKs)
pub static STEPS_ADDRESS_ERROR: [MicroStep; 25] = [
    ALU_AERR_INIT,
    ALU_IDLE,
    AERR_PUSH_PCLO_IDLE,
    AERR_PUSH_PCLO_WRITE,
    AERR_PUSH_SR_IDLE,
    AERR_PUSH_SR_WRITE,
    AERR_PUSH_PCHI_IDLE,
    AERR_PUSH_PCHI_WRITE,
    AERR_PUSH_IR_IDLE,
    AERR_PUSH_IR_WRITE,
    AERR_PUSH_ADDR_LO_IDLE,
    AERR_PUSH_ADDR_LO_WRITE,
    AERR_PUSH_INFO_IDLE,
    AERR_PUSH_INFO_WRITE,
    AERR_PUSH_ADDR_HI_IDLE,
    AERR_PUSH_ADDR_HI_WRITE,
    READ_VECTOR_HIGH_READ,
    BUS_READ_IDLE,
    READ_VECTOR_LOW_READ,
    READ_VECTOR_LOW_FINISH,
    READ_TARGET_OPCODE_READ,
    BUS_READ_IDLE,
    ALU_IDLE,
    PREFETCH_TARGET_READ,
    PREFETCH_TARGET_FINISH,
];
