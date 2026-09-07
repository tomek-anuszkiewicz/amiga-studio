# Amiga 500 MemoryBus Architecture & Hardware Quirks

> [!NOTE]
> Global endianness rules, wrapping arithmetic, and WASM constraints are defined in [AGENTS.md](../../../AGENTS.md).

---

## 1. Scope & Physical Address Space

- **Module Location:** `memory_bus/`
- **Bus Width:** 24-bit physical address space (`$000000`–`$FFFFFF`, 16 MB) and a 16-bit wide data bus supporting 8-bit byte and 16-bit Big-Endian word accesses.

---

## 2. Complete 24-Bit Physical Memory Map

| Address Range | Size | Description | Hardware Master / Quirks |
| :--- | :---: | :--- | :--- |
| **`$000000 - $07FFFF`** | 512 KB | **Chip RAM** (Base) | Shared with Agnus/Denise/Paula. Contention causes CPU wait states. |
| **`$080000 - $0FFFFF`** | 512 KB | **Extended Chip RAM** | Present on ECS (1MB Agnus); unmapped open bus (`$FF`) on 512KB OCS. |
| **`$100000 - $1FFFFF`** | 1 MB | **Reserved Space** | Open bus floating lines (`$FF`). |
| **`$200000 - $9FFFFF`** | 8 MB | **Auto-Config Fast RAM** | Zorro II expansion space. Zero Agnus contention (CPU runs at full speed). |
| **`$A00000 - $BEFFFF`** | ~1.9 MB | **Reserved / Expansion** | Open bus floating lines. |
| **`$BFD000 - $BFDF00`** | 4 KB | **CIA-B Peripheral Registers** | 8-bit device mapped to **Even byte addresses** (`A0 = 0`). Odd bytes return `$FF`. |
| **`$BFE001 - $BFEF01`** | 4 KB | **CIA-A Peripheral Registers** | 8-bit device mapped to **Odd byte addresses** (`A0 = 1`). Even bytes return `$FF`. |
| **`$C00000 - $C7FFFF`** | 512 KB | **Slow RAM (Trapdoor)** | Pseudo-fast RAM. Access handled via Gary; shared bus contention applies. |
| **`$C80000 - $DBFFFF`** | ~1.2 MB | **Reserved Space** | Open bus floating lines (`$FF`). |
| **`$DC0000 - $DC003F`** | 64 B | **[Real-Time Clock (RTC)](RTC.md)** | OKI MSM6242B on A501 / A500+ / A2000. 16 4-bit registers on odd byte addresses (`A0 = 1`). Returns open bus `$FF` when no RTC installed (`RtcModel::None`). See [RTC.md](RTC.md). |
| **`$DC0040 - $DDFEFF`** | ~127 KB | **Reserved Space** | Open bus floating lines / mirror of custom registers. |
| **`$DFF000 - $DFFFFE`** | 512 B | **Custom Chip Registers** | 16-bit registers (Agnus, Denise, Paula). Mirrored across `$DFF000-$DFFFFF`. |
| **`$E00000 - $E7FFFF`** | 512 KB | **Extended ROM / Mirror** | CDTV extended ROM or mirror of 512KB Kickstart lower half. |
| **`$E80000 - $EFFFFF`** | 512 KB | **Auto-Config I/O Space** | Expansion board autoconfig registers (`$E80000`). |
| **`$F00000 - $F7FFFF`** | 512 KB | **Cartridge / Diagnostic ROM** | Action Replay / diagnostic expansion space. |
| **`$F80000 - $FFFFFF`** | 512 KB | **Kickstart ROM** | 256 KB Kickstart (mirrored twice) or 512 KB Kickstart high half. |

> [!IMPORTANT]
> **Low-Memory Boot Overlay (`_OVL`):**
> When the overlay is active (at cold or warm reset via CIA-A Port A bit 0), any access targeting `$000000-$07FFFF` is redirected directly to Kickstart ROM at `$F80000-$FFFFFF`. This guarantees that vector fetches ($SSP$ at `$000000`, $PC$ at `$000004`) execute directly from ROM.

### 2.1 256-Entry Direct Bank Dispatch Table (`addr >> 16`)

To eliminate branch mispredictions and cascaded conditional checks in hot memory access loops, the 16 MB physical address space is divided into **256 banks of 64 KB each** ($256 \times 64\text{ KB} = 16\text{ MB}$).

- **Direct Function Pointer Method Dispatch**: Mimicking the CPU's direct opcode table (`[OpcodeHandler; 65536]`), `bank_map` is a 256-entry array of `BankHandler` structs containing direct function pointers to read and write handler methods:
  ```rust
  pub type BankReadByteFn = fn(&MemoryBus, u32) -> u8;
  pub type BankWriteByteFn = fn(&mut MemoryBus, u32, u8);

  #[derive(Clone, Copy, PartialEq, Eq)]
  pub struct BankHandler {
      pub bank: MemoryBank,
      pub read_byte: BankReadByteFn,
      pub write_byte: BankWriteByteFn,
  }
  ```
