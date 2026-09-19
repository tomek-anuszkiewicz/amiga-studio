---
title: "Amiga 500 MemoryBus Architecture & Bus Topology"
aliases: ["MemoryBus", "Gary", "Address Map", "Bus Arbitration", "Bus Topology"]
tags: ["amiga", "design", "physical_memory", "memory_bus", "chip_ram", "gary"]
category: "Design"
subsystem: "physical_memory"
status: "active"
created: 2026-08-31
updated: 2026-09-19
related: ["[Agnus.md](Agnus.md)", "[Main loop A500.md](Main%20loop%20A500.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[RTC.md](RTC.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/memory_bus"
  - "crates/physical_memory"
last_synced_commit: "859017971b0453661ce25cc2c6f4de6b7475f2df"
last_synced_date: "2026-09-19"
---
# Amiga 500 MemoryBus Architecture & Bus Topology

> [!NOTE]
> Global endianness rules, wrapping arithmetic, and WASM constraints are defined in [AGENTS.md](../../../AGENTS.md).
> Color Clock timing is defined in [Main loop A500.md](Main%20loop%20A500.md). DMA contention and cycle stealing are coordinated with [Agnus.md](Agnus.md). Machine stepping and reset cycles are driven by [Main loop A500.md](Main%20loop%20A500.md), and CPU bus transactions are executed in [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md).

---

## 1. Scope & Physical Address Space

- **Module Locations:**
  - [`crates/physical_memory/src/physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs): Pure 24-bit physical storage (`PhysicalMemory`), RAM/ROM buffers, open-bus defaults, and DMA wait-state contention.
  - [`crates/memory_bus/src/memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Zero-cost motherboard address router (`MemoryBus<'a>`), decoding the 24-bit physical address space and routing transactions live to `PhysicalMemory`, Custom Chips, CIAs, and the RTC.
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
| **`$DFF000 - $DFFFFE`** | 512 B | **[Custom Chip Registers](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)** | 16-bit registers (Agnus, Denise, Paula). Mirrored across `$DFF000-$DFFFFF`. See [Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md). |
| **`$E00000 - $E7FFFF`** | 512 KB | **Extended ROM / Mirror** | CDTV extended ROM or mirror of 512KB Kickstart lower half. |
| **`$E80000 - $EFFFFF`** | 512 KB | **Auto-Config I/O Space** | Expansion board autoconfig registers (`$E80000`). |
| **`$F00000 - $F7FFFF`** | 512 KB | **Cartridge / Diagnostic ROM** | Action Replay / diagnostic expansion space. |
| **`$F80000 - $FFFFFF`** | 512 KB | **Kickstart ROM** | 256 KB Kickstart (mirrored twice) or 512 KB Kickstart high half. |

> [!IMPORTANT]
> **Low-Memory Boot Overlay (`_OVL`):**
> When the overlay is active (at cold or warm reset via CIA-A Port A bit 0), any access targeting `$000000-$07FFFF` is redirected directly to Kickstart ROM at `$F80000-$FFFFFF`. This guarantees that vector fetches ($SSP$ at `$000000`, $PC$ at `$000004`) execute directly from ROM.

### 2.1 256-Entry Direct Bank Dispatch Table (`addr >> 16`)

To eliminate branch mispredictions and cascaded conditional checks in hot memory access loops, the 16 MB physical address space is divided into **256 banks of 64 KB each** ($256 \times 64\text{ KB} = 16\text{ MB}$).

- **Direct Function Pointer Method Dispatch**: Mimicking the CPU's direct opcode table (`[OpcodeHandler; 65536]`), `bank_map` is a 256-entry array of `BankHandler` structs containing direct function pointers (`BankReadByteFn`, `BankWriteByteFn`, `BankReadWordFn`, `BankWriteWordFn`) returning `BusResult<T>` directly. Implementation resides in [`crates/physical_memory/src/map.rs`](../../../crates/physical_memory/src/map.rs).
- **Native 16-Bit Word Accesses**: In accordance with the 68000's physical 16-bit wide data bus, word transfers (instruction fetches, stack frames, 16-bit operands) execute directly via `read_word` and `write_word` function pointers, reading or writing aligned 16-bit words directly without decomposing into two separate 8-bit indirect function calls.
- **Zero Outer Contention Branches (Solution B)**: Memory accesses execute directly through table indexing `(self.bank_map[(addr >> 16) as usize].read_byte)(self, addr)` or `read_word`. Contention is an intrinsic physical property evaluated directly by individual bank handlers (Chip RAM and Slow RAM check `bus.chip_ram_blocked`; Fast RAM, ROM, and Open Bus never branch on contention). Outer bus methods perform zero contention checks.
- **Zero Runtime Setup (`static`/`const`)**: Precalculated as compile-time `static` arrays (`BANK_MAP_BARE`, `BANK_MAP_STANDARD`, `BANK_MAP_EXPANDED`), eliminating all initialization loops or runtime reallocation overhead. Topology presets reside in [`crates/physical_memory/src/presets.rs`](../../../crates/physical_memory/src/presets.rs).
- **Direct Dispatch in `PhysicalMemory`**:
  - `$00..=$07`: `CHIP_RAM_HANDLER`
  - `$20..=$5F`: `FAST_RAM_HANDLER` (4 MB, active in `ExpandedPowerUser`)
  - `$C0..=$C7`: `SLOW_RAM_HANDLER` (512 KB A501 trapdoor RAM, active in `Standard1Mb` & `ExpandedPowerUser`)
  - `$F8..=$FF`: `KICKSTART_ROM_HANDLER`
  - `$BF`, `$DC`, `$DF`, and unmapped ranges: `OPEN_BUS_HANDLER` (floating high `$FF` / `$FFFF`, silent writes).
- **Motherboard Routing in `MemoryBus` ([`crates/memory_bus/src/memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs)):**
  - `$DF`: Custom Chip Registers (`$DFF000..$DFFFFE`) routed directly to live Agnus, Denise, and Paula registers (see [Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) and [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)).
  - `$BF`: CIA Peripheral Registers (`$BFD000..$BFEF01`) routed directly to CIA-A and CIA-B.
  - `$DC`: Real-Time Clock (`$DC0000..$DC003F`) routed to OKI MSM6242B.
  - All other banks: Delegated directly to `PhysicalMemory`.
### 2.2 Unmapped Open Bus Physics ($FF / $FFFF) & Decoupled Test Architecture

- **Amiga Physical Open Bus:**
  On physical Amiga 500 hardware, unpopulated memory address space, disconnected expansions, unmapped chip areas, or CIA lane gaps float high due to internal/external pull-up resistors.
  - A byte read from an unmapped address must return **`$FF`** (`0xFF`).
  - A word read from an unmapped address must return **`$FFFF`** (`0xFFFF`).
  - Writes to unmapped space are silent no-ops and must never trigger host panics or out-of-bounds indexing.
- **Decoupled Architecture (`TestMemoryBus` vs `MemoryBus`):**
  - Real emulation strictly uses `MemoryBus`, with unmapped open bus space defaulting to `$FF`. `MemoryBus` is completely free of test-harness branching or logging in its read/write bank dispatch.
  - Synthetic CPU test vectors (`SingleStepTests`, Cartesian DMA contention sweeps) execute against `TestMemoryBus`, which encapsulates test RAM, configurable unmapped byte defaults (`0x00`), and automatic bus cycle transaction recording.
  - **Dual Storage Engine Architecture**:
    - `TestMemoryStorage::Sparse`: Powered by `std::collections::HashMap`, used by default in `TestMemoryBus::new()` for arbitrary unmapped defaults (`0xFF` open bus simulation).
    - `TestMemoryStorage::Flat`: Powered by a pre-allocated 16 MB buffer (`Box<[u8]>`) and a dirty address tracking list (`Vec<u32>`), created via `TestMemoryBus::new_flat()`. Provides $O(1)$ array accesses and $O(K)$ resets (`bus.clear()`) between test cases, eliminating all dynamic heap allocations in inner test execution loops.
  - Both buses implement the unified `AddressBus` trait ([`crates/physical_memory/src/address_bus.rs`](../../../crates/physical_memory/src/address_bus.rs)).

---

## 3. Sub-Cycle Timing & 2-Phase Bus Arbitration

The Motorola 68000 bus cycle spans 4 CPU clocks ($S_0$ through $S_7$), which maps to two Color Clock slots (CCK / 3.54 MHz): **Phase 1 / CCK1 (S0–S3)** and **Phase 2 / CCK2 (S4–S7)**.

While the CPU requires 2 Color Clocks to complete an instruction bus transaction, **Amiga Chip RAM can complete a physical access in just 1 Color Clock (280 ns)**. The hardware exploits this difference to interleave access 50/50 between CPU and custom chip DMA:
- **Read Cycle:** At CCK1, the CPU asserts address and strobes. Gary arbitrates access against Agnus DMA. At CCK2 ($S_6$), data is driven onto $D_0–D_{15}$ and sampled directly into the CPU's internal register (`source`, `destination`, `prefetch[0]`, or `irc`). No artificial intermediate bus latch is needed.
- **Write Cycle:** At CCK1, the CPU drives address and data onto pins (`BusCycle`). At CCK2, Gary asserts $\overline{\text{DTACK}}$ (or withholds it if Agnus DMA is active), and the write commits directly to physical Chip RAM.

Maintain the following internal bus state:
- `chip_ram_blocked: bool`: Flag indicating whether Agnus / Blitter / DMA is currently occupying the Chip RAM bus. Evaluated cycle-by-cycle by the Agnus Master DMA Arbiter (8-tier priority schedule: Refresh, Disk, Audio, Bitplane, Sprite, Copper, Blitter, CPU) and forwarded to `PhysicalMemory` by the central machine loop. Fast RAM (`$200000-$9FFFFF`) bypasses this check completely and executes with zero wait states.

### Types & Arbitration Primitives

The bus timing and transfer types reside in [`crates/physical_memory/src/address_bus.rs`](../../../crates/physical_memory/src/address_bus.rs):
- **`BusAccessSize` (`Byte`, `Word`)**: Test-runner bus transfer operand widths defined in [`crates/test_runner/src/transactions.rs`](../../../crates/test_runner/src/transactions.rs).
- **Function Code Lines (`m68000::function_code::*`)**: FC0–FC2 processor output pins defined in [`crates/cpu/src/state.rs`](../../../crates/cpu/src/state.rs) (`USER_DATA = 1`, `USER_PROGRAM = 2`, `SUPERVISOR_DATA = 5`, `SUPERVISOR_PROGRAM = 6`, `CPU_SPACE = 7`).

### Direct Passive Bus API & Contention Arbitration
The `MemoryBus` acts as a passive hardware backplane. Subsystem clients (CPU micro-engine, Copper, Blitter) execute single-cycle or multi-phase bus transactions directly against memory. Contention arbitration is encapsulated within the bus access methods, returning a dedicated `BusResult<T>`:

1. **`BusResult<T>` Return Semantics (Defined in [`crates/physical_memory/src/address_bus.rs`](../../../crates/physical_memory/src/address_bus.rs)):**
   - **`BusResult::Ready(T)`**: Bus access completed successfully with requested data (or `()` for write transfers).
   - **`BusResult::WaitState`**: Bus access stalled due to Agnus DMA cycle stealing / Chip RAM contention.
2. **Direct Memory Access Methods:**
   - `read_byte(&self, addr: u32) -> BusResult<u8>`
   - `read_word(&self, addr: u32) -> BusResult<u16>`
   - `write_byte(&mut self, addr: u32, val: u8) -> BusResult<()>`
   - `write_word(&mut self, addr: u32, val: u16) -> BusResult<()>`
   - `read(&self, addr: u32, size: BusAccessSize) -> BusResult<u16>`
   - `write(&mut self, addr: u32, val: u16, size: BusAccessSize) -> BusResult<()>`
   - All accesses verify whether the target address resides in Chip RAM (or contention-affected Slow RAM) and `chip_ram_blocked == true`. If blocked, the method returns `BusResult::WaitState` immediately without modifying state or reading corrupted/stale bus data.
   - All addresses are automatically masked to 24 bits (`addr & 0x00FF_FFFF`) and dispatched via the 256-entry bank dispatch table.
3. **2-Phase CCK Execution Flow (CPU Driven):**
    - **Read Transaction (`step_read_word_at` / `step_read_prog_word_at` / `step_read_byte_at`):**
      - **CCK1 ($S_0–S_3$):** CPU issues `bus.read_word(addr)` (or `read_byte`). If `BusResult::WaitState`, CPU stalls without advancing `phase`. When `BusResult::Ready(data)` is returned, data latches into `state.micro.source` and advances `phase` to `Cck2`.
      - **CCK2 ($S_4–S_7$):** CPU does nothing on the physical bus (already released for Agnus DMA). Records bus transaction and resets `phase` to `Cck1`, completing the bus cycle and allowing the micro-step to advance.
    - **Write Transaction (`step_write_word_at` / `step_write_byte_at`):**
      - **CCK1 ($S_0–S_3$):** CPU drives address and data internally, advancing `phase` to `Cck2` without touching the physical bus.
      - **CCK2 ($S_4–S_7$):** CPU issues `bus.write_word(addr, val)` (or `write_byte`). If `BusResult::WaitState` (Agnus DMA active, Gary withholds $\overline{\text{DTACK}}$), CPU stalls at CCK2 holding write pins asserted. When `BusResult::Ready(())` is returned, data has committed to memory; CPU records transaction and `phase` resets to `Cck1`.

---

## 4. Address Decoding Architecture & Memory Topology

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

### 8520 CIA Registers & Partial Address Decoding ($BFE001 / $BFD000)
- **Native 8-Bit Devices & Byte Lane Mapping:**
  - **CIA-A:** Odd byte addresses (`$BFE001`, `$BFE101`, ..., `$BFEF01`) mapped via `_LDS`.
  - **CIA-B:** Even byte addresses (`$BFD000`, `$BFD100`, ..., `$BFDE00`) mapped via `_UDS`.
- **Physical Motherboard Wiring & Address Line Connections:**
  - The MOS 8520 CIA has only 4 register address select pins (`RS0`, `RS1`, `RS2`, `RS3`) to address its 16 internal 8-bit registers.
  - On the Amiga 500 motherboard schematics, `RS0..RS3` connect directly to M68000 address lines **`A8, A9, A10, A11`**.
  - Address lines **`A1..A7` are completely unconnected** to the CIAs.
  - Gary performs coarse address decoding and generates the active-low Chip Select (`_CS`):
    - **CIA-A (`_CS` asserted when `A12 = 0`):** Enabled on odd byte addresses.
    - **CIA-B (`_CS` asserted when `A13 = 0`):** Enabled on even byte addresses.
- **Hardware Register Aliasing:**
  - Because `A1..A7` (and `A14..A15`) are ignored by the hardware, each internal CIA register repeats every 2 bytes across a 256-byte boundary, and the entire 16-register block mirrors continuously across the `$BF0000-$BFFFFF` range.
- **Custom Chip Address Decoding Comparison:**
  - Custom chips (Agnus, Denise, Paula) connect only to address lines **`A1..A8`** (selecting the 256 words / 512 bytes from `$000` to `$1FE`).
  - Gary asserts `_CUSTOM` when `A23..A16 = $DF`. Address lines **`A9..A15` are ignored**, producing the 128-fold mirror of the 512-byte register block across `$DFF000-$DFFFFE`.
- **16-bit Word Read:** Read 8-bit register from addressed CIA on its active byte lane, and set the unmapped byte lane to **`0xFF`**.

### TAS (Test-And-Set) Read-Modify-Write Silicon Erratum
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
- **CPU vs Custom Chipset Asymmetry (The Overlay Invariant):**
  - Gary's boot overlay routing applies strictly to the **M68000 CPU** bus interface: Gary intercepts CPU bus cycles targeting `$000000-$07FFFF` while `_OVL = 0` and routes them to Kickstart ROM space (`$F80000-$FFFFFF`). This allows the CPU to fetch the initial supervisor stack pointer ($SSP$ at `$000000`) and program counter ($PC$ at `$000004`) from ROM on reset.
  - **Custom Chipset Invariance**: Agnus features its own dedicated DRAM address bus (`DRA0..DRA8`) directly wired to the Chip RAM chips. **Agnus DMA memory cycles do not pass through Gary's `_OVL` multiplexer**.
  - **The Ground Truth:** Even while the boot overlay is active (`_OVL = 0`), any DMA access initiated by the custom chipset (Copper, Blitter, Bitplanes, Sprites, Audio, Disk) to address `$000000` **always accesses physical Chip RAM, never Kickstart ROM**. Only the CPU experiences the Kickstart overlay.
- **Dynamic 64 KB Bank Swapping:**
  - Toggling overlay dynamically swaps banks `0x00..=0x07` in `self.bank_map`:
    - **Overlay Active (`_OVL = 0`):** Populates `bank_map[0x00..=0x07]` directly with `KICKSTART_ROM_HANDLER` (`MemoryBank::KickstartRom`). Because Kickstart ROM address decoding masks with `rom_len - 1`, address lines `A18..A0` match whether accessed at `$000000` or `$F80000`, requiring zero address translation or separate overlay handlers.
    - **Overlay Inactive (`_OVL = 1`):** Populates `bank_map[0x00..=0x07]` with `CHIP_RAM_HANDLER` (`MemoryBank::ChipRam`).
  - This completely eliminates all runtime `if low_memory_overlay` branch evaluations from `read_chip_ram`, `write_chip_ram`, and `is_chip_ram_target`.

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
  - `write_kickstart_rom`: Implemented as a direct no-op (`fn write_kickstart_rom(_bus: &mut MemoryBus, _addr: u32, _val: u8) {}`). Discards writes to ROM at both `$F80000` and `$000000` (during boot overlay) without altering underlying Chip RAM or ROM.

### Slow RAM ($C00000-$C7FFFF) Gary Contention & OCS Agnus Invisibility
- **Gary Address Decoding:** The 512 KB expansion memory (e.g. trapdoor A501) at `$C00000-$C7FFFF` is decoded by **Gary** (asserting `_RAMEN` / `_EXRAM`).
- **Shared Bus Contention:** Slow RAM physically resides on the shared, multiplexed Chip RAM bus. When the 68000 accesses `$C00000`, Gary coordinates with Agnus and **withholds `_DTACK` whenever Agnus DMA is active**. Thus, CPU accesses to Slow RAM suffer the **exact same wait-state penalties as Chip RAM**.
- **OCS Agnus Invisibility:** The OCS Fat Agnus (MOS 8370/8371) contains only **19 DRAM address lines (`DRA0..DRA8` multiplexed = $2^{19} = 512\,\text{KB}$)**. Agnus physically cannot generate addresses outside `$000000-$07FFFF`. Therefore, custom chip DMA (Copper, Blitter, Bitplanes, Audio, Disk) **cannot see or access Slow RAM**.
- **The "Slow RAM" Trade-Off:** It has the speed disadvantages of Chip RAM (bus contention stalls), but none of the privileges (no chipset DMA visibility). Only the 68000 CPU can use it for program code and variables. *(On ECS Agnus 8372A with A500 Rev 6A motherboard jumper JP2 reconfigured, this physical RAM is remapped to `$080000-$0FFFFF`, promoting it to true 1 MB Chip RAM).*
- **Intrinsic Bank Contention:** In Solution B, `SLOW_RAM_HANDLER` and `CHIP_RAM_HANDLER` check `bus.chip_ram_blocked` and return `BusResult::WaitState` directly within their handler functions, while `FAST_RAM_HANDLER` and `KICKSTART_ROM_HANDLER` never inspect contention. Bus arbitration queries `is_chip_ram_target(addr)` in $O(1)$ by matching the bank classification without runtime range checks.

### Custom Chip DMA Bus Signaling (DMAL & RGA Bus Architecture)
- **Agnus as Exclusive DMA Address Generator:** Paula, Denise, and CIA contain no autonomous DMA bus master or address generation circuits. Agnus acts as the exclusive DMA address generator and bus arbiter for the entire system, owning all pointers (`BPLxPT`, `SPRxPT`, `AUDxPT`, `DSKPT`, `COPxLC`, `BLTxPT`).
- **Physical Signalling & Bus Lines:**
  - **`DRA19..0` (Chip RAM Address Bus):** Agnus places the memory address onto the Chip RAM multiplexed address lines during the assigned DMA slot.
  - **`RGA(8:1)` (Register Address Bus — Denise & Paula pins):** Agnus drives the target custom register offset on the internal Register Address bus:
    - When `RGA` corresponds to `BPL1DAT`..`BPL6DAT` (`$110`..`$11A`), **Denise** latches the 16-bit bitplane word from the shared data bus (`D15..D0`).
    - When `RGA` corresponds to `SPR0DAT`/`SPR0POS`..`SPR7DAT`/`SPR7CTL` (`$140`..`$17E`), **Denise** latches the 16-bit sprite data word from the shared data bus.
    - When `RGA` corresponds to `AUD0DAT`..`AUD3DAT` (`$0AA`, `$0BA`, `$0CA`, `$0DA`), **Paula** latches the 16-bit audio sample word from the shared data bus into the respective audio channel holding latch.
    - When `RGA` corresponds to `DSKDAT` (`$026`), **Paula** transfers a 16-bit word between the floppy MFM serializer/deserializer and the data bus.
  - **`DMAL` (DMA Line — Paula pin 12):** Agnus asserts `DMAL` to notify Paula that the current bus cycle is dedicated to a Paula DMA transfer.
- **Strict Invariant (Zero Direct Memory Reads in Specialized Chips):** Neither Denise nor Paula holds references to `PhysicalMemory` or calls `memory.read()`. They are strictly passive bus latchers triggered by Agnus-driven memory and RGA bus cycles.
- **Prohibition of Direct Inter-Chip Shortcuts:** Direct method calls, shared state, or synthetic backchannels between custom chips are forbidden; all inter-chip coordination models physical bus lines per [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md).

### DMA Arbitration Fields
Expose raw field to simulate Agnus cycle stealing directly with zero method overhead:
- `chip_ram_blocked: bool`: Direct public flag asserted by Agnus DMA to block CPU Chip RAM accesses (`BusResult::WaitState`).

### Test Memory & Direct State Injection for Test Runners
To support headless unit testing, SingleStepTests, and debugger inspection without side effects:
- `load_test_ram(&mut self, entries: &[\[u32; 2\]])`: Injects initial `[address, byte]` vectors directly into physical memory arrays (bypassing bus wait states and latches).
- `read_byte_debug(&self, addr: u32) -> u8`: Side-effect-free byte read for debugger inspection and test result assertions.
- `read_word_debug(&self, addr: u32) -> u16`: Side-effect-free word read for disassemblers and test result assertions.

---

## 5. Memory Bus Reset Semantics

The memory bus reset behavior is implemented in [`crates/physical_memory/src/physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs):

- **Cold / Hard Reset (`reset_cold`)**:
  - Wipes all physical RAM (Chip RAM, Slow RAM, Fast RAM) to zero.
  - Clears bus contention locks (`chip_ram_blocked = false`).
  - Re-engages the low-memory boot overlay (`_OVL`), mapping `$000000-$07FFFF` directly to Kickstart ROM.
- **Warm Reset (`reset_warm`)**:
  - Preserves all RAM contents intact, allowing Kickstart resident modules and Exec ColdCapture/CoolCapture vectors to survive reboot.
  - Clears bus contention locks (`chip_ram_blocked = false`).
  - Re-engages the low-memory boot overlay (`_OVL`).

---

## 6. Reference Documentation & Upstream Ground Truth

- [Amiga Hardware Reference Manual: Appendix D (System Memory Map)](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Standard memory map allocations, chip register blocks, CIA odd/even byte mirrors, and expansion ranges.
- [68000 User's Manual: Section 5 (16-Bit Bus Operations)](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md): Bus cycle state transitions ($S_0$ through $S_7$), `/AS`, `/UDS`, `/LDS`, and `/DTACK` handshake protocols.
- [A500/A2000 Technical Reference Manual: System Block Diagrams](../Reference/A500%20A2000%20Technical%20Reference%20Manual/02%20-%20Section%202%20System%20Block%20Diagrams.md): Gary custom gate array architecture, address decoding, and system bus buffers.
- [PhysicalMemory Subsystem Implementation Source](../../../crates/physical_memory/src/physical_memory.rs): Living Rust implementation of 24-bit physical addressing, 256-entry bank table dispatch, open bus float pull-up, and test RAM injection.