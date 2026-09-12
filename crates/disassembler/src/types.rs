//! Disassembly data structures and line formatting

/// Disassembled instruction representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disassembly {
    /// Program Counter address
    pub pc: u32,
    /// Instruction words (up to 5 words on MC68000)
    pub words: [u16; 5],
    pub word_count: usize,
    /// Instruction mnemonic (e.g. "MOVE.W", "ADD.L", "NOP")
    pub mnemonic: &'static str,
    /// Operands string (e.g. "D0, D1", "#$0042, (A0)")
    pub operands: String,
}

impl Disassembly {
    /// Formats the disassembly into standard debugger view string (e.g. "00FC0004: 4E71            NOP")
    pub fn format_line(&self) -> String {
        let mut hex = String::with_capacity(16);
        for i in 0..self.word_count {
            hex.push_str(&format!("{:04X} ", self.words[i]));
        }
        if self.operands.is_empty() {
            format!("{:08X}: {:<16} {}", self.pc, hex, self.mnemonic)
        } else {
            format!(
                "{:08X}: {:<16} {:<8} {}",
                self.pc, hex, self.mnemonic, self.operands
            )
        }
    }
}
