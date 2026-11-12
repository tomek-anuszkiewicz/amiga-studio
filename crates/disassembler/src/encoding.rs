//! M68000 encodings owned by the standalone disassembler.

pub mod opcode {
    pub const MASK_OPCODE_LINE: u16 = 0xF000;
    pub const MASK_OPCODE_HIGH: u16 = 0xFF00;
    pub const MASK_REGISTER_LINE: u16 = 0xF100;
    pub const MASK_REGISTER_OPCODE: u16 = 0xFFF8;
    pub const MASK_EA_OPCODE: u16 = 0xFFC0;
    pub const MASK_REGISTER_EA_OPCODE: u16 = 0xF1C0;
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

pub mod size {
    pub const BYTE: u16 = 0;
    pub const WORD: u16 = 1;
    pub const LONG: u16 = 2;
    pub const SPECIAL: u16 = 3;
}

pub mod move_size {
    pub const INVALID: u16 = 0;
    pub const BYTE: u16 = 1;
    pub const LONG: u16 = 2;
    pub const WORD: u16 = 3;
}

pub mod opmode {
    pub const EA_TO_BYTE: u8 = 0;
    pub const EA_TO_WORD: u8 = 1;
    pub const EA_TO_LONG: u8 = 2;
    pub const ADDRESS_WORD: u8 = 3;
    pub const BYTE_TO_EA: u8 = 4;
    pub const WORD_TO_EA: u8 = 5;
    pub const LONG_TO_EA: u8 = 6;
    pub const ADDRESS_LONG: u8 = 7;
}

pub mod field {
    pub const MASK_REGISTER: u16 = 7;
    pub const BIT_REGISTER_SHIFT: u16 = 9;
    pub const MASK_EA_MODE: u16 = 7;
    pub const BIT_EA_MODE_SHIFT: u16 = 3;
    pub const MASK_SIZE: u16 = 3;
    pub const BIT_SIZE_SHIFT: u16 = 6;
    pub const BIT_MOVE_DEST_MODE_SHIFT: u16 = 6;
    pub const BIT_MOVE_SIZE_SHIFT: u16 = 12;
    pub const BIT_MOVE_CLASS_SHIFT: u16 = 14;
    pub const MASK_MOVE_CLASS: u16 = 3;
    pub const MOVE_CLASS: u16 = 0;
    pub const BIT_OPMODE_SHIFT: u16 = 6;
    pub const MASK_OPMODE: u16 = 7;
    pub const BIT_INDEX_ADDRESS_REGISTER: u16 = 0x8000;
    pub const BIT_INDEX_REGISTER_SHIFT: u16 = 12;
    pub const BIT_INDEX_LONG: u16 = 0x0800;
    pub const BIT_OPCODE_LINE_SHIFT: u16 = 12;
    pub const MASK_OPCODE_LINE_VALUE: u16 = 0x0F;
    pub const BIT_OPCODE_HIGH_BYTE_SHIFT: u16 = 8;
    pub const MASK_TRAP_NUMBER: u16 = 0x0F;
    pub const QUICK_ZERO_VALUE: u8 = 8;
}

pub const MASK_PHYSICAL_ADDRESS: u32 = 0x00FF_FFFF;
pub const MASK_WORD_ADDRESS: u32 = 0x00FF_FFFE;
