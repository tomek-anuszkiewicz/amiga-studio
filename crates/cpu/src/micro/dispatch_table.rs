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
    let mut op = 0x1000usize;
    while op <= 0x1FFF {
        let ir = op as u16;
        let dst_reg = ((ir >> 9) & 7) as u8;
        let dst_mode = ((ir >> 6) & 7) as u8;
        let src_mode = ((ir >> 3) & 7) as u8;
        let src_reg = (ir & 7) as u8;
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
    let mut op = 0x2000usize;
    while op <= 0x2FFF {
        let ir = op as u16;
        let dst_reg = ((ir >> 9) & 7) as u8;
        let dst_mode = ((ir >> 6) & 7) as u8;
        let src_mode = ((ir >> 3) & 7) as u8;
        let src_reg = (ir & 7) as u8;
        let steps_opt = if dst_mode == 1 {
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
            } else if let Some(steps) =
                crate::instructions::add::decode_add_steps(dir, size, mode, reg)
            {
                let (reg_src, reg_dst) = if dir == 0 { (reg, reg_d) } else { (reg_d, reg) };
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

    // Phase 3: SUB, SUBA, SUBX opcodes ($9000..=$9FFF)
    let mut op = 0x9000usize;
    while op <= 0x9FFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            let is_subx = dir == 1 && (mode == 0 || mode == 1);
            if is_subx {
                let is_memory = mode == 1;
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
                let (reg_src, reg_dst) = if dir == 0 { (reg, reg_d) } else { (reg_d, reg) };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else {
            let is_long = dir != 0;
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
    let mut op = 0xB000usize;
    while op <= 0xBFFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            if dir == 0 {
                // CMP <ea>, Dn
                if let Some(steps) = crate::instructions::cmp::decode_cmp_steps(size, mode, reg) {
                    table[op] = OpcodeDescriptor {
                        steps,
                        reg_src: reg,
                        reg_dst: reg_d,
                    };
                }
            } else if mode == 1 {
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
            // CMPA.W (dir == 0) and CMPA.L (dir == 1)
            let is_long = dir != 0;
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
            // Batch 1.9: size == 3: DBcc (mode == 1) and Scc (mode != 1)
            if mode == 1 {
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

    // Phase 3: SUBI opcodes ($0400..=$04FF)
    let mut op = 0x0400usize;
    while op <= 0x04FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 4: Dynamic bit manipulation opcodes ($0100..=$0FFF where (op & 0xF100) == 0x0100)
    let mut op = 0x0100usize;
    while op <= 0x0FFF {
        if (op & 0xF100) == 0x0100 {
            let ir = op as u16;
            let reg_s = ((ir >> 9) & 7) as u8;
            let op_type = ((ir >> 6) & 3) as u8;
            let mode = ((ir >> 3) & 7) as u8;
            let reg = (ir & 7) as u8;

            let maybe_steps = match op_type {
                0 => crate::instructions::btst::decode_btst_dyn_steps(mode, reg),
                1 => crate::instructions::bchg::decode_bchg_dyn_steps(mode, reg),
                2 => crate::instructions::bclr::decode_bclr_dyn_steps(mode, reg),
                3 => crate::instructions::bset::decode_bset_dyn_steps(mode, reg),
                _ => None,
            };

            if let Some(steps) = maybe_steps {
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src: reg_s,
                    reg_dst: reg,
                };
            }
        }
        op += 1;
    }

    // Phase 4: Static bit manipulation opcodes ($0800..=$08FF)
    let mut op = 0x0800usize;
    while op <= 0x08FF {
        let ir = op as u16;
        let op_type = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        let maybe_steps = match op_type {
            0 => crate::instructions::btst::decode_btst_stat_steps(mode, reg),
            1 => crate::instructions::bchg::decode_bchg_stat_steps(mode, reg),
            2 => crate::instructions::bclr::decode_bclr_stat_steps(mode, reg),
            3 => crate::instructions::bset::decode_bset_stat_steps(mode, reg),
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

                let maybe_steps = match (shift_type, dir) {
                    (0, 0) => crate::instructions::asr::decode_asr_mem_steps(mode, reg),
                    (0, 1) => crate::instructions::asl::decode_asl_mem_steps(mode, reg),
                    (1, 0) => crate::instructions::lsr::decode_lsr_mem_steps(mode, reg),
                    (1, 1) => crate::instructions::lsl::decode_lsl_mem_steps(mode, reg),
                    (2, 0) => crate::instructions::roxr::decode_roxr_mem_steps(mode, reg),
                    (2, 1) => crate::instructions::roxl::decode_roxl_mem_steps(mode, reg),
                    (3, 0) => crate::instructions::ror::decode_ror_mem_steps(mode, reg),
                    (3, 1) => crate::instructions::rol::decode_rol_mem_steps(mode, reg),
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
            let raw_cnt = ((ir >> 9) & 7) as u8;
            let dir = ((ir >> 8) & 1) as u8;
            let size = size_bits;
            let is_reg_count = ((ir >> 5) & 1) != 0;
            let shift_type = ((ir >> 3) & 3) as u8;
            let reg_dst = (ir & 7) as u8;

            let maybe_steps = match (shift_type, dir) {
                (0, 0) => crate::instructions::asr::decode_asr_reg_steps(is_reg_count, size),
                (0, 1) => crate::instructions::asl::decode_asl_reg_steps(is_reg_count, size),
                (1, 0) => crate::instructions::lsr::decode_lsr_reg_steps(is_reg_count, size),
                (1, 1) => crate::instructions::lsl::decode_lsl_reg_steps(is_reg_count, size),
                (2, 0) => crate::instructions::roxr::decode_roxr_reg_steps(is_reg_count, size),
                (2, 1) => crate::instructions::roxl::decode_roxl_reg_steps(is_reg_count, size),
                (3, 0) => crate::instructions::ror::decode_ror_reg_steps(is_reg_count, size),
                (3, 1) => crate::instructions::rol::decode_rol_reg_steps(is_reg_count, size),
                _ => None,
            };

            if let Some(steps) = maybe_steps {
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
        op += 1;
    }

    // Phase 6: AND ($C000..=$CFFF where size < 3), MULU (dir == 0, size == 3), MULS (dir == 1, size == 3)
    let mut op = 0xC000usize;
    while op <= 0xCFFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            if let Some(steps) = crate::instructions::and::decode_and_steps(dir, size, mode, reg) {
                let (reg_src, reg_dst) = if dir == 0 { (reg, reg_d) } else { (reg_d, reg) };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else if dir == 0 {
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

    // Phase 6: OR ($8000..=$8FFF where size < 3), DIVU (dir == 0, size == 3), DIVS (dir == 1, size == 3)
    let mut op = 0x8000usize;
    while op <= 0x8FFF {
        let ir = op as u16;
        let reg_d = ((ir >> 9) & 7) as u8;
        let dir = ((ir >> 8) & 1) as u8;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;

        if size < 3 {
            if let Some(steps) = crate::instructions::or::decode_or_steps(dir, size, mode, reg) {
                let (reg_src, reg_dst) = if dir == 0 { (reg, reg_d) } else { (reg_d, reg) };
                table[op] = OpcodeDescriptor {
                    steps,
                    reg_src,
                    reg_dst,
                };
            }
        } else if dir == 0 {
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

    // Phase 6: ORI opcodes ($0000..=$00FF where size < 3)
    let mut op = 0x0000usize;
    while op <= 0x00FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 6: ANDI opcodes ($0200..=$02FF where size < 3)
    let mut op = 0x0200usize;
    while op <= 0x02FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 6: EORI opcodes ($0A00..=$0AFF where size < 3)
    let mut op = 0x0A00usize;
    while op <= 0x0AFF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 7: CMPI opcodes ($0C00..=$0CFF where size < 3)
    let mut op = 0x0C00usize;
    while op <= 0x0CFF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Phase 7: TST opcodes ($4A00..=$4AFF where size < 3)
    let mut op = 0x4A00usize;
    while op <= 0x4AFF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Batch 1.8: CLR opcodes ($4200..=$42FF where size < 3)
    let mut op = 0x4200usize;
    while op <= 0x42FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Batch 1.8: NEG opcodes ($4400..=$44FF where size < 3)
    let mut op = 0x4400usize;
    while op <= 0x44FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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

    // Batch 1.8: NEGX opcodes ($4000..=$40FF where size < 3)
    let mut op = 0x4000usize;
    while op <= 0x40FF {
        let ir = op as u16;
        let size = ((ir >> 6) & 3) as u8;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
        if size < 3 {
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
    let mut op = 0x4800usize;
    while op <= 0x483F {
        let ir = op as u16;
        let mode = ((ir >> 3) & 7) as u8;
        let reg = (ir & 7) as u8;
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
    while reg < 8 {
        let op_w = 0x4880 | (reg as usize);
        table[op_w] = OpcodeDescriptor {
            steps: &crate::instructions::ext::STEPS_EXT_W,
            reg_src: 0,
            reg_dst: reg,
        };
        let op_l = 0x48C0 | (reg as usize);
        table[op_l] = OpcodeDescriptor {
            steps: &crate::instructions::ext::STEPS_EXT_L,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.8: ABCD ($C000..=$CFFF where bits 15..12 == 0xC, bit 8 == 1, bits 7..4 == 0)
    let mut op = 0xC000usize;
    while op <= 0xCFFF {
        if (op & 0xF1F0) == 0xC100 {
            let rx = ((op >> 9) & 7) as u8;
            let ry = (op & 7) as u8;
            let is_mem = (op & 8) != 0;
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
    let mut op = 0x8000usize;
    while op <= 0x8FFF {
        if (op & 0xF1F0) == 0x8100 {
            let rx = ((op >> 9) & 7) as u8;
            let ry = (op & 7) as u8;
            let is_mem = (op & 8) != 0;
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
    while reg < 8 {
        let op = 0x4840 | (reg as usize);
        table[op] = OpcodeDescriptor {
            steps: &crate::instructions::swap::STEPS_SWAP,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.10: EXG ($C000..=$CFFF where bits 15..12 == 0xC, bit 8 == 1)
    let mut op = 0xC000usize;
    while op <= 0xCFFF {
        if (op & 0xF1F8) == 0xC140 {
            // EXG Dx, Dy (opmode 0b01000)
            let rx = ((op >> 9) & 7) as u8;
            let ry = (op & 7) as u8;
            table[op] = OpcodeDescriptor {
                steps: &crate::instructions::exg::STEPS_EXG_DX_DY,
                reg_src: ry,
                reg_dst: rx,
            };
        } else if (op & 0xF1F8) == 0xC148 {
            // EXG Ax, Ay (opmode 0b01001)
            let rx = ((op >> 9) & 7) as u8;
            let ry = (op & 7) as u8;
            table[op] = OpcodeDescriptor {
                steps: &crate::instructions::exg::STEPS_EXG_AX_AY,
                reg_src: ry,
                reg_dst: rx,
            };
        } else if (op & 0xF1F8) == 0xC188 {
            // EXG Dx, Ay (opmode 0b10001)
            let rx = ((op >> 9) & 7) as u8;
            let ry = (op & 7) as u8;
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
    while reg < 8 {
        let op_link = 0x4E50 | (reg as usize);
        table[op_link] = OpcodeDescriptor {
            steps: &crate::instructions::link::STEPS_LINK,
            reg_src: 0,
            reg_dst: reg,
        };
        let op_unlk = 0x4E58 | (reg as usize);
        table[op_unlk] = OpcodeDescriptor {
            steps: &crate::instructions::unlk::STEPS_UNLK,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.10: LEA ($41C0..=$4FFF where (op & 0xF1C0) == 0x41C0)
    let mut op = 0x41C0usize;
    while op <= 0x4FFF {
        if (op & 0xF1C0) == 0x41C0 {
            let an = ((op >> 9) & 7) as u8;
            let mode = ((op >> 3) & 7) as u8;
            let reg = (op & 7) as u8;
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
    let mut op = 0x4180usize;
    while op <= 0x4FFF {
        if (op & 0xF1C0) == 0x4180 {
            let dn = ((op >> 9) & 7) as u8;
            let mode = ((op >> 3) & 7) as u8;
            let reg = (op & 7) as u8;
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
    table[0x4E70] = OpcodeDescriptor {
        steps: &crate::instructions::reset::STEPS_RESET,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x4E72] = OpcodeDescriptor {
        steps: &crate::instructions::stop::STEPS_STOP,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x4E73] = OpcodeDescriptor {
        steps: &crate::instructions::rte::STEPS_RTE,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x4E76] = OpcodeDescriptor {
        steps: &crate::instructions::trapv::STEPS_TRAPV,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x4E77] = OpcodeDescriptor {
        steps: &crate::instructions::rtr::STEPS_RTR,
        reg_src: 0,
        reg_dst: 0,
    };

    let mut reg = 0u8;
    while reg < 8 {
        table[0x4E60 | (reg as usize)] = OpcodeDescriptor {
            steps: &crate::instructions::move_usp::STEPS_MOVE_TO_USP,
            reg_src: reg,
            reg_dst: 0,
        };
        table[0x4E68 | (reg as usize)] = OpcodeDescriptor {
            steps: &crate::instructions::move_usp::STEPS_MOVE_FROM_USP,
            reg_src: 0,
            reg_dst: reg,
        };
        reg += 1;
    }

    // Batch 1.11: Immediate Logic to CCR / SR
    table[0x003C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ORI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x007C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ORI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x023C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ANDI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x027C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_ANDI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x0A3C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_EORI_TO_CCR,
        reg_src: 0,
        reg_dst: 0,
    };
    table[0x0A7C] = OpcodeDescriptor {
        steps: &crate::instructions::logic_sr_ccr::STEPS_EORI_TO_SR,
        reg_src: 0,
        reg_dst: 0,
    };

    // Batch 1.11: MOVE <ea>, CCR ($44C0..=$44FF)
    let mut op = 0x44C0usize;
    while op <= 0x44FF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x46C0usize;
    while op <= 0x46FF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x40C0usize;
    while op <= 0x40FF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
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
    let mut op = 0x4AC0usize;
    while op <= 0x4AFF {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        if let Some(steps) = crate::instructions::tas::decode_tas_steps(mode, reg) {
            table[op] = OpcodeDescriptor {
                steps,
                reg_src: reg,
                reg_dst: reg,
            };
        }
        op += 1;
    }

    table
}

/// Master 65,536 opcode descriptor table resident in host .rodata
pub static OPCODE_DESCRIPTOR_TABLE: [OpcodeDescriptor; 65536] = build_opcode_descriptor_table();
