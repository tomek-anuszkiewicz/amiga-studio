//! ALU, Compare, Bitwise, Immediate, Multiply/Divide and Shift/Rotate disassembler module

const BIT_TEST: u16 = 0;
const BIT_CHANGE: u16 = 1;
const BIT_CLEAR: u16 = 2;
const BIT_SET: u16 = 3;
const MOVEP_LOAD_WORD: u16 = 0;
const MOVEP_LOAD_LONG: u16 = 1;
const MOVEP_STORE_WORD: u16 = 2;
const IMMEDIATE_OR: u16 = 0;
const IMMEDIATE_AND: u16 = 1;
const IMMEDIATE_SUB: u16 = 2;
const IMMEDIATE_ADD: u16 = 3;
const IMMEDIATE_EOR: u16 = 5;
const IMMEDIATE_CMP: u16 = 6;
const SHIFT_ARITHMETIC: u16 = 0;
const SHIFT_LOGICAL: u16 = 1;
const SHIFT_ROTATE_EXTEND: u16 = 2;
const SHIFT_ROTATE: u16 = 3;
const OPMODE_SIZE_BYTE: u8 = 0;
const OPMODE_SIZE_WORD: u8 = 1;
const OPMODE_SIZE_LONG: u8 = 2;
const MASK_OPMODE_SIZE: u8 = 3;
const BIT_EXG_MODE_SHIFT: u16 = 3;
const BIT_SHIFT_MEMORY_TYPE_SHIFT: u16 = 9;
const BIT_SHIFT_REGISTER_TYPE_SHIFT: u16 = 3;
const MASK_SHIFT_TYPE: u16 = 3;
use crate::encoding::opcode;
const BIT_SHIFT_LEFT: u16 = 0x0100;
const EXTEND_ARITHMETIC_PATTERN: u16 = 0x0100;
const EXG_CLASS_PATTERN: u16 = 0xC100;
const BIT_BCD_MEMORY_MODE: u16 = 0x0008;
const IMMEDIATE_LINE: u16 = 0x0000;
const STATIC_BIT_PATTERN: u16 = 0x0800;
const DYNAMIC_BIT_PATTERN: u16 = 0x0100;
const ORI_CCR: u16 = 0x003C;
const ORI_SR: u16 = 0x007C;
const ANDI_CCR: u16 = 0x023C;
const ANDI_SR: u16 = 0x027C;
const EORI_CCR: u16 = 0x0A3C;
const EORI_SR: u16 = 0x0A7C;
const CMP_EOR_LINE: u16 = 0xB000;
const MASK_CMPM: u16 = 0xF138;
const CMPM_PATTERN: u16 = 0xB108;
const AND_MUL_LINE: u16 = 0xC000;
const OR_DIV_LINE: u16 = 0x8000;
const MASK_EXG_CLASS: u16 = 0xF130;
const MASK_EXG_MODE: u16 = 0x001F;
const OR_LINE_VALUE: u16 = 0x0008;
const SUB_LINE_VALUE: u16 = 0x0009;
const EXG_MIXED_MODE: u16 = 0x0011;
const MULU_PATTERN: u16 = 0xC0C0;
const MULS_PATTERN: u16 = 0xC1C0;
const DIVU_PATTERN: u16 = 0x80C0;
const DIVS_PATTERN: u16 = 0x81C0;
const MASK_BCD: u16 = 0xF1F0;
const ABCD_PATTERN: u16 = 0xC100;
const SBCD_PATTERN: u16 = 0x8100;
const ADD_LINE_VALUE: u16 = 0x000D;
const AND_LINE_VALUE: u16 = 0x000C;
const MASK_EXTEND_ARITHMETIC: u16 = 0x0130;
const SHIFT_LINE: u16 = 0xE000;
const MASK_MEMORY_SHIFT: u16 = 0xF8C0;
const MEMORY_SHIFT_PATTERN: u16 = 0xE0C0;
const BIT_REGISTER_SHIFT_COUNT: u16 = 0x0020;
use crate::ea::{
    format_ea, format_immediate, is_data_alterable_ea, is_data_ea, is_memory_alterable_ea,
};
use crate::encoding::{extended_mode, field, mode, opmode, size};

