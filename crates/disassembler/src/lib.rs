//! Built-in zero-dependency Motorola 68000 opcode disassembler and stream alignment engine

pub mod align;
pub mod alu;
pub mod branch;
pub mod data;
pub mod ea;
pub mod types;

pub use align::find_aligned_disassembly_start;
pub use ea::{
    bcc_condition_name, dbcc_condition_name, format_ea, format_immediate, format_movem_reg_list,
    scc_condition_name,
};
pub use types::Disassembly;

/// Disassembles one instruction starting at `pc` using a side-effect-free word reader function
pub fn disassemble(pc: u32, read_word: impl Fn(u32) -> u16) -> (Disassembly, u32) {
    let mut words = [0u16; 5];
    let mut offset = 0u32;

    let op = read_word(pc);
    words[0] = op;
    let mut word_count = 1;
    offset += 2;

    let mut next_word = || {
        let w = read_word(pc.wrapping_add(offset));
        if word_count < 5 {
            words[word_count] = w;
            word_count += 1;
        }
        offset += 2;
        w
    };

    // 1. Control flow, branches, jumps, traps, returns, DBcc, Scc
    let decoded = if let Some(res) = branch::try_disassemble_branch(pc, op, &mut next_word) {
        Some(res)
    // 2. Data movement, stack ops, unaries, quick arithmetic
    } else if let Some(res) = data::try_disassemble_data(op, &mut next_word) {
        Some(res)
    // 3. Arithmetic, logic, compare, multiply/divide, shifts & rotates
    } else if let Some(res) = alu::try_disassemble_alu(op, &mut next_word) {
        Some(res)
    } else {
        None
    };

    if let Some((mnem, ops)) = decoded {
        (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: mnem,
                operands: ops,
            },
            offset,
        )
    } else {
        // Default raw instruction word fallback
        (
            Disassembly {
                pc,
                words,
                word_count: 1,
                mnemonic: "DATA.W",
                operands: format!("${:04X}", op),
            },
            2,
        )
    }
}
