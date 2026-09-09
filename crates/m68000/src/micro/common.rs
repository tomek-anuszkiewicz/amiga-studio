//! Reusable Common Micro-Step Building Blocks
//!
//! Centralizes universal atomic bus steps and retirement sequences shared
//! across all Motorola 68000 instructions.

use crate::core::Cpu;
use super::types::MicroStep;



// ============================================================================
// 2-Clock Micro-Step Building Blocks (1 CCK = 2 CPU Clocks)
// ============================================================================

/// CCK1: Prefetch next instruction opcode from PC
pub const PREFETCH_NEXT_READ: MicroStep = MicroStep::cck(Cpu::step_prefetch_next_read);

/// CCK2: Prefetch next instruction opcode finish & standard retirement
pub const PREFETCH_NEXT_RETIRE: MicroStep = MicroStep::cck(Cpu::step_prefetch_next_and_retire);

/// CCK1: Instruction extension word read from PC
pub const FETCH_EXT_READ: MicroStep = MicroStep::cck(Cpu::step_fetch_extension_read);

/// CCK2: Instruction extension word finish (latches into prefetch[0], PC += 2)
pub const FETCH_EXT_FINISH: MicroStep = MicroStep::cck(Cpu::step_fetch_extension_finish);

/// CCK1: Prefetch next opcode directly to IRC
pub const PREFETCH_IRC_READ: MicroStep = MicroStep::cck(Cpu::step_prefetch_irc_read);

/// CCK2: Prefetch next opcode to IRC finish (latches into irc and advances prefetch)
pub const PREFETCH_IRC_FINISH: MicroStep = MicroStep::cck(Cpu::step_prefetch_irc_finish);

/// CCK1: Forwarding alias for prefetch to IRC
pub const PREFETCH_SCRATCH_READ: MicroStep = PREFETCH_IRC_READ;

/// CCK2: Forwarding alias for prefetch to IRC finish
pub const PREFETCH_SCRATCH_FINISH: MicroStep = PREFETCH_IRC_FINISH;

/// CCK1: Bus write idle step (internal address/pin setup, bus free for Agnus DMA)
pub const BUS_WRITE_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_idle);

/// CCK2: Finishes bus read word cycle and records transaction
pub const READ_WORD_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_word_finish);

/// CCK2: Finishes bus read byte cycle and records transaction
pub const READ_BYTE_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_byte_finish);

/// CCK1: Reads source word from memory into `source`
pub const READ_SRC_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_word);

/// CCK1: Reads source byte from memory into `source`
pub const READ_SRC_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_src_byte);

/// CCK1: Reads destination word from memory into `destination`
pub const READ_DST_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_word);

/// CCK1: Reads destination byte from memory into `destination`
pub const READ_DST_BYTE: MicroStep = MicroStep::cck(Cpu::step_bus_read_dst_byte);

/// CCK2: Writes word from `destination` to memory and retires
pub const WRITE_DST_WORD_RETIRE: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_word_and_retire);

/// CCK2: Writes byte from `destination` to memory and retires
pub const WRITE_DST_BYTE_RETIRE: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_byte_and_retire);

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

/// CCK2: Finishes reading split high word and records transaction from bits 16..31
pub const READ_SPLIT_HIGH_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_split_high_finish);

/// CCK2: Writes word from `destination` to memory (non-retiring)
pub const WRITE_DST_WORD: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_word);

/// CCK2: Writes high word of 32-bit destination to memory (non-retiring)
pub const WRITE_DST_LONG_HIGH: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_long_high);

/// CCK2: Writes low word of 32-bit destination to memory + 2 and retires
pub const WRITE_DST_LONG_LOW_RETIRE: MicroStep = MicroStep::cck(Cpu::step_bus_write_dst_long_low_and_retire);

// ============================================================================
// 2-Clock Control Flow, Stack & Target Refill Building Blocks
// ============================================================================

/// CCK1: Reads first word of target instruction from `ea_addr`
pub const READ_TARGET_OPCODE_READ: MicroStep = MicroStep::cck(Cpu::step_bus_read_target_opcode_read);

/// CCK2: Latches target opcode into `irc` and logs transaction
pub const READ_TARGET_OPCODE_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_target_opcode_finish);

/// CCK1: Reads second word of target pipeline from `ea_addr + 2`
pub const PREFETCH_TARGET_READ: MicroStep = MicroStep::cck(Cpu::step_prefetch_target_read);

/// CCK2: Logs second target word transaction and completes pipeline refill retirement
pub const PREFETCH_TARGET_RETIRE_2CLK: MicroStep = MicroStep::cck(Cpu::step_prefetch_target_and_retire_2clk);

/// CCK1: Stack push high word setup: SP -= 4, address check, idle bus
pub const PUSH_STACK_HIGH_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_high_idle);

/// CCK2: Stack push high word write to SP
pub const PUSH_STACK_HIGH_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_high_write);

/// CCK2: Stack push low word write to SP + 2 (non-retiring, for JSR/BSR)
pub const PUSH_STACK_LOW_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_low_write);

/// CCK2: Stack push low word write to SP + 2 and retires with scratch prefetch (for PEA)
pub const PUSH_STACK_LOW_WRITE_RETIRE: MicroStep = MicroStep::cck(Cpu::step_bus_push_stack_low_write_and_retire);

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

/// CCK1/CCK2: 2-clock internal ALU/idle cycle without bus activity
pub const ALU_IDLE: MicroStep = MicroStep::cck(Cpu::step_alu);

/// CCK1/CCK2: 2-clock internal processing cycle, recording internal transaction if enabled
pub const ALU_INTERNAL_2CLK: MicroStep = MicroStep::cck(Cpu::step_alu_internal_2clk);

// ============================================================================
// 2-Clock Exception Processing Building Blocks
// ============================================================================

/// CCK1: Exception stack push low word of return PC setup to SP - 2 (idle bus, address check)
pub const EXCEPTION_PUSH_PCLO_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pclo_idle);

/// CCK2: Exception stack push low word of return PC write to SP - 2
pub const EXCEPTION_PUSH_PCLO_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pclo_write);

/// CCK1: Exception stack push SR setup to SP - 6 (idle bus, address check)
pub const EXCEPTION_PUSH_SR_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_sr_idle);

/// CCK2: Exception stack push SR write to SP - 6
pub const EXCEPTION_PUSH_SR_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_sr_write);

/// CCK1: Exception stack push high word of return PC setup to SP - 4 (idle bus, address check)
pub const EXCEPTION_PUSH_PCHI_IDLE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pchi_idle);

/// CCK2: Exception stack push high word of return PC write to SP - 4 and commit SP = SP - 6
pub const EXCEPTION_PUSH_PCHI_WRITE: MicroStep = MicroStep::cck(Cpu::step_bus_write_trap_pchi_write);

/// CCK1: Reads exception vector high word from ea_addr into ea_high
pub const READ_VECTOR_HIGH_READ: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_high_read);

/// CCK2: Logs exception vector high word read transaction
pub const READ_VECTOR_HIGH_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_high_finish);

/// CCK1: Reads exception vector low word from ea_addr + 2 into source
pub const READ_VECTOR_LOW_READ: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_low_read);

/// CCK2: Logs exception vector low word read transaction, checks target alignment, and updates ea_addr
pub const READ_VECTOR_LOW_FINISH: MicroStep = MicroStep::cck(Cpu::step_bus_read_vector_low_finish);


