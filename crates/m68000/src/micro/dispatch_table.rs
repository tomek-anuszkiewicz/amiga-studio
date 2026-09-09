//! 65,536-Entry Compile-Time Static Micro-Step Dispatch Table
//!
//! Evaluated at compile time via `const fn` and placed directly in .rodata
//! for single-cycle O(1) instruction dispatch without runtime branching.
//! Represents the Microcode Archetype Baseline for the M68000 core.

use super::types::{OpcodeDescriptor, EMPTY_STEPS};

/// Evaluates and builds the static 65,536 opcode descriptor table at compile time
pub const fn build_opcode_descriptor_table() -> [OpcodeDescriptor; 65536] {
    #[allow(unused_mut)]
    let mut table = [OpcodeDescriptor {
        steps: &EMPTY_STEPS,
        reg_src: 0,
        reg_dst: 0,
    }; 65536];

    // Phase 1: NOP ($4E71)
    table[0x4E71] = OpcodeDescriptor {
        steps: &crate::instructions::nop::STEPS_NOP,
        reg_src: 0,
        reg_dst: 0,
    };

    // Phase 1: RTS ($4E75)
    table[0x4E75] = OpcodeDescriptor {
        steps: &crate::instructions::rts::STEPS_RTS,
        reg_src: 0,
        reg_dst: 0,
    };

    // Phase 2: TRAP #0..#15 ($4E40..=$4E4F)
    let mut op = 0x4E40usize;
    while op <= 0x4E4F {
        table[op] = OpcodeDescriptor {
            steps: &crate::instructions::trap::STEPS_TRAP,
            reg_src: (op & 0x0F) as u8,
            reg_dst: 0,
        };
        op += 1;
    }

    // Phase 2: BRA, BSR, Bcc ($6000..=$6FFF)
    let mut op = 0x6000usize;
    while op <= 0x6FFF {
        let cond = ((op >> 8) & 0x0F) as u8;
        let d8 = (op & 0xFF) as u8;
        let steps = match cond {
            0 => crate::instructions::bra::decode_bra_steps(d8),
            1 => crate::instructions::bsr::decode_bsr_steps(d8),
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
    let mut op = 0x4840usize;
    while op <= 0x487F {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x4E90usize;
    while op <= 0x4EBF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x4ED0usize;
    while op <= 0x4EFF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x4880usize;
    while op <= 0x48FF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        if let Some(steps) = crate::instructions::movem::decode_movem_steps(true, mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }
    let mut op = 0x4C80usize;
    while op <= 0x4CFF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        if let Some(steps) = crate::instructions::movem::decode_movem_steps(false, mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    // Phase 6: MOVEA.L ($2000..=$2FFF, dst_mode == 1)
    let mut op = 0x2000usize;
    while op <= 0x2FFF {
        let ir = op as u16;
        let dst_reg = ((ir >> 9) & 7) as u8;
        let dst_mode = ((ir >> 6) & 7) as u8;
        let src_mode = ((ir >> 3) & 7) as u8;
        let src_reg = (ir & 7) as u8;
        if dst_mode == 1 {
            if let Some(steps) = crate::instructions::movea::decode_movea_steps(true, src_mode, src_reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: src_reg,
                    reg_dst: dst_reg,
                };
            }
        }
        op += 1;
    }

    // Phase 6: MOVE.W & MOVEA.W ($3000..=$3FFF)
    let mut op = 0x3000usize;
    while op <= 0x3FFF {
        let ir = op as u16;
        let dst_reg = ((ir >> 9) & 7) as u8;
        let dst_mode = ((ir >> 6) & 7) as u8;
        let src_mode = ((ir >> 3) & 7) as u8;
        let src_reg = (ir & 7) as u8;
        let steps_opt = if dst_mode == 1 {
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
    let mut op = 0x7000usize;
    while op <= 0x7FFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
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
    let mut op = 0xD000usize;
    while op <= 0xDFFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            let is_addx = dir == 1 && (mode == 0 || mode == 1);
            if is_addx {
                let is_memory = mode == 1;
                if let Some(steps) = crate::instructions::addx::decode_addx_steps(is_memory, size) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if let Some(steps) = crate::instructions::add::decode_add_steps(dir, size, mode, reg) {
                let (reg_src, reg_dst) = if dir == 0 {
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
            let is_long = dir != 0;
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

    // Phase 3: CMPM opcodes ($B000..=$BFFF, dir == 1, mode == 1)
    let mut op = 0xB000usize;
    while op <= 0xBFFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 && dir == 1 && mode == 1 {
            // CMPM (Ay)+, (Ax)+
            if let Some(steps) = crate::instructions::cmpm::decode_cmpm_steps(size) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg,
                    reg_dst: reg_d,
                };
            }
        }
        op += 1;
    }

    // Phase 3: ADDQ opcodes ($5000..=$5FFF, !is_sub)
    let mut op = 0x5000usize;
    while op <= 0x5FFF {
        let ir = op as u16;
        let raw_data = ((ir >> 9) & 7) as u8;
        let imm = if raw_data == 0 { 8 } else { raw_data };
        let is_sub = ((ir >> 8) & 1) != 0;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            // Byte on An is not valid in M68000
            let valid = !(mode == 1 && size == 0);
            if valid && !is_sub {
                if let Some(steps) = crate::instructions::addq::decode_addq_steps(size, mode, reg) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: imm,
                        reg_dst: reg,
                    };
                }
            }
        }
        op += 1;
    }

    // Phase 3: ADDI opcodes ($0600..=$06FF)
    let mut op = 0x0600usize;
    while op <= 0x06FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 4: NOT opcodes ($4600..=$46FF)
    let mut op = 0x4600usize;
    while op <= 0x46FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 4: Dynamic BSET opcodes ($0100..=$0FFF where (op & 0xF100) == 0x0100 && op_type == 3)
    let mut op = 0x0100usize;
    while op <= 0x0FFF {
        if (op & 0xF100) == 0x0100 {
            let ir = op as u16;
            let reg_s = ((ir >> 9) & 7) as u8;
            let op_type = ((ir >> 6) & 3) as u8;
            let mode = ((ir >> 3) & 7) as u8;
            let reg = (ir & 7) as u8;

            if op_type == 3 {
                if let Some(steps) = crate::instructions::bset::decode_bset_dyn_steps(mode, reg) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg_s,
                        reg_dst: reg,
                    };
                }
            }
        }
        op += 1;
    }

    // Phase 4: Static BSET opcodes ($0800..=$08FF where op_type == 3)
    let mut op = 0x0800usize;
    while op <= 0x08FF {
        let ir = op as u16;
        let op_type = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if op_type == 3 {
            if let Some(steps) = crate::instructions::bset::decode_bset_stat_steps(mode, reg) {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: 0,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 5: ASL opcodes ($E000..=$EFFF where shift_type == 0 && dir == 1)
    let mut op = 0xE000usize;
    while op <= 0xEFFF {
        let ir = op as u16;
        let size_bits = ((ir >> 6) & 3) as u8;
        if size_bits == 3 {
            // Memory shift (Word only, Count = 1, bit 11 must be 0)
            if ((ir >> 11) & 1) == 0 {
                let shift_type = ((ir >> 9) & 3) as u8;
                let dir = ((ir >> 8) & 1) as u8;
                let mode = ((ir >> 3) & 7) as u8;
                let reg = (ir & 7) as u8;

                if shift_type == 0 && dir == 1 {
                    if let Some(steps) = crate::instructions::asl::decode_asl_mem_steps(mode, reg) {
                        table[op] = OpcodeDescriptor {
                            steps,
                            reg_src: 0,
                            reg_dst: reg,
                        };
                    }
                }
            }
        } else {
            // Register shift
            let raw_cnt = ((ir >> 9) & 7) as u8;
            let dir = ((ir >> 8) & 1) as u8;
            let size = size_bits;
            let is_reg_count = ((ir >> 5) & 1) != 0;
            let shift_type = ((ir >> 3) & 3) as u8;
            let reg_dst = (ir & 7) as u8;

            if shift_type == 0 && dir == 1 {
                if let Some(steps) = crate::instructions::asl::decode_asl_reg_steps(is_reg_count, size) {
                    let reg_src = if is_reg_count {
                        raw_cnt
                    } else if raw_cnt == 0 {
                        8
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
        }
        op += 1;
    }

    table
}

/// Master 65,536 opcode descriptor table resident in host .rodata
pub static OPCODE_DESCRIPTOR_TABLE: [OpcodeDescriptor; 65536] = build_opcode_descriptor_table();
