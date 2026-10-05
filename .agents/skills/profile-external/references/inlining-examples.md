# Inlining decision examples

Use this matrix when inspecting measured hot paths. Attributes express compiler intent; do not infer a performance improvement from an annotation alone. The [performance rule](../../../rules/performance-and-readability.md) owns the required project policy.

## 2. Inlining Decision Matrix

| Attribute | When to Use | Examples in this Codebase |
| :--- | :--- | :--- |
| **`#[inline]`** | • Public getters, setters, accessors called across crates<br/>• Lightweight forwarding/delegation wrappers<br/>• Endian and byte packing helpers | `pub fn chip_ram(&self) -> ChipRamSize`<br/>`pub fn is_chip_ram_blocked(&self) -> bool`<br/>`pub fn step_cck(&mut self, cck: u64) { self.rtc.step_cck(cck); }` |
| **`#[inline(always)]`** | • Ultra-hot ALU condition code calculations ($X, N, Z, V, C$)<br/>• CPU register accessors used in instruction micro-steps | CCR flag evaluations for byte/word/long math<br/>`self.update_ccr_add(...)` |
| **No attribute** (Default) | • Functions with > 15–20 lines of control flow<br/>• Indirect table targets (e.g. `BankHandler.read_byte`, opcode tables) | `fn read_chip_ram(...)`<br/>`fn execute_move_w(...)` |
| **`#[inline(never)]`** | • Cold exception paths and traps<br/>• Panic / unreachable / diagnostic dumps | Vector 3 Address Error stack frame creation<br/>Illegal instruction reporter |
