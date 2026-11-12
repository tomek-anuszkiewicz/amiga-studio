//! Encoded M68000 fields used by the execution decoder.

pub mod opcode {
    pub const BIT_BCD_MEMORY_MODE: usize = 8;
    pub const TRAP_FIRST: usize = 0x4E40;
    pub const TRAP_LAST: usize = 0x4E4F;
    pub const BRANCH_FIRST: usize = 0x6000;
    pub const BRANCH_LAST: usize = 0x6FFF;
    pub const PEA_FIRST: usize = 0x4840;
    pub const PEA_LAST: usize = 0x487F;
    pub const JSR_FIRST: usize = 0x4E90;
    pub const JSR_LAST: usize = 0x4EBF;
    pub const JMP_FIRST: usize = 0x4ED0;
    pub const JMP_LAST: usize = 0x4EFF;
    pub const MOVEM_STORE_FIRST: usize = 0x4880;
    pub const MOVEM_STORE_LAST: usize = 0x48FF;
    pub const MOVEM_LOAD_FIRST: usize = 0x4C80;
    pub const MOVEM_LOAD_LAST: usize = 0x4CFF;
    pub const MOVE_LONG_FIRST: usize = 0x2000;
    pub const MOVE_LONG_LAST: usize = 0x2FFF;
    pub const MOVE_BYTE_FIRST: usize = 0x1000;
    pub const MOVE_BYTE_LAST: usize = 0x1FFF;
    pub const MOVE_WORD_FIRST: usize = 0x3000;
    pub const MOVE_WORD_LAST: usize = 0x3FFF;
    pub const MOVEQ_FIRST: usize = 0x7000;
    pub const MOVEQ_LAST: usize = 0x7FFF;
    pub const ADD_FIRST: usize = 0xD000;
    pub const ADD_LAST: usize = 0xDFFF;
    pub const SUB_FIRST: usize = 0x9000;
    pub const SUB_LAST: usize = 0x9FFF;
    pub const CMP_EOR_FIRST: usize = 0xB000;
    pub const CMP_EOR_LAST: usize = 0xBFFF;
    pub const QUICK_SCC_DBCC_FIRST: usize = 0x5000;
    pub const QUICK_SCC_DBCC_LAST: usize = 0x5FFF;
    pub const ADDI_FIRST: usize = 0x0600;
    pub const ADDI_LAST: usize = 0x06FF;
    pub const SUBI_FIRST: usize = 0x0400;
    pub const SUBI_LAST: usize = 0x04FF;
    pub const NOT_FIRST: usize = 0x4600;
    pub const NOT_LAST: usize = 0x46FF;
    pub const DYNAMIC_BIT_FIRST: usize = 0x0100;
    pub const DYNAMIC_BIT_LAST: usize = 0x0FFF;
    pub const STATIC_BIT_FIRST: usize = 0x0800;
    pub const STATIC_BIT_LAST: usize = 0x08FF;
    pub const SHIFT_FIRST: usize = 0xE000;
    pub const SHIFT_LAST: usize = 0xEFFF;
    pub const LINE_C_FIRST: usize = 0xC000;
    pub const LINE_C_LAST: usize = 0xCFFF;
    pub const LINE_8_FIRST: usize = 0x8000;
    pub const LINE_8_LAST: usize = 0x8FFF;
    pub const ORI_FIRST: usize = 0x0000;
    pub const ORI_LAST: usize = 0x00FF;
    pub const ANDI_FIRST: usize = 0x0200;
    pub const ANDI_LAST: usize = 0x02FF;
    pub const EORI_FIRST: usize = 0x0A00;
    pub const EORI_LAST: usize = 0x0AFF;
    pub const CMPI_FIRST: usize = 0x0C00;
    pub const CMPI_LAST: usize = 0x0CFF;
    pub const TST_FIRST: usize = 0x4A00;
    pub const TST_LAST: usize = 0x4AFF;
    pub const CLR_FIRST: usize = 0x4200;
    pub const CLR_LAST: usize = 0x42FF;
    pub const NEG_FIRST: usize = 0x4400;
    pub const NEG_LAST: usize = 0x44FF;
    pub const NEGX_FIRST: usize = 0x4000;
    pub const NEGX_LAST: usize = 0x40FF;
    pub const NBCD_FIRST: usize = 0x4800;
    pub const NBCD_LAST: usize = 0x483F;
    pub const LEA_FIRST: usize = 0x41C0;
    pub const LEA_LAST: usize = 0x4FFF;
    pub const CHK_FIRST: usize = 0x4180;
    pub const CHK_LAST: usize = 0x4FFF;
    pub const MOVE_TO_CCR_FIRST: usize = 0x44C0;
    pub const MOVE_TO_CCR_LAST: usize = 0x44FF;
    pub const MOVE_TO_SR_FIRST: usize = 0x46C0;
    pub const MOVE_TO_SR_LAST: usize = 0x46FF;
    pub const MOVE_FROM_SR_FIRST: usize = 0x40C0;
    pub const MOVE_FROM_SR_LAST: usize = 0x40FF;
    pub const TAS_FIRST: usize = 0x4AC0;
    pub const TAS_LAST: usize = 0x4AFF;
    pub const NOP_OPCODE: usize = 0x4E71;
    pub const RTS_OPCODE: usize = 0x4E75;
    pub const MASK_DYNAMIC_BIT: usize = 0xF100;
    pub const DYNAMIC_BIT_PATTERN: usize = 0x0100;
    pub const EXT_WORD_OPCODE: usize = 0x4880;
    pub const EXT_LONG_OPCODE: usize = 0x48C0;
    pub const MASK_BCD_OPERATION: usize = 0xF1F0;
    pub const ABCD_PATTERN: usize = 0xC100;
    pub const SBCD_PATTERN: usize = 0x8100;
    pub const SWAP_OPCODE: usize = 0x4840;
    pub const MASK_EXG: usize = 0xF1F8;
    pub const EXG_DATA_PATTERN: usize = 0xC140;
    pub const EXG_ADDRESS_PATTERN: usize = 0xC148;
    pub const EXG_MIXED_PATTERN: usize = 0xC188;
    pub const LINK_OPCODE: usize = 0x4E50;
    pub const UNLK_OPCODE: usize = 0x4E58;
    pub const MASK_REGISTER_EA: usize = 0xF1C0;
    pub const LEA_PATTERN: usize = 0x41C0;
    pub const CHK_PATTERN: usize = 0x4180;
    pub const RESET_OPCODE: usize = 0x4E70;
    pub const STOP_OPCODE: usize = 0x4E72;
    pub const RTE_OPCODE: usize = 0x4E73;
    pub const TRAPV_OPCODE: usize = 0x4E76;
    pub const RTR_OPCODE: usize = 0x4E77;
    pub const MOVE_TO_USP_OPCODE: usize = 0x4E60;
    pub const MOVE_FROM_USP_OPCODE: usize = 0x4E68;
    pub const ORI_CCR_OPCODE: usize = 0x003C;
    pub const ORI_SR_OPCODE: usize = 0x007C;
    pub const ANDI_CCR_OPCODE: usize = 0x023C;
    pub const ANDI_SR_OPCODE: usize = 0x027C;
    pub const EORI_CCR_OPCODE: usize = 0x0A3C;
    pub const EORI_SR_OPCODE: usize = 0x0A7C;
}