const SHIFT_IMMEDIATE_ZERO_VALUE: u8 = 8;

const BIT_BIT_TYPE_SHIFT: u16 = 6;
const MASK_BIT_TYPE: u16 = 3;

const EXG_DATA_MODE: u16 = 8;
const EXG_ADDRESS_MODE: u16 = 9;

/// Attempts to disassemble arithmetic, logic, compare, bit, multiply/divide, and shift/rotate instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub(crate) fn try_disassemble_alu(
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. Immediate operations ($0000..$0FFF: ORI, ANDI, SUBI, ADDI, EORI, CMPI, BitOps)
    if (op & opcode::MASK_OPCODE_LINE) == IMMEDIATE_LINE {
        // Bit operations with immediate bit number ($0800..$08FF)
        if (op & opcode::MASK_OPCODE_HIGH) == STATIC_BIT_PATTERN {
            let bit_type = (op >> BIT_BIT_TYPE_SHIFT) & MASK_BIT_TYPE;
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            let reg = (op & field::MASK_REGISTER) as u8;
            let valid_ea = if bit_type == BIT_TEST {
                is_data_ea(mode, reg)
                    && !(mode == mode::EXTENDED && reg == extended_mode::IMMEDIATE)
            } else {
                is_data_alterable_ea(mode, reg)
            };
            if !valid_ea {
                return None;
            }
            let mnem = match bit_type {
                BIT_TEST => "BTST",
                BIT_CHANGE => "BCHG",
                BIT_CLEAR => "BCLR",
                BIT_SET => "BSET",
                _ => "BIT",
            };
            let bit_val = (next_word() & 0xFF) as u8;
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("#{}, {}", bit_val, ea_str)));
        }

        // MOVEP occupies the address-register-direct slot of dynamic bit operations.
        if (op & opcode::MASK_REGISTER_LINE) == DYNAMIC_BIT_PATTERN {
            let dn = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
            let bit_type = (op >> BIT_BIT_TYPE_SHIFT) & MASK_BIT_TYPE;
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            if mode == mode::ADDRESS_REGISTER {
                let ea_str = format_ea(
                    mode::DISPLACEMENT,
                    (op & field::MASK_REGISTER) as u8,
                    &mut next_word,
                );
                let (mnem, ops) = match bit_type {
                    MOVEP_LOAD_WORD => ("MOVEP.W", format!("{}, D{}", ea_str, dn)),
                    MOVEP_LOAD_LONG => ("MOVEP.L", format!("{}, D{}", ea_str, dn)),
                    MOVEP_STORE_WORD => ("MOVEP.W", format!("D{}, {}", dn, ea_str)),
                    _ => ("MOVEP.L", format!("D{}, {}", dn, ea_str)),
                };
                return Some((mnem, ops));
            }
            let reg = (op & field::MASK_REGISTER) as u8;
            let valid_ea = if bit_type == BIT_TEST {
                is_data_ea(mode, reg)
            } else {
                is_data_alterable_ea(mode, reg)
            };
            if !valid_ea {
                return None;
            }
            // Bit operations with dynamic bit in Dn ($0100..$01FF).
            let mnem = match bit_type {
                BIT_TEST => "BTST",
                BIT_CHANGE => "BCHG",
                BIT_CLEAR => "BCLR",
                BIT_SET => "BSET",
                _ => "BIT",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("D{}, {}", dn, ea_str)));
        }

        // Special immediate to CCR/SR
        match op {
            ORI_CCR => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("ORI.B", format!("{}, CCR", imm)));
            }
            ORI_SR => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("ORI.W", format!("{}, SR", imm)));
            }
            ANDI_CCR => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("ANDI.B", format!("{}, CCR", imm)));
            }
            ANDI_SR => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("ANDI.W", format!("{}, SR", imm)));
            }
            EORI_CCR => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("EORI.B", format!("{}, CCR", imm)));
            }
            EORI_SR => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("EORI.W", format!("{}, SR", imm)));
            }
            _ => {}
        }

        let op_type = (op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER;
        let size_bits = (op >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE;
        if size_bits < size::SPECIAL {
            let (sz_bytes, sz_str) = match size_bits {
                size::BYTE => (1, ".B"),
                size::WORD => (2, ".W"),
                size::LONG => (4, ".L"),
                _ => (2, ".W"),
            };
            let base_mnem = match op_type {
                IMMEDIATE_OR => "ORI",
                IMMEDIATE_AND => "ANDI",
                IMMEDIATE_SUB => "SUBI",
                IMMEDIATE_ADD => "ADDI",
                IMMEDIATE_EOR => "EORI",
                IMMEDIATE_CMP => "CMPI",
                _ => "",
            };
            if !base_mnem.is_empty() {
                let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
                let reg = (op & field::MASK_REGISTER) as u8;
                let valid_ea = is_data_alterable_ea(mode, reg);
                if !valid_ea {
                    return None;
                }
                let full_mnem = match (base_mnem, sz_str) {
                    ("ORI", ".B") => "ORI.B",
                    ("ORI", ".W") => "ORI.W",
                    ("ORI", ".L") => "ORI.L",
                    ("ANDI", ".B") => "ANDI.B",
                    ("ANDI", ".W") => "ANDI.W",
                    ("ANDI", ".L") => "ANDI.L",
                    ("SUBI", ".B") => "SUBI.B",
                    ("SUBI", ".W") => "SUBI.W",
                    ("SUBI", ".L") => "SUBI.L",
                    ("ADDI", ".B") => "ADDI.B",
                    ("ADDI", ".W") => "ADDI.W",
                    ("ADDI", ".L") => "ADDI.L",
                    ("EORI", ".B") => "EORI.B",
                    ("EORI", ".W") => "EORI.W",
                    ("EORI", ".L") => "EORI.L",
                    ("CMPI", ".B") => "CMPI.B",
                    ("CMPI", ".W") => "CMPI.W",
                    ("CMPI", ".L") => "CMPI.L",
                    _ => base_mnem,
                };
                let imm = format_immediate(sz_bytes, &mut next_word);
                let ea_str = format_ea(
                    ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                    (op & field::MASK_REGISTER) as u8,
                    &mut next_word,
                );
                return Some((full_mnem, format!("{}, {}", imm, ea_str)));
            }
        }
    }

    // 2. Compare, EOR, CMPA, CMPM ($B000..$BFFF)
    if (op & opcode::MASK_OPCODE_LINE) == CMP_EOR_LINE {
        let reg_d = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let opmode = ((op >> field::BIT_OPMODE_SHIFT) & field::MASK_OPMODE) as u8;
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;

        // CMPA.W / CMPA.L
        if opmode == opmode::ADDRESS_WORD || opmode == opmode::ADDRESS_LONG {
            if !is_data_ea(mode, reg) && mode != mode::ADDRESS_REGISTER {
                return None;
            }
            let mnem = if opmode == opmode::ADDRESS_WORD {
                "CMPA.W"
            } else {
                "CMPA.L"
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }

        // CMPM (Ay)+, (Ax)+
        if (op & MASK_CMPM) == CMPM_PATTERN {
            let mnem = match opmode & MASK_OPMODE_SIZE {
                OPMODE_SIZE_BYTE => "CMPM.B",
                OPMODE_SIZE_WORD => "CMPM.W",
                OPMODE_SIZE_LONG => "CMPM.L",
                _ => "CMPM",
            };
            let ax = reg_d;
            let ay = (op & field::MASK_REGISTER) as u8;
            return Some((mnem, format!("(A{})+, (A{})+", ay, ax)));
        }

        // EOR Dn, <ea>
        if opmode == opmode::BYTE_TO_EA
            || opmode == opmode::WORD_TO_EA
            || opmode == opmode::LONG_TO_EA
        {
            if !is_data_alterable_ea(mode, reg) {
                return None;
            }
            let mnem = match opmode {
                opmode::BYTE_TO_EA => "EOR.B",
                opmode::WORD_TO_EA => "EOR.W",
                opmode::LONG_TO_EA => "EOR.L",
                _ => "EOR",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("D{}, {}", reg_d, ea_str)));
        }

        // CMP <ea>, Dn
        if opmode <= opmode::EA_TO_LONG {
            if !(is_data_ea(mode, reg)
                || (mode == mode::ADDRESS_REGISTER && opmode != opmode::EA_TO_BYTE))
            {
                return None;
            }
            let mnem = match opmode {
                opmode::EA_TO_BYTE => "CMP.B",
                opmode::EA_TO_WORD => "CMP.W",
                opmode::EA_TO_LONG => "CMP.L",
                _ => "CMP",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("{}, D{}", ea_str, reg_d)));
        }
    }

    // 3. Multiply, Divide & EXG ($C000..$CFFF, $8000..$8FFF)
    if (op & opcode::MASK_OPCODE_LINE) == AND_MUL_LINE
        || (op & opcode::MASK_OPCODE_LINE) == OR_DIV_LINE
    {
        let is_c = (op & opcode::MASK_OPCODE_LINE) == AND_MUL_LINE;
        let reg = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;

        // EXG
        if is_c && (op & MASK_EXG_CLASS) == EXG_CLASS_PATTERN {
            let mode = (op >> BIT_EXG_MODE_SHIFT) & MASK_EXG_MODE;
            let ry = (op & field::MASK_REGISTER) as u8;
            let rx = reg;
            match mode {
                EXG_DATA_MODE => return Some(("EXG", format!("D{}, D{}", rx, ry))),
                EXG_ADDRESS_MODE => return Some(("EXG", format!("A{}, A{}", rx, ry))),
                EXG_MIXED_MODE => return Some(("EXG", format!("D{}, A{}", rx, ry))),
                _ => {}
            }
        }

        // MULU / MULS / DIVU / DIVS
        if (op & opcode::MASK_REGISTER_EA_OPCODE) == MULU_PATTERN {
            if !is_data_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
            ) {
                return None;
            }
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some(("MULU.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & opcode::MASK_REGISTER_EA_OPCODE) == MULS_PATTERN {
            if !is_data_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
            ) {
                return None;
            }
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some(("MULS.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & opcode::MASK_REGISTER_EA_OPCODE) == DIVU_PATTERN {
            if !is_data_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
            ) {
                return None;
            }
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some(("DIVU.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & opcode::MASK_REGISTER_EA_OPCODE) == DIVS_PATTERN {
            if !is_data_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
            ) {
                return None;
            }
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some(("DIVS.W", format!("{}, D{}", ea_str, reg)));
        }
    }

    // ABCD & SBCD ($C100..$C10F, $8100..$810F)
    if (op & MASK_BCD) == ABCD_PATTERN || (op & MASK_BCD) == SBCD_PATTERN {
        let is_abcd = (op & opcode::MASK_OPCODE_LINE) == AND_MUL_LINE;
        let mnem = if is_abcd { "ABCD" } else { "SBCD" };
        let rx = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let ry = (op & field::MASK_REGISTER) as u8;
        let is_mem = (op & BIT_BCD_MEMORY_MODE) != 0;
        let ops = if is_mem {
            format!("-(A{}), -(A{})", ry, rx)
        } else {
            format!("D{}, D{}", ry, rx)
        };
        return Some((mnem, ops));
    }

    // 4. ADD, ADDA, ADDX, SUB, SUBA, SUBX, AND, OR
    let op_group = (op >> field::BIT_OPCODE_LINE_SHIFT) & field::MASK_OPCODE_LINE_VALUE;
    if matches!(
        op_group,
        ADD_LINE_VALUE | SUB_LINE_VALUE | AND_LINE_VALUE | OR_LINE_VALUE
    ) {
        let base_name = match op_group {
            ADD_LINE_VALUE => "ADD",
            SUB_LINE_VALUE => "SUB",
            AND_LINE_VALUE => "AND",
            _ => "OR",
        };
        let reg_d = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let opmode = ((op >> field::BIT_OPMODE_SHIFT) & field::MASK_OPMODE) as u8;

        // ADDA / SUBA
        let is_add_or_sub = matches!(op_group, ADD_LINE_VALUE | SUB_LINE_VALUE);
        let is_address_arithmetic =
            is_add_or_sub && matches!(opmode, opmode::ADDRESS_WORD | opmode::ADDRESS_LONG);
        if is_address_arithmetic {
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            let reg = (op & field::MASK_REGISTER) as u8;
            if !is_data_ea(mode, reg) && mode != mode::ADDRESS_REGISTER {
                return None;
            }
            let mnem = match (op_group, opmode) {
                (ADD_LINE_VALUE, opmode::ADDRESS_WORD) => "ADDA.W",
                (ADD_LINE_VALUE, opmode::ADDRESS_LONG) => "ADDA.L",
                (SUB_LINE_VALUE, opmode::ADDRESS_WORD) => "SUBA.W",
                (SUB_LINE_VALUE, opmode::ADDRESS_LONG) => "SUBA.L",
                _ => "ADDA",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }

        // ADDX / SUBX
        let is_extend_arithmetic =
            is_add_or_sub && (op & MASK_EXTEND_ARITHMETIC) == EXTEND_ARITHMETIC_PATTERN;
        if is_extend_arithmetic {
            let sz = match opmode & MASK_OPMODE_SIZE {
                OPMODE_SIZE_BYTE => ".B",
                OPMODE_SIZE_WORD => ".W",
                OPMODE_SIZE_LONG => ".L",
                _ => "",
            };
            let is_mem = (op & BIT_BCD_MEMORY_MODE) != 0;
            let rx = reg_d;
            let ry = (op & field::MASK_REGISTER) as u8;
            let mnem = if op_group == ADD_LINE_VALUE {
                match sz {
                    ".B" => "ADDX.B",
                    ".W" => "ADDX.W",
                    ".L" => "ADDX.L",
                    _ => "ADDX",
                }
            } else {
                match sz {
                    ".B" => "SUBX.B",
                    ".W" => "SUBX.W",
                    ".L" => "SUBX.L",
                    _ => "SUBX",
                }
            };
            let ops = if is_mem {
                format!("-(A{}), -(A{})", ry, rx)
            } else {
                format!("D{}, D{}", ry, rx)
            };
            return Some((mnem, ops));
        }

        let (sz_str, ea_is_source) = match opmode {
            opmode::EA_TO_BYTE => (".B", true),
            opmode::EA_TO_WORD => (".W", true),
            opmode::EA_TO_LONG => (".L", true),
            opmode::BYTE_TO_EA => (".B", false),
            opmode::WORD_TO_EA => (".W", false),
            opmode::LONG_TO_EA => (".L", false),
            _ => return None,
        };

        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        let valid_ea = if ea_is_source {
            is_data_ea(mode, reg)
                || (is_add_or_sub && opmode != opmode::EA_TO_BYTE && mode == mode::ADDRESS_REGISTER)
        } else {
            is_memory_alterable_ea(mode, reg)
        };
        if !valid_ea {
            return None;
        }

        let ea_str = format_ea(
            ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
            (op & field::MASK_REGISTER) as u8,
            &mut next_word,
        );
        let ops = if ea_is_source {
            format!("{}, D{}", ea_str, reg_d)
        } else {
            format!("D{}, {}", reg_d, ea_str)
        };

        let full_mnem = match (base_name, sz_str) {
            ("ADD", ".B") => "ADD.B",
            ("ADD", ".W") => "ADD.W",
            ("ADD", ".L") => "ADD.L",
            ("SUB", ".B") => "SUB.B",
            ("SUB", ".W") => "SUB.W",
            ("SUB", ".L") => "SUB.L",
            ("AND", ".B") => "AND.B",
            ("AND", ".W") => "AND.W",
            ("AND", ".L") => "AND.L",
            ("OR", ".B") => "OR.B",
            ("OR", ".W") => "OR.W",
            ("OR", ".L") => "OR.L",
            _ => "OP",
        };

        return Some((full_mnem, ops));
    }

    // 5. Shifts & Rotates ($E000..$EFFF)
    if (op & opcode::MASK_OPCODE_LINE) == SHIFT_LINE {
        // Memory shifts: op & 0xF8C0 == 0xE0C0
        if (op & MASK_MEMORY_SHIFT) == MEMORY_SHIFT_PATTERN {
            if !is_memory_alterable_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
            ) {
                return None;
            }
            let dir_left = (op & BIT_SHIFT_LEFT) != 0;
            let shift_type = (op >> BIT_SHIFT_MEMORY_TYPE_SHIFT) & MASK_SHIFT_TYPE;
            let mnem = match (shift_type, dir_left) {
                (SHIFT_ARITHMETIC, false) => "ASR.W",
                (SHIFT_ARITHMETIC, true) => "ASL.W",
                (SHIFT_LOGICAL, false) => "LSR.W",
                (SHIFT_LOGICAL, true) => "LSL.W",
                (SHIFT_ROTATE_EXTEND, false) => "ROXR.W",
                (SHIFT_ROTATE_EXTEND, true) => "ROXL.W",
                (SHIFT_ROTATE, false) => "ROR.W",
                (SHIFT_ROTATE, true) => "ROL.W",
                _ => "SHIFT",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, ea_str));
        }

        // Register shifts
        let size_bits = (op >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE;
        if size_bits < size::SPECIAL {
            let dir_left = (op & BIT_SHIFT_LEFT) != 0;
            let shift_type = (op >> BIT_SHIFT_REGISTER_TYPE_SHIFT) & MASK_SHIFT_TYPE;
            let is_reg = (op & BIT_REGISTER_SHIFT_COUNT) != 0;
            let count_raw = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
            let target_reg = (op & field::MASK_REGISTER) as u8;

            let sz = match size_bits {
                size::BYTE => ".B",
                size::WORD => ".W",
                size::LONG => ".L",
                _ => "",
            };

            let mnem = match (shift_type, dir_left, sz) {
                (SHIFT_ARITHMETIC, false, ".B") => "ASR.B",
                (SHIFT_ARITHMETIC, false, ".W") => "ASR.W",
                (SHIFT_ARITHMETIC, false, ".L") => "ASR.L",
                (SHIFT_ARITHMETIC, true, ".B") => "ASL.B",
                (SHIFT_ARITHMETIC, true, ".W") => "ASL.W",
                (SHIFT_ARITHMETIC, true, ".L") => "ASL.L",
                (SHIFT_LOGICAL, false, ".B") => "LSR.B",
                (SHIFT_LOGICAL, false, ".W") => "LSR.W",
                (SHIFT_LOGICAL, false, ".L") => "LSR.L",
                (SHIFT_LOGICAL, true, ".B") => "LSL.B",
                (SHIFT_LOGICAL, true, ".W") => "LSL.W",
                (SHIFT_LOGICAL, true, ".L") => "LSL.L",
                (SHIFT_ROTATE_EXTEND, false, ".B") => "ROXR.B",
                (SHIFT_ROTATE_EXTEND, false, ".W") => "ROXR.W",
                (SHIFT_ROTATE_EXTEND, false, ".L") => "ROXR.L",
                (SHIFT_ROTATE_EXTEND, true, ".B") => "ROXL.B",
                (SHIFT_ROTATE_EXTEND, true, ".W") => "ROXL.W",
                (SHIFT_ROTATE_EXTEND, true, ".L") => "ROXL.L",
                (SHIFT_ROTATE, false, ".B") => "ROR.B",
                (SHIFT_ROTATE, false, ".W") => "ROR.W",
                (SHIFT_ROTATE, false, ".L") => "ROR.L",
                (SHIFT_ROTATE, true, ".B") => "ROL.B",
                (SHIFT_ROTATE, true, ".W") => "ROL.W",
                (SHIFT_ROTATE, true, ".L") => "ROL.L",
                _ => "SHIFT",
            };

            let count_str = if is_reg {
                format!("D{}", count_raw)
            } else {
                let cnt = if count_raw == 0 {
                    SHIFT_IMMEDIATE_ZERO_VALUE
                } else {
                    count_raw
                };
                format!("#{}", cnt)
            };

            return Some((mnem, format!("{}, D{}", count_str, target_reg)));
        }
    }

    None
}