- **Zero Runtime Branches**: Memory accesses execute directly through the table:
  ```rust
  (self.bank_map[(addr >> 16) as usize].read_byte)(self, addr);
  (self.bank_map[(addr >> 16) as usize].write_byte)(self, addr, val);
  ```
- **Zero Runtime Setup (`static`/`const`)**: Precalculated as compile-time `static` arrays (`BANK_MAP_BARE`, `BANK_MAP_STANDARD`, `BANK_MAP_EXPANDED`), eliminating all initialization loops or runtime reallocation overhead.
- **Direct Dispatch**:
  - `$00..=$07`: `CHIP_RAM_HANDLER`
  - `$20..=$5F`: `FAST_RAM_HANDLER` (4 MB, active in `ExpandedPowerUser`)
  - `$BF`: `CIA_HANDLER` (CIA-A & CIA-B)
  - `$C0..=$C7`: `SLOW_RAM_HANDLER` (512 KB A501 trapdoor RAM, active in `Standard1Mb` & `ExpandedPowerUser`)
  - `$DC`: `RTC_HANDLER` (OKI MSM6242B, active in `Standard1Mb` & `ExpandedPowerUser`)
  - `$DF`: `CUSTOM_CHIPS_HANDLER` (Agnus, Denise, Paula)
  - `$F8..=$FF`: `KICKSTART_ROM_HANDLER`
### 2.2 Unmapped Open Bus Physics ($FF / $FFFF) & Test Memory Override

- **Amiga Physical Open Bus:**
  On physical Amiga 500 hardware, unpopulated memory address space, disconnected expansions, unmapped chip areas, or CIA lane gaps float high due to internal/external pull-up resistors.
  - A byte read from an unmapped address must return **`$FF`** (`0xFF`).
  - A word read from an unmapped address must return **`$FFFF`** (`0xFFFF`).
  - Writes to unmapped space are silent no-ops and must never trigger host panics or out-of-bounds indexing.
- **Configurable `unmapped_byte` Field:**
  - Real emulation strictly defaults to `unmapped_byte = 0xFF` across all constructors (`MemoryBus::new()`, `from_config()`, `new_test()`).
  - Synthetic CPU test vectors (such as `SingleStepTests`) expect a flat RAM model where unpopulated memory addresses default to `$00`.
  - The default unmapped value is dynamically configurable via `bus.set_unmapped_byte(val)`. In `test_runner/src/runner.rs`, SingleStepTests explicitly set `bus.set_unmapped_byte(0x00)` so unpopulated test locations read 0, while real emulator execution and production testing always use `$FF`.

---

## 3. Sub-Cycle Timing & 2-Phase Bus Arbitration

The Motorola 68000 bus cycle spans 4 CPU clocks ($S_0$ through $S_7$), which maps to two Color Clock slots (CCK / 3.54 MHz): **Phase 1 / CCK1 (S0–S3)** and **Phase 2 / CCK2 (S4–S7)**.

While the CPU requires 2 Color Clocks to complete an instruction bus transaction, **Amiga Chip RAM can complete a physical access in just 1 Color Clock (280 ns)**. The hardware exploits this difference to interleave access 50/50 between CPU and DMA without slowing down the CPU:
- **Read Cycle:** On CCK1, data is read from physical Chip RAM into `read_latch`. On CCK2, the CPU reads safely from `read_latch` while **the physical Chip RAM bus is completely freed for custom chip DMA**.
- **Write Cycle:** On CCK1, the CPU prepares address and data internally without needing physical memory access (leaving CCK1 free for DMA). On CCK2, the CPU commits the unbuffered write directly to physical Chip RAM.

Maintain the following internal bus state:
- `read_latch: u16`: Transparent buffer holding sampled read data between phases.
- `chip_ram_blocked: bool`: Flag indicating whether Agnus / Blitter / DMA is currently occupying the Chip RAM bus.

### Types & Structures
```rust
use serde::{Deserialize, Serialize};

/// Color Clock (CCK) sub-cycle phase of the 4-clock M68000 bus cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CckPhase {
    /// Color Clock Phase 1 (CPU S0–S3): Address output, _AS strobe, contention arbitration
    Cck1,
    /// Color Clock Phase 2 (CPU S4–S7): Data latch/write commit, _DTACK acknowledgement
    Cck2,
}

/// Bus access transfer size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusAccessSize {
    Byte,
    Word,
}

/// Structured M68000 bus cycle representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusCycle {
    pub addr: u32,
    pub data: u16,
    pub size: BusAccessSize,
    pub fc: u8,
    pub is_read: bool,
    pub uds: bool,
    pub lds: bool,
}

/// Result of an M68000 bus transaction phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryBusResult {
    /// Phase 1 completed; proceed to Phase 2
    Phase1Ready,
    /// Transaction completed successfully (carries 16-bit word or zero-extended 8-bit byte on read, 0 on write)
    Ready(u16),
    /// Target bus is currently occupied (CPU must hold state and retry/insert wait states)
    Blocked,
}
```