pub mod mode {
    pub const DATA_REGISTER: u8 = 0;
    pub const ADDRESS_REGISTER: u8 = 1;
    pub const INDIRECT: u8 = 2;
    pub const POSTINCREMENT: u8 = 3;
    pub const PREDECREMENT: u8 = 4;
    pub const DISPLACEMENT: u8 = 5;
    pub const INDEXED: u8 = 6;
    pub const EXTENDED: u8 = 7;
}

pub mod extended_mode {
    pub const ABSOLUTE_WORD: u8 = 0;
    pub const ABSOLUTE_LONG: u8 = 1;
    pub const PC_DISPLACEMENT: u8 = 2;
    pub const PC_INDEXED: u8 = 3;
    pub const IMMEDIATE: u8 = 4;
}

pub mod operand_size {
    pub const BYTE: u8 = 0;
    pub const WORD: u8 = 1;
    pub const LONG: u8 = 2;
    pub const SPECIAL: u8 = 3;
}

pub mod condition {
    pub const MASK_CONDITION: u8 = 0x0F;
    pub const BIT_SHIFT: u8 = 8;
    pub const TRUE: u8 = 0x0;
    pub const FALSE: u8 = 0x1;
    pub const HIGH: u8 = 0x2;
    pub const LOW_OR_SAME: u8 = 0x3;
    pub const CARRY_CLEAR: u8 = 0x4;
    pub const CARRY_SET: u8 = 0x5;
    pub const NOT_EQUAL: u8 = 0x6;
    pub const EQUAL: u8 = 0x7;
    pub const OVERFLOW_CLEAR: u8 = 0x8;
    pub const OVERFLOW_SET: u8 = 0x9;
    pub const PLUS: u8 = 0xA;
    pub const MINUS: u8 = 0xB;
    pub const GREATER_OR_EQUAL: u8 = 0xC;
    pub const LESS_THAN: u8 = 0xD;
    pub const GREATER_THAN: u8 = 0xE;
    pub const LESS_OR_EQUAL: u8 = 0xF;
}

