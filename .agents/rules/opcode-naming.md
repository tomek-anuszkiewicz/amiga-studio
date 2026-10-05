# M68000 Opcode Handler Names

Applies to CPU dispatch descriptors and instruction handlers.

- Use `op_<mnemonic>_<size>_<source>_<destination>` in Motorola source-then-destination order. Mnemonics are lowercase; sizes are `b`, `w`, or `l` and are omitted for unsized instructions.
- Unary handlers omit the source; zero-operand handlers omit operands; control-transfer handlers may include their target mode. Do not append numeric opcode suffixes for variable register fields.
- Canonical operands are `dn`, `an`, fixed `d0`-`d7` / `a0`-`a7`, `ai`, `pi`, `pd`, `disp`, `idx`, `absw`, `absl`, `pcdisp`, `pcidx`, `imm`, `sr`, `ccr`, and `usp`.
- Read [CPU Motorola M68000](../../Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md) and [CPU Micro-Step State Machine](../../Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md) before changing decoding or execution phases; silicon verification is specified in [CPU SingleStepTests](../../Obsidian/Amiga/Design/CPU%20SingleStepTests.md).
- Use [add-m68k-instruction](../skills/add-m68k-instruction/SKILL.md) when implementing or refactoring opcodes. Addressing vocabulary, naming examples, and bitfield tables are in its [opcode reference](../skills/add-m68k-instruction/references/opcode-reference.md).