### Structured 2-Phase Transaction API (`begin_cycle` and `end_cycle`)
1. **`begin_cycle(&mut self, cycle: &mut BusCycle) -> MemoryBusResult` (CCK1 / S0–S3):**
   - Address is masked to 24 bits (`addr & 0x00FF_FFFF`).
   - If **Read**:
     - Detect targeted region. If Chip RAM (or Slow RAM) and `chip_ram_blocked == true`, return `MemoryBusResult::Blocked` (Gary withholds `_DTACK`, CPU holds at CCK1).
     - Otherwise, perform read into `self.read_latch` and return `MemoryBusResult::Phase1Ready`.
   - If **Write**:
     - CPU places address and data onto bus lines (`pending_write_data = cycle.data`). Returns `MemoryBusResult::Phase1Ready`.
2. **`end_cycle(&mut self, cycle: &mut BusCycle) -> MemoryBusResult` (CCK2 / S4–S7):**
   - Address is masked to 24 bits (`addr & 0x00FF_FFFF`).
   - If **Read**:
     - Reads safely from `self.read_latch` without external contention (bus is free for DMA).
     - Extracts byte (based on `uds`/`lds`) or full word into `cycle.data`.
     - Returns `MemoryBusResult::Ready(cycle.data)`.
   - If **Write**:
     - If target is Chip RAM and `chip_ram_blocked == true`, return `MemoryBusResult::Blocked` (Gary withholds `_DTACK`, CPU stalls at CCK2).
     - Otherwise, commits byte or word to memory and returns `MemoryBusResult::Ready(0)`.

---

## 4. Hardware Quirks & Address Decoding Rules

### Floating Bus / Unmapped Address Space
- Reads to unmapped regions return **`0xFF`** (8-bit) or **`0xFFFF`** (16-bit).
- Writes to unmapped regions are silent no-ops.
- Do not trigger Bus Error exceptions (`_BERR` is not wired on stock A500).

### Custom Chip Registers & 512-Byte Mirroring (`$DFF000-$DFFFFE`)
- Dedicated registers are 16-bit wide (256 words, 512 bytes, `$DFF000-$DFF1FE`).
- **64 KB Block Mirroring:** Gary and custom chip address decoders evaluate address bits `A1..A8` (`(addr & 0x1FE) >> 1`), ignoring higher address bits `A9..A15`. As a result, the 512-byte register block repeats **128 times** across the entire 64 KB space (`$DFF000-$DFFFFF`).
- **Byte Write Merging:** Byte writes merge the written byte into the addressed half of the 16-bit register, preserving the unaddressed half:
  - Even address (`addr & 1 == 0`): Overwrites upper byte (`D15-D8`), preserving lower byte.
  - Odd address (`addr & 1 == 1`): Overwrites lower byte (`D7-D0`), preserving upper byte.
- **Read:** Return register value; disconnected or write-only register bits return `1`s (`0xFF`).

### 8520 CIA Registers (`$BFE001` / `$BFD000`)
- Native 8-bit devices accessed via byte lanes:
  - **CIA-A:** Odd byte addresses (`$BFE001`, `$BFE101`, ..., `$BFEF01`).
  - **CIA-B:** Even byte addresses (`$BFD000`, `$BFD100`, ..., `$BFDE00`).
- Direct byte reads and writes are native.
- **16-bit Word Read:** Read 8-bit register from addressed CIA on its active byte lane, and set the unmapped byte lane to **`0xFF`**.

### TAS (Test-And-Set) Read-Modify-Write Hardware Bug
- **Chip RAM (`$000000-$07FFFF`) and Slow RAM (`$C00000-$C7FFFF`):**
  - Agnus/Gary fails to latch the write phase of an unbroken RMW cycle.
  - Evaluate memory content, return data, and allow CPU condition codes ($N$, $Z$) to update.
  - **Drop the write phase** (do NOT set bit 7 in memory).
- **Fast RAM (`$200000-$5FFFFF`):**
  - Read succeeds, and write succeeds (bit 7 is set to 1).