pub mod direction {
    pub const TO_REGISTER: u8 = 0;
    pub const TO_MEMORY: u8 = 1;
}

pub mod shift {
    pub const ARITHMETIC: u8 = 0;
    pub const LOGICAL: u8 = 1;
    pub const ROTATE_EXTEND: u8 = 2;
    pub const ROTATE: u8 = 3;
    pub const RIGHT: u8 = 0;
    pub const LEFT: u8 = 1;
    pub const MASK_REGISTER_COUNT: u32 = 0x3F;
}

pub mod bit_operation {
    pub const TEST: u8 = 0;
    pub const CHANGE: u8 = 1;
    pub const CLEAR: u8 = 2;
    pub const SET: u8 = 3;
}

pub mod field {
    pub const MASK_EA_MODE: u16 = 0x7;
    pub const BIT_DIRECTION_SHIFT: u16 = 8;
    pub const MASK_DIRECTION: u16 = 1;
    pub const BIT_BIT_OPERATION_SHIFT: u16 = 6;
    pub const MASK_BIT_OPERATION: u16 = 3;
    pub const BIT_SHIFT_MEMORY_TYPE_SHIFT: u16 = 9;
    pub const BIT_SHIFT_REGISTER_TYPE_SHIFT: u16 = 3;
    pub const MASK_SHIFT_TYPE: u16 = 3;
    pub const BIT_SHIFT_REGISTER_COUNT_SHIFT: u16 = 5;
    pub const MASK_SHIFT_REGISTER_COUNT: u16 = 1;
    pub const BIT_SHIFT_MEMORY_INVALID_SHIFT: u16 = 11;
    pub const MASK_SHIFT_MEMORY_INVALID: u16 = 1;
    pub const QUICK_IMMEDIATE_ZERO_VALUE: u8 = 8;
    pub const MASK_TRAP_NUMBER: u16 = 0x000F;

    pub const MASK_REGISTER: u16 = 0x7;
    pub const BIT_EA_MODE_SHIFT: u16 = 3;
    pub const BIT_MOVE_DEST_MODE_SHIFT: u16 = 6;
    pub const BIT_DEST_REGISTER_SHIFT: u16 = 9;
    pub const BIT_SIZE_SHIFT: u16 = 6;
    pub const MASK_SIZE: u16 = 0x3;
    pub const BIT_INDEX_ADDRESS_REGISTER: u16 = 0x8000;
    pub const BIT_INDEX_REGISTER_SHIFT: u16 = 12;
    pub const BIT_INDEX_LONG: u16 = 0x0800;
}
