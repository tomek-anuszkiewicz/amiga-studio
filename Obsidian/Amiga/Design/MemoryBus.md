# AGENT INSTRUCTION: Implement Amiga 500 MemoryBus Module

## 1. Scope & Location
- Place all module implementation code inside the `MemoryBus` directory[cite: 2].
- Implement 24-bit physical address decoding (`0x000000` - `0xFFFFFF`) and a 16-bit wide data bus supporting 8-bit byte and 16-bit big-endian word accesses[cite: 2].

---

## 2. Data Structures & Storage
- Represent all internal memory regions (Chip RAM, Slow RAM, Fast RAM, ROM) as **raw byte buffers (`Vec<u8>` or `[u8]`)**[cite: 2].
- Implement getters/setters that consume and return raw `&[u8]` / `&mut [u8]` buffers; do not use Base64 at this layer[cite: 2].
- Provide constructors/factory methods for the three standard hardware configurations[cite: 2]:
  - `0.5MB CHIP` (512 KB)[cite: 2]
  - `0.5MB CHIP + 0.5MB SLOW` (512 KB Chip + 512 KB at `$C00000`)[cite: 2]
  - `0.5MB CHIP + 0.5MB SLOW + 4MB FAST` (Fast RAM at `$200000-$5FFFFF`)[cite: 2]

---

## 3. Sub-Cycle Timing & 2-Phase Bus Arbitration

Model bus access according to the Motorola 68000 4-clock execution cycle (7.09 MHz PAL), divided into two Color Clock (CCK / 3.54 MHz) slots: **Phase 1 (S0–S3)** and **Phase 2 (S4–S7)**.

Maintain the following internal state on the bus:
- `read_latch: u16`: Transparent buffer holding sampled read data between phases.
- `chip_ram_blocked: bool`: Flag indicating whether Agnus/Blitter/DMA is currently occupying the Chip RAM bus.

### Return Type
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryBusResult {
    /// Phase 1 completed; proceed to Phase 2
    Phase1Ready,
    /// Transaction completed successfully (carries 16-bit word or zero-extended 8-bit byte on read, 0 on write)
    Ready(u16),
    /// Target bus is currently occupied (CPU must hold state and retry/insert wait states)
    Blocked,
}
```

### Read Transaction Algorithm (`read_phase1` and `read_phase2`)
1. **`read_phase1(addr: u32, is_byte: bool, high_byte: bool) -> MemoryBusResult`:**
   - Detect targeted address region.
   - If target is **Chip RAM** (or Slow RAM shared with Agnus) and `chip_ram_blocked == true`:
     - Return `MemoryBusResult::Blocked`. Do not fetch data; CPU must repeat Phase 1.
   - If target is **unblocked Chip RAM, Fast RAM, or ROM**:
     - Perform DRAM/ROM read cycle immediately.
     - Store fetched word/byte into `self.read_latch`.
     - Return `MemoryBusResult::Phase1Ready`.
2. **`read_phase2(addr: u32) -> MemoryBusResult`:**
   - Do **NOT** check `chip_ram_blocked`. The CPU reads exclusively from `self.read_latch`, isolated from the memory bus.
   - Return `MemoryBusResult::Ready(self.read_latch)`.

### Write Transaction Algorithm (`write_phase1` and `write_phase2`)
**Crucial Architectural Rule:** The write path is completely unbuffered (no posted-write FIFO).
1. **`write_phase1(addr: u32, data: u16) -> MemoryBusResult`:**
   - CPU presents address and data onto external pins.
   - Do **NOT** write to memory or check `chip_ram_blocked`.
   - Store incoming data in temporary register (e.g. `pending_write_data`).
   - Return `MemoryBusResult::Phase1Ready`.
2. **`write_phase2(addr: u32, data: u16, is_byte: bool, high_byte: bool) -> MemoryBusResult`:**
   - If target is **Chip RAM** and `chip_ram_blocked == true`:
     - Gary withholds `_DTACK`.
     - Return `MemoryBusResult::Blocked`. CPU must re-invoke Phase 2 without changing bus state.
   - If target is **unblocked Chip RAM** or **Fast RAM**:
     - Commit the byte or word directly into memory.
     - Return `MemoryBusResult::Ready(0)`.

---

## 4. Hardware Quirks & Address Decoding Rules

### Floating Bus / Unmapped Address Space
- Reads to unmapped regions return **`0xFF`** (8-bit) or **`0xFFFF`** (16-bit)[cite: 2].
- Writes to unmapped regions are silent no-ops[cite: 2].
- Do not trigger Bus Error exceptions (`_BERR` is not wired on stock A500)[cite: 2].

### Custom Chip Registers (`$DFF000-$DFFFFE`)
- Dedicated registers are 16-bit wide[cite: 2].
- **Byte Write:** Write the active byte to the addressed half of the register, while treating the inactive half as open bus lines (`0xFF`)[cite: 2].
- **Read:** Return register value; disconnected or write-only register bits must return `1`s (`0xFF`)[cite: 2].

### 8520 CIA Registers (`$BFE001` / `$BFD000`)
- Native 8-bit devices accessed via byte lanes (CIA-A at odd byte addresses, CIA-B at even byte addresses)[cite: 2].
- Direct byte reads and writes are native[cite: 2].
- **16-bit Word Read:** Read 8-bit register from addressed CIA on its active byte lane, and set the unmapped byte lane to **`0xFF`**[cite: 2].

### TAS (Test-And-Set) Read-Modify-Write Hardware Bug
- **Chip RAM (`$000000-$07FFFF`) and Slow RAM (`$C00000-$C7FFFF`):**
  - Agnus/Gary fails to latch the write phase of an unbroken RMW cycle[cite: 2].
  - Evaluate memory content, return data, and allow CPU condition codes ($N$, $Z$) to update[cite: 2].
  - **Drop the write phase** (do NOT set bit 7 in memory)[cite: 2].
- **Fast RAM (`$200000-$5FFFFF`):**
  - Read succeeds, and write succeeds (bit 7 is set to 1)[cite: 2].

### Low-Memory Boot Overlay
Implement explicit address mapping methods without using "OVL" or "overlay" in identifiers[cite: 2]:
- `map_kickstart_to_low_memory()`: Routes `$000000-$07FFFF` accesses to Kickstart ROM[cite: 2].
- `map_chip_ram_to_low_memory()`: Restores physical Chip RAM mapping at `$000000-$07FFFF`[cite: 2].

### DMA Arbitration Methods
Expose methods to simulate Agnus cycle stealing:
- `lock_chip_ram()`: Sets `chip_ram_blocked = true`[cite: 2].
- `unlock_chip_ram()`: Sets `chip_ram_blocked = false`[cite: 2].
- `is_chip_ram_blocked() -> bool`: Queries current arbitration status.


TODO:
Reset:
- zero memory, only cold reset