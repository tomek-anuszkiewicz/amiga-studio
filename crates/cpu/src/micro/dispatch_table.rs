//! 65,536-Entry Compile-Time Static Micro-Step Dispatch Table
//!
//! Evaluated at compile time via `const fn` and placed directly in .rodata
//! for single-cycle O(1) instruction dispatch without runtime branching.
//! Represents the Microcode Archetype Baseline for the M68000 core.

use super::encoding::opcode;
use super::types::{OpcodeDescriptor, EMPTY_STEPS};
use crate::micro::encoding::field;
use crate::micro::encoding::{bit_operation, direction, shift};
use crate::micro::encoding::{condition, mode, operand_size};

const OPCODE_COUNT: usize = 65536;

/// Evaluates and builds the static 65,536 opcode descriptor table at compile time
pub const fn build_opcode_descriptor_table() -> [OpcodeDescriptor; OPCODE_COUNT] {
    #[allow(unused_mut)]
    let mut table = [OpcodeDescriptor {
        steps: &EMPTY_STEPS,
        reg_src: 0,
        reg_dst: 0,
    }; OPCODE_COUNT];

    // Phase 1: NOP ($4E71)
    table[opcode::NOP_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::nop::STEPS_NOP,
        reg_src: 0,
        reg_dst: 0,
    };

    // Phase 1: RTS ($4E75)
    table[opcode::RTS_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::rts::STEPS_RTS,
        reg_src: 0,
        reg_dst: 0,
    };

    // Phase 2: TRAP #0..#15 ($4E40..=$4E4F)
    let mut op = opcode::TRAP_FIRST;
    while op <= opcode::TRAP_LAST {
        table[op] = OpcodeDescriptor {
            steps: &crate::instructions::trap::STEPS_TRAP,
            reg_src: (op & field::MASK_TRAP_NUMBER as usize) as u8,
            reg_dst: 0,
        };
        op += 1;
    }

    // Phase 2: BRA, BSR, Bcc ($6000..=$6FFF)
    let mut op = opcode::BRANCH_FIRST;
    while op <= opcode::BRANCH_LAST {
        let cond = ((op >> condition::BIT_SHIFT) & condition::MASK_CONDITION as usize) as u8;
        let d8 = (op & 0xFF) as u8;
        let steps = match cond {
            condition::TRUE => crate::instructions::bra::decode_bra_steps(d8),
            condition::FALSE => crate::instructions::bsr::decode_bsr_steps(d8),
            _ => crate::instructions::bcc::decode_bcc_steps(d8),
        };
        table[op] = OpcodeDescriptor {
            steps,
            reg_src: 0,
            reg_dst: 0,
        };
        op += 1;
    }

    // Phase 2: PEA ($4840..=$487F)
    let mut op = opcode::PEA_FIRST;
    while op <= opcode::PEA_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::pea::decode_pea_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: 0,
            };
        }
        op += 1;
    }

    // Phase 2: JSR ($4E90..=$4EBF)
    let mut op = opcode::JSR_FIRST;
    while op <= opcode::JSR_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::jsr::decode_jsr_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: 0,
            };
        }
        op += 1;
    }

    // Phase 2: JMP ($4ED0..=$4EFF)
    let mut op = opcode::JMP_FIRST;
    while op <= opcode::JMP_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::jmp::decode_jmp_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: 0,
            };
        }
        op += 1;
    }

    // Phase 6: MOVEM ($4880..=$48FF, $4C80..=$4CFF)
    let mut op = opcode::MOVEM_STORE_FIRST;
    while op <= opcode::MOVEM_STORE_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::movem::decode_movem_steps(true, mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }
    let mut op = opcode::MOVEM_LOAD_FIRST;
    while op <= opcode::MOVEM_LOAD_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::movem::decode_movem_steps(false, mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // Phase 6: MOVEA.L ($2000..=$2FFF, dst_mode == mode::ADDRESS_REGISTER)
    let mut op = opcode::MOVE_LONG_FIRST;
    while op <= opcode::MOVE_LONG_LAST {
        let ir = op as u16;
        let dst_reg = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dst_mode = ((ir >> field::BIT_MOVE_DEST_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_reg = (ir & field::MASK_REGISTER as u16) as u8;
        if dst_mode == mode::ADDRESS_REGISTER {
            if let Some(steps) =
                crate::instructions::movea::decode_movea_steps(true, src_mode, src_reg)
            {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: src_reg,
                    reg_dst: dst_reg,
                };
            }
        }
        op += 1;
    }

    // Phase 6: MOVE.B ($1000..=$1FFF)
    let mut op = opcode::MOVE_BYTE_FIRST;
    while op <= opcode::MOVE_BYTE_LAST {
        let ir = op as u16;
        let dst_reg = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dst_mode = ((ir >> field::BIT_MOVE_DEST_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_reg = (ir & field::MASK_REGISTER as u16) as u8;
        if let Some(steps) =
            crate::instructions::move_b::decode_move_b_steps(src_mode, src_reg, dst_mode, dst_reg)
        {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: src_reg,
                reg_dst: dst_reg,
            };
        }
        op += 1;
    }

    // Phase 6: MOVE.L & MOVEA.L ($2000..=$2FFF)
    let mut op = opcode::MOVE_LONG_FIRST;
    while op <= opcode::MOVE_LONG_LAST {
        let ir = op as u16;
        let dst_reg = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dst_mode = ((ir >> field::BIT_MOVE_DEST_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_reg = (ir & field::MASK_REGISTER as u16) as u8;
        let steps_opt = if dst_mode == mode::ADDRESS_REGISTER {
            crate::instructions::movea::decode_movea_steps(true, src_mode, src_reg)
        } else {
            crate::instructions::move_l::decode_move_l_steps(src_mode, src_reg, dst_mode, dst_reg)
        };
        if let Some(steps) = steps_opt {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: src_reg,
                reg_dst: dst_reg,
            };
        }
        op += 1;
    }

    // Phase 6: MOVE.W & MOVEA.W ($3000..=$3FFF)
    let mut op = opcode::MOVE_WORD_FIRST;
    while op <= opcode::MOVE_WORD_LAST {
        let ir = op as u16;
        let dst_reg = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dst_mode = ((ir >> field::BIT_MOVE_DEST_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let src_reg = (ir & field::MASK_REGISTER as u16) as u8;
        let steps_opt = if dst_mode == mode::ADDRESS_REGISTER {
            crate::instructions::movea::decode_movea_steps(false, src_mode, src_reg)
        } else {
            crate::instructions::move_w::decode_move_w_steps(src_mode, src_reg, dst_mode, dst_reg)
        };
        if let Some(steps) = steps_opt {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: src_reg,
                reg_dst: dst_reg,
            };
        }
        op += 1;
    }

    // Phase 6: MOVEQ ($7000..=$7FFF)
    let mut op = opcode::MOVEQ_FIRST;
    while op <= opcode::MOVEQ_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        if let Some(steps) = crate::instructions::moveq::decode_moveq_steps(ir) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: 0,
                reg_dst: reg_d,
            };
        }
        op += 1;
    }

    // Phase 3: ADD opcodes ($D000..=$DFFF)
    let mut op = opcode::ADD_FIRST;
    while op <= opcode::ADD_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            let is_addx = dir == direction::TO_MEMORY
                && (mode == mode::DATA_REGISTER || mode == mode::ADDRESS_REGISTER);
            if is_addx {
                let is_memory = mode == mode::ADDRESS_REGISTER;
                if let Some(steps) = crate::instructions::addx::decode_addx_steps(is_memory, size) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if let Some(steps) =
                crate::instructions::add::decode_add_steps(dir, size, mode, reg)
            {
                let (reg_src, reg_dst) = if dir == direction::TO_REGISTER {
                    (reg, reg_d)
                } else {
                    (reg_d, reg)
                };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else {
            let is_long = dir != direction::TO_REGISTER;
            if let Some(steps) = crate::instructions::adda::decode_adda_steps(is_long, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        }
        op += 1;
    }

    // Phase 3: SUB, SUBA, SUBX opcodes ($9000..=$9FFF)
    let mut op = opcode::SUB_FIRST;
    while op <= opcode::SUB_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            let is_subx = dir == direction::TO_MEMORY
                && (mode == mode::DATA_REGISTER || mode == mode::ADDRESS_REGISTER);
            if is_subx {
                let is_memory = mode == mode::ADDRESS_REGISTER;
                if let Some(steps) = crate::instructions::subx::decode_subx_steps(is_memory, size) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if let Some(steps) =
                crate::instructions::sub::decode_sub_steps(dir, size, mode, reg)
            {
                let (reg_src, reg_dst) = if dir == direction::TO_REGISTER {
                    (reg, reg_d)
                } else {
                    (reg_d, reg)
                };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else {
            let is_long = dir != direction::TO_REGISTER;
            if let Some(steps) = crate::instructions::suba::decode_suba_steps(is_long, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        }
        op += 1;
    }

    // Phase 3: CMP, CMPA, CMPM, EOR opcodes ($B000..=$BFFF)
    let mut op = opcode::CMP_EOR_FIRST;
    while op <= opcode::CMP_EOR_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            if dir == direction::TO_REGISTER {
                // CMP <ea>, Dn
                if let Some(steps) = crate::instructions::cmp::decode_cmp_steps(size, mode, reg) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if mode == mode::ADDRESS_REGISTER {
                // CMPM (Ay)+, (Ax)+
                if let Some(steps) = crate::instructions::cmpm::decode_cmpm_steps(size) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if let Some(steps) = crate::instructions::eor::decode_eor_steps(size, mode, reg)
            {
                // EOR Dn, <ea>
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg_d,
                    reg_dst: reg,
                };
            }
        } else {
            // CMPA.W (dir == direction::TO_REGISTER) and CMPA.L (dir == direction::TO_MEMORY)
            let is_long = dir != direction::TO_REGISTER;
            if let Some(steps) = crate::instructions::cmpa::decode_cmpa_steps(is_long, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        }
        op += 1;
    }

    // Phase 3: ADDQ and SUBQ opcodes ($5000..=$5FFF)
    let mut op = opcode::QUICK_SCC_DBCC_FIRST;
    while op <= opcode::QUICK_SCC_DBCC_LAST {
        let ir = op as u16;
        let raw_data = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let imm = if raw_data == 0 {
            field::QUICK_IMMEDIATE_ZERO_VALUE
        } else {
            raw_data
        };
        let is_sub = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) != 0;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            // Byte on An is not valid in M68000
            let valid = !(mode == mode::ADDRESS_REGISTER && size == operand_size::BYTE);
            if valid {
                if !is_sub {
                    if let Some(steps) =
                        crate::instructions::addq::decode_addq_steps(size, mode, reg)
                    {
                        table[op] = OpcodeDescriptor {
                            steps,
                            reg_src: imm,
                            reg_dst: reg,
                        };
                    }
                } else if let Some(steps) =
                    crate::instructions::subq::decode_subq_steps(size, mode, reg)
                {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: imm,
                        reg_dst: reg,
                    };
                }
            }
        } else {
            // Batch 1.9: size == operand_size::SPECIAL: DBcc (mode == mode::ADDRESS_REGISTER) and Scc (mode != mode::ADDRESS_REGISTER)
            if mode == mode::ADDRESS_REGISTER {
                table[op] = OpcodeDescriptor {
                    steps: &crate::instructions::dbcc::STEPS_DBCC,
                    reg_src: 0,
                    reg_dst: reg,
                };
            } else if let Some(steps) = crate::instructions::scc::decode_scc_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 3: ADDI opcodes ($0600..=$06FF)
    let mut op = opcode::ADDI_FIRST;
    while op <= opcode::ADDI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::addi::decode_addi_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 3: SUBI opcodes ($0400..=$04FF)
    let mut op = opcode::SUBI_FIRST;
    while op <= opcode::SUBI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::subi::decode_subi_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 4: NOT opcodes ($4600..=$46FF)
    let mut op = opcode::NOT_FIRST;
    while op <= opcode::NOT_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::not::decode_not_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 4: Dynamic bit manipulation opcodes ($0100..=$0FFF where (op & 0xF100) == 0x0100)
    let mut op = opcode::DYNAMIC_BIT_FIRST;
    while op <= opcode::DYNAMIC_BIT_LAST {
        if (op & opcode::MASK_DYNAMIC_BIT) == opcode::DYNAMIC_BIT_PATTERN {
            let ir = op as u16;
            let reg_s =
                ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
            let op_type =
                ((ir >> field::BIT_BIT_OPERATION_SHIFT) & field::MASK_BIT_OPERATION) as u8;
            let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
            let reg = (ir & field::MASK_REGISTER as u16) as u8;

            let maybe_steps = if mode == mode::ADDRESS_REGISTER {
                Some(crate::instructions::movep::decode_movep_steps(op_type))
            } else {
                match op_type {
                    bit_operation::TEST => {
                        crate::instructions::btst::decode_btst_dyn_steps(mode, reg)
                    }
                    bit_operation::CHANGE => {
                        crate::instructions::bchg::decode_bchg_dyn_steps(mode, reg)
                    }
                    bit_operation::CLEAR => {
                        crate::instructions::bclr::decode_bclr_dyn_steps(mode, reg)
                    }
                    bit_operation::SET => {
                        crate::instructions::bset::decode_bset_dyn_steps(mode, reg)
                    }
                    _ => None,
                }
            };

            if let Some(steps) = maybe_steps {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: if mode == mode::ADDRESS_REGISTER && op_type < bit_operation::CLEAR {
                        reg
                    } else {
                        reg_s
                    },
                    reg_dst: if mode == mode::ADDRESS_REGISTER && op_type < bit_operation::CLEAR {
                        reg_s
                    } else {
                        reg
                    },
                };
            }
        }
        op += 1;
    }

    // Phase 4: Static bit manipulation opcodes ($0800..=$08FF)
    let mut op = opcode::STATIC_BIT_FIRST;
    while op <= opcode::STATIC_BIT_LAST {
        let ir = op as u16;
        let op_type = ((ir >> field::BIT_BIT_OPERATION_SHIFT) & field::MASK_BIT_OPERATION) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        let maybe_steps = match op_type {
            bit_operation::TEST => crate::instructions::btst::decode_btst_stat_steps(mode, reg),
            bit_operation::CHANGE => crate::instructions::bchg::decode_bchg_stat_steps(mode, reg),
            bit_operation::CLEAR => crate::instructions::bclr::decode_bclr_stat_steps(mode, reg),
            bit_operation::SET => crate::instructions::bset::decode_bset_stat_steps(mode, reg),
            _ => None,
        };

        if let Some(steps) = maybe_steps {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: 0,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // Phase 5: Shift & Rotate opcodes ($E000..=$EFFF)
    let mut op = opcode::SHIFT_FIRST;
    while op <= opcode::SHIFT_LAST {
        let ir = op as u16;
        let size_bits = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        if size_bits == operand_size::SPECIAL {
            // Memory shift (Word only, Count = 1, bit 11 must be 0)
            if ((ir >> field::BIT_SHIFT_MEMORY_INVALID_SHIFT) & field::MASK_SHIFT_MEMORY_INVALID)
                == 0
            {
                let shift_type =
                    ((ir >> field::BIT_SHIFT_MEMORY_TYPE_SHIFT) & field::MASK_SHIFT_TYPE) as u8;
                let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
                let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
                let reg = (ir & field::MASK_REGISTER as u16) as u8;

                let maybe_steps = match (shift_type, dir) {
                    (shift::ARITHMETIC, shift::RIGHT) => {
                        crate::instructions::asr::decode_asr_mem_steps(mode, reg)
                    }
                    (shift::ARITHMETIC, shift::LEFT) => {
                        crate::instructions::asl::decode_asl_mem_steps(mode, reg)
                    }
                    (shift::LOGICAL, shift::RIGHT) => {
                        crate::instructions::lsr::decode_lsr_mem_steps(mode, reg)
                    }
                    (shift::LOGICAL, shift::LEFT) => {
                        crate::instructions::lsl::decode_lsl_mem_steps(mode, reg)
                    }
                    (shift::ROTATE_EXTEND, shift::RIGHT) => {
                        crate::instructions::roxr::decode_roxr_mem_steps(mode, reg)
                    }
                    (shift::ROTATE_EXTEND, shift::LEFT) => {
                        crate::instructions::roxl::decode_roxl_mem_steps(mode, reg)
                    }
                    (shift::ROTATE, shift::RIGHT) => {
                        crate::instructions::ror::decode_ror_mem_steps(mode, reg)
                    }
                    (shift::ROTATE, shift::LEFT) => {
                        crate::instructions::rol::decode_rol_mem_steps(mode, reg)
                    }
                    _ => None,
                };

                if let Some(steps) = maybe_steps {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: 0,
                        reg_dst: reg,
                    };
                }
            }
        } else {
            // Register shift
            let raw_cnt =
                ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
            let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
            let size = size_bits;
            let is_reg_count = ((ir >> field::BIT_SHIFT_REGISTER_COUNT_SHIFT)
                & field::MASK_SHIFT_REGISTER_COUNT)
                != 0;
            let shift_type =
                ((ir >> field::BIT_SHIFT_REGISTER_TYPE_SHIFT) & field::MASK_SHIFT_TYPE) as u8;
            let reg_dst = (ir & field::MASK_REGISTER as u16) as u8;

            let maybe_steps = match (shift_type, dir) {
                (shift::ARITHMETIC, shift::RIGHT) => {
                    crate::instructions::asr::decode_asr_reg_steps(is_reg_count, size)
                }
                (shift::ARITHMETIC, shift::LEFT) => {
                    crate::instructions::asl::decode_asl_reg_steps(is_reg_count, size)
                }
                (shift::LOGICAL, shift::RIGHT) => {
                    crate::instructions::lsr::decode_lsr_reg_steps(is_reg_count, size)
                }
                (shift::LOGICAL, shift::LEFT) => {
                    crate::instructions::lsl::decode_lsl_reg_steps(is_reg_count, size)
                }
                (shift::ROTATE_EXTEND, shift::RIGHT) => {
                    crate::instructions::roxr::decode_roxr_reg_steps(is_reg_count, size)
                }
                (shift::ROTATE_EXTEND, shift::LEFT) => {
                    crate::instructions::roxl::decode_roxl_reg_steps(is_reg_count, size)
                }
                (shift::ROTATE, shift::RIGHT) => {
                    crate::instructions::ror::decode_ror_reg_steps(is_reg_count, size)
                }
                (shift::ROTATE, shift::LEFT) => {
                    crate::instructions::rol::decode_rol_reg_steps(is_reg_count, size)
                }
                _ => None,
            };

            if let Some(steps) = maybe_steps {
                let reg_src = if is_reg_count {
                    raw_cnt
                } else if raw_cnt == 0 {
                    field::QUICK_IMMEDIATE_ZERO_VALUE
                } else {
                    raw_cnt
                };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        }
        op += 1;
    }

    // Phase 6: AND ($C000..=$CFFF where size < operand_size::SPECIAL), MULU (dir == direction::TO_REGISTER, size == operand_size::SPECIAL), MULS (dir == direction::TO_MEMORY, size == operand_size::SPECIAL)
    let mut op = opcode::LINE_C_FIRST;
    while op <= opcode::LINE_C_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::and::decode_and_steps(dir, size, mode, reg) {
                let (reg_src, reg_dst) = if dir == direction::TO_REGISTER {
                    (reg, reg_d)
                } else {
                    (reg_d, reg)
                };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else if dir == direction::TO_REGISTER {
            if let Some(steps) = crate::instructions::mulu::decode_mulu_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        } else if let Some(steps) = crate::instructions::muls::decode_muls_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg_d,
            };
        }
        op += 1;
    }

    // Phase 6: OR ($8000..=$8FFF where size < operand_size::SPECIAL), DIVU (dir == direction::TO_REGISTER, size == operand_size::SPECIAL), DIVS (dir == direction::TO_MEMORY, size == operand_size::SPECIAL)
    let mut op = opcode::LINE_8_FIRST;
    while op <= opcode::LINE_8_LAST {
        let ir = op as u16;
        let reg_d = ((ir >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as u16) as u8;
        let dir = ((ir >> field::BIT_DIRECTION_SHIFT) & field::MASK_DIRECTION as u16) as u8;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;

        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::or::decode_or_steps(dir, size, mode, reg) {
                let (reg_src, reg_dst) = if dir == direction::TO_REGISTER {
                    (reg, reg_d)
                } else {
                    (reg_d, reg)
                };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else if dir == direction::TO_REGISTER {
            if let Some(steps) = crate::instructions::divu::decode_divu_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        } else if let Some(steps) = crate::instructions::divs::decode_divs_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg_d,
            };
        }
        op += 1;
    }

    // Phase 6: ORI opcodes ($0000..=$00FF where size < operand_size::SPECIAL)
    let mut op = opcode::ORI_FIRST;
    while op <= opcode::ORI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::ori::decode_ori_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 6: ANDI opcodes ($0200..=$02FF where size < operand_size::SPECIAL)
    let mut op = opcode::ANDI_FIRST;
    while op <= opcode::ANDI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::andi::decode_andi_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 6: EORI opcodes ($0A00..=$0AFF where size < operand_size::SPECIAL)
    let mut op = opcode::EORI_FIRST;
    while op <= opcode::EORI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::eori::decode_eori_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 7: CMPI opcodes ($0C00..=$0CFF where size < operand_size::SPECIAL)
    let mut op = opcode::CMPI_FIRST;
    while op <= opcode::CMPI_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::cmpi::decode_cmpi_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 7: TST opcodes ($4A00..=$4AFF where size < operand_size::SPECIAL)
    let mut op = opcode::TST_FIRST;
    while op <= opcode::TST_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::tst::decode_tst_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Batch 1.8: CLR opcodes ($4200..=$42FF where size < operand_size::SPECIAL)
    let mut op = opcode::CLR_FIRST;
    while op <= opcode::CLR_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::clr::decode_clr_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Batch 1.8: NEG opcodes ($4400..=$44FF where size < operand_size::SPECIAL)
    let mut op = opcode::NEG_FIRST;
    while op <= opcode::NEG_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::neg::decode_neg_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Batch 1.8: NEGX opcodes ($4000..=$40FF where size < operand_size::SPECIAL)
    let mut op = opcode::NEGX_FIRST;
    while op <= opcode::NEGX_LAST {
        let ir = op as u16;
        let size = ((ir >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE as u16) as u8;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if size < operand_size::SPECIAL {
            if let Some(steps) = crate::instructions::negx::decode_negx_steps(size, mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Batch 1.8: NBCD ($4800..=$483F)
    let mut op = opcode::NBCD_FIRST;
    while op <= opcode::NBCD_LAST {
        let ir = op as u16;
        let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
        let reg = (ir & field::MASK_REGISTER as u16) as u8;
        if let Some(steps) = crate::instructions::nbcd::decode_nbcd_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: 0,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // Batch 1.8: EXT.W ($4880..=$4887) and EXT.L ($48C0..=$48C7)
    let mut reg = 0u8;
    while reg < crate::state::REGISTER_COUNT as u8 {
        let op_w = opcode::EXT_WORD_OPCODE | (reg as usize);
        table[op_w] = OpcodeDescriptor {
            steps: &crate::instructions::ext::STEPS_EXT_W,
            reg_src: 0,
            reg_dst: reg,
        };
        let op_l = opcode::EXT_LONG_OPCODE | (reg as usize);
        table[op_l] = OpcodeDescriptor {
            steps: &crate::instructions::ext::STEPS_EXT_L,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.8: ABCD ($C000..=$CFFF where bits 15..12 == 0xC, bit 8 == 1, bits 7..4 == 0)
    let mut op = opcode::LINE_C_FIRST;
    while op <= opcode::LINE_C_LAST {
        if (op & opcode::MASK_BCD_OPERATION) == opcode::ABCD_PATTERN {
            let rx = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let ry = (op & field::MASK_REGISTER as usize) as u8;
            let is_mem = (op & opcode::BIT_BCD_MEMORY_MODE) != 0;
            let steps: &'static [super::types::MicroStep] = if is_mem {
                &crate::instructions::abcd::STEPS_ABCD_PD_PD
            } else {
                &crate::instructions::abcd::STEPS_ABCD_DN_DN
            };
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: ry,
                reg_dst: rx,
            };
        }
        op += 1;
    }

    // Batch 1.8: SBCD ($8000..=$8FFF where bits 15..12 == 0x8, bit 8 == 1, bits 7..4 == 0)
    let mut op = opcode::LINE_8_FIRST;
    while op <= opcode::LINE_8_LAST {
        if (op & opcode::MASK_BCD_OPERATION) == opcode::SBCD_PATTERN {
            let rx = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let ry = (op & field::MASK_REGISTER as usize) as u8;
            let is_mem = (op & opcode::BIT_BCD_MEMORY_MODE) != 0;
            let steps: &'static [super::types::MicroStep] = if is_mem {
                &crate::instructions::sbcd::STEPS_SBCD_PD_PD
            } else {
                &crate::instructions::sbcd::STEPS_SBCD_DN_DN
            };
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: ry,
                reg_dst: rx,
            };
        }
        op += 1;
    }

    // Batch 1.10: SWAP ($4840..=$4847)
    let mut reg = 0u8;
    while reg < crate::state::REGISTER_COUNT as u8 {
        let op = opcode::SWAP_OPCODE | (reg as usize);
        table[op] = OpcodeDescriptor {
            steps: &crate::instructions::swap::STEPS_SWAP,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.10: EXG ($C000..=$CFFF where bits 15..12 == 0xC, bit 8 == 1)
    let mut op = opcode::LINE_C_FIRST;
    while op <= opcode::LINE_C_LAST {
        if (op & opcode::MASK_EXG) == opcode::EXG_DATA_PATTERN {
            // EXG Dx, Dy (opmode 0b01000)
            let rx = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let ry = (op & field::MASK_REGISTER as usize) as u8;
            table[op] = OpcodeDescriptor {
                steps: &crate::instructions::exg::STEPS_EXG_DX_DY,
                reg_src: ry,
                reg_dst: rx,
            };
        } else if (op & opcode::MASK_EXG) == opcode::EXG_ADDRESS_PATTERN {
            // EXG Ax, Ay (opmode 0b01001)
            let rx = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let ry = (op & field::MASK_REGISTER as usize) as u8;
            table[op] = OpcodeDescriptor {
                steps: &crate::instructions::exg::STEPS_EXG_AX_AY,
                reg_src: ry,
                reg_dst: rx,
            };
        } else if (op & opcode::MASK_EXG) == opcode::EXG_MIXED_PATTERN {
            // EXG Dx, Ay (opmode 0b10001)
            let rx = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let ry = (op & field::MASK_REGISTER as usize) as u8;
            table[op] = OpcodeDescriptor {
                steps: &crate::instructions::exg::STEPS_EXG_DX_AY,
                reg_src: ry,
                reg_dst: rx,
            };
        }
        op += 1;
    }

    // Batch 1.10: LINK ($4E50..=$4E57) and UNLK ($4E58..=$4E5F)
    let mut reg = 0u8;
    while reg < crate::state::REGISTER_COUNT as u8 {
        let op_link = opcode::LINK_OPCODE | (reg as usize);
        table[op_link] = OpcodeDescriptor {
            steps: &crate::instructions::link::STEPS_LINK,
            reg_src: 0,
            reg_dst: reg,
        };
        let op_unlk = opcode::UNLK_OPCODE | (reg as usize);
        table[op_unlk] = OpcodeDescriptor {
            steps: &crate::instructions::unlk::STEPS_UNLK,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.10: LEA ($41C0..=$4FFF where (op & 0xF1C0) == 0x41C0)
    let mut op = opcode::LEA_FIRST;
    while op <= opcode::LEA_LAST {
        if (op & opcode::MASK_REGISTER_EA) == opcode::LEA_PATTERN {
            let an = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
            let reg = (op & field::MASK_REGISTER as usize) as u8;
            if let Some(steps) = crate::instructions::lea::decode_lea_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: an,
                };
            }
        }
        op += 1;
    }

    // Batch 1.10: CHK ($4180..=$4FFF where (op & 0xF1C0) == 0x4180)
    let mut op = opcode::CHK_FIRST;
    while op <= opcode::CHK_LAST {
        if (op & opcode::MASK_REGISTER_EA) == opcode::CHK_PATTERN {
            let dn = ((op >> field::BIT_DEST_REGISTER_SHIFT) & field::MASK_REGISTER as usize) as u8;
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
            let reg = (op & field::MASK_REGISTER as usize) as u8;
            if let Some(steps) = crate::instructions::chk::decode_chk_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: dn,
                };
            }
        }
        op += 1;
    }

    // Batch 1.11: Privileged & System Control Operations
    table[opcode::RESET_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::reset::STEPS_RESET,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::STOP_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::stop::STEPS_STOP,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::RTE_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::rte::STEPS_RTE,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::TRAPV_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::trapv::STEPS_TRAPV,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::RTR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::rtr::STEPS_RTR,
        reg_src: 0,
        reg_dst: 0,
    };

    let mut reg = 0u8;
    while reg < crate::state::REGISTER_COUNT as u8 {
        table[opcode::MOVE_TO_USP_OPCODE | (reg as usize)] = OpcodeDescriptor {
            steps: &crate::instructions::move_usp::STEPS_MOVE_TO_USP,
            reg_src: reg,
            reg_dst: 0,
        };
        table[opcode::MOVE_FROM_USP_OPCODE | (reg as usize)] = OpcodeDescriptor {
            steps: &crate::instructions::move_usp::STEPS_MOVE_FROM_USP,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.11: Immediate Logic to CCR / SR
    table[opcode::ORI_CCR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ORI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::ORI_SR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ORI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::ANDI_CCR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ANDI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::ANDI_SR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ANDI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::EORI_CCR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_EORI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[opcode::EORI_SR_OPCODE] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_EORI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };

    // Batch 1.11: MOVE <ea>, CCR ($44C0..=$44FF)
    let mut op = opcode::MOVE_TO_CCR_FIRST;
    while op <= opcode::MOVE_TO_CCR_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::move_sr_ccr::decode_move_to_ccr_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: 0,
            };
        }
        op += 1;
    }

    // Batch 1.11: MOVE <ea>, SR ($46C0..=$46FF)
    let mut op = opcode::MOVE_TO_SR_FIRST;
    while op <= opcode::MOVE_TO_SR_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::move_sr_ccr::decode_move_to_sr_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: 0,
            };
        }
        op += 1;
    }

    // Batch 1.11: MOVE SR, <ea> ($40C0..=$40FF)
    let mut op = opcode::MOVE_FROM_SR_FIRST;
    while op <= opcode::MOVE_FROM_SR_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::move_sr_ccr::decode_move_from_sr_steps(mode, reg)
        {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: 0,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // Batch 1.11: TAS <ea> ($4AC0..=$4AFF)
    let mut op = opcode::TAS_FIRST;
    while op <= opcode::TAS_LAST {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as usize) as u8;
        let reg = (op & field::MASK_REGISTER as usize) as u8;
        if let Some(steps) = crate::instructions::tas::decode_tas_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // All unassigned encodings enter Illegal or the corresponding emulator line.
    let mut op = 0;
    while op < table.len() {
        if table[op].steps.is_empty() {
            table[op].steps = &super::common::STEPS_DECODE_EXCEPTION;
        }
        op += 1;
    }
    table
}

/// Master 65,536 opcode descriptor table resident in host .rodata
pub static OPCODE_DESCRIPTOR_TABLE: [OpcodeDescriptor; OPCODE_COUNT] =
    build_opcode_descriptor_table();