### Low-Memory Boot Overlay Control via CIA-A Port A Bit 0
- At hardware reset (cold or warm), Gary initializes with low-memory overlay active (`low_memory_overlay = true`), routing `$000000-$07FFFF` accesses to Kickstart ROM.
- Software explicitly controls this mapping via **CIA-A Port A bit 0 (`_OVL`)** at address `$BFE001`:
  - **Bit 0 written as `0`:** Invokes `map_kickstart_to_low_memory()`, re-engaging Kickstart ROM over low Chip RAM.
  - **Bit 0 written as `1`:** Invokes `map_chip_ram_to_low_memory()`, exposing physical Chip RAM at `$000000-$07FFFF`.
- Methods use explicit routing semantics without "OVL" or "overlay" in their names (`map_kickstart_to_low_memory()` and `map_chip_ram_to_low_memory()`).

### Kickstart ROM Space ($F80000-$FFFFFF) & Mirroring Rules
- **Physical Hardware Behavior (Gary & Mask-ROM):**
  - The Amiga 500 Kickstart ROM (256 KB or 512 KB) is physically read-only (Mask-ROM/EPROM without a write-enable `_WE` line).
  - When the M68000 initiates a write bus cycle (`R/_W = LOW`) targeting Kickstart ROM space (`$F80000-$FFFFFF`) or low memory during boot overlay (`$000000-$07FFFF` while overlay is active):
    - The Gary custom chip decodes the address and asserts `_DTACK` to terminate the bus transaction cleanly.
    - Data placed on the data bus (`D0-D15`) is discarded (silent drop / no-op) by the un-writable ROM hardware.
    - No Bus Error exception (`_BERR`) is asserted, and execution proceeds uninterrupted.
- **256 KB vs 512 KB Mirroring:**
  - **256 KB Kickstart ROMs (Kickstart 1.2 / 1.3):** Address decoding masks the offset using `offset & (rom_len - 1)`. The 256 KB ROM image is physically mirrored twice across the 512 KB range: `$F80000-$FBFFFF` and `$FC0000-$FFFFFF`.
  - **512 KB Kickstart ROMs (Kickstart 2.04 / 3.1):** Fills the complete 512 KB range without aliasing.
  - **Unloaded / Missing ROM:** Returns floating bus `$FF`.
- **Emulator Implementation:**
  - `write_kickstart_rom`: Implemented as a direct no-op (`fn write_kickstart_rom(_bus: &mut MemoryBus, _addr: u32, _val: u8) {}`).
  - `write_chip_ram`: When `low_memory_overlay == true` and `addr < 0x080000`, writes are safely discarded without altering underlying Chip RAM or ROM.

### Slow RAM ($C00000-$C7FFFF) Gary / Agnus Bus Contention Quirk
- Although Slow RAM is physically located on the trapdoor expansion, Gary routes its bus control through Agnus arbitration lines.
- Consequently, whenever Agnus DMA blocks Chip RAM (`chip_ram_blocked == true`), accesses to Slow RAM are **also blocked and stall the CPU**.
- **Exception under Overlay:** When low-memory overlay is active (`low_memory_overlay == true`), accesses below `$080000` route to Kickstart ROM, which is non-contended and never stalls.

### DMA Arbitration Methods
Expose methods to simulate Agnus cycle stealing:
- `lock_chip_ram()`: Sets `chip_ram_blocked = true`.
- `unlock_chip_ram()`: Sets `chip_ram_blocked = false`.
- `is_chip_ram_blocked() -> bool`: Queries current arbitration status.

### Test Memory & Direct State Injection for Test Runners
To support headless unit testing, SingleStepTests, and debugger inspection without side effects:
- `load_test_ram(&mut self, entries: &[[u32; 2]])`: Injects initial `[address, byte]` vectors directly into physical memory arrays (bypassing bus wait states and latches).
- `read_byte_debug(&self, addr: u32) -> u8`: Side-effect-free byte read for debugger inspection and test result assertions.
- `read_word_debug(&self, addr: u32) -> u16`: Side-effect-free word read for disassemblers and test result assertions.

---

## 5. Memory Bus Reset Methods

```rust
impl MemoryBus {
    /// Cold / Hard Reset: Wipes all physical RAM to zero and re-engages Kickstart overlay
    pub fn reset_cold(&mut self) {
        self.chip_ram.fill(0x00);
        if let Some(slow_ram) = &mut self.slow_ram {
            slow_ram.fill(0x00);
        }
        if let Some(fast_ram) = &mut self.fast_ram {
            fast_ram.fill(0x00);
        }
        self.chip_ram_blocked = false;
        self.read_latch = 0xFFFF;
        self.map_kickstart_to_low_memory();
    }

    /// Warm Reset: Preserves RAM contents (allowing Kickstart resident tags to survive)
    pub fn reset_warm(&mut self) {
        self.chip_ram_blocked = false;
        self.read_latch = 0xFFFF;
        self.map_kickstart_to_low_memory();
    }
}
```