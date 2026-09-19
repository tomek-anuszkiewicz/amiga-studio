# Strict Hardware Bus Topology & Inter-Chip Signal Isolation Rule

This rule governs the physical electronic bus model, DMA address mastership, and signal propagation boundaries between custom chips, CPU, and memory across the emulator.

---

## 1. The Core Invariant: Zero Direct Inter-Chip Shortcuts ("No Signal Smuggling")

In the physical Amiga 500 motherboard architecture, custom chips do not share memory handles, invoke methods on peer chips, or maintain dynamic software callbacks:
- **Strict Prohibition of Direct Inter-Chip Calls:** Subsystems (`Agnus`, `Denise`, `Paula`, `CiaA`, `CiaB`, `Cpu`) must **never** hold direct pointers or invoke mutating methods directly on each other.
- **Strict Prohibition of Ad-Hoc Inter-Chip Backchannels:** Passing state, event notifications, or triggers between chips via private struct fields or synthetic backchannels is forbidden.
- **Physical Wire & Bus Routing:** All inter-chip interactions—including DMA channel triggers, interrupt lines (`_INTREQ`, IPL 1..6), reset strobes, raster beam synchronization, and clock ticks—must be routed exclusively through the top-level machine loop and `MemoryBus` modeling physical PCB copper traces.

### 1.1 The `MachineLoop` Motherboard Coordinator & Poll-Based Signal Routing
- **Motherboard PCB Simulation:** The top-level machine struct (`MachineLoop` / `A500Machine`) acts as the physical motherboard PCB simulator. All electronic copper traces, pin connections, and cross-chip signal paths run through it.
- **Post-Cycle State Querying via `poll_*` Methods:**
  - Individual custom chips and coprocessors execute their internal logic for the current Color Clock (`step_cck()` or `step_cck_ram()`).
  - Upon cycle completion, chips do not push events or mutate peer chips. Instead, `MachineLoop` queries the chip's updated output pins using explicit polling methods:
    - **Agnus:** `poll_blitter_irq()`, `poll_vblank_irq()`, `poll_copper_write()`, `poll_bpl_dma()`
    - **Paula / Floppy:** `poll_audio_restart()`, `poll_dskblk_irq()`
    - **CIAs:** `irq_pending()`, `ovl_transition()`
  - Based on the polled pin states, `MachineLoop` routes the signals to the appropriate target subsystem's input methods (e.g. `paula.set_interrupt_request()`, `denise.write_bpldat()`, `agnus.reload_audio_ptr()`, `physical_memory.map_chip_ram_to_low_memory()`).
- **Strict Information Flow Invariant:** Information flow between chips is strictly **Execute Cycle $\to$ Motherboard Polls Outputs $\to$ Motherboard Drives Target Inputs**. No chip ever reaches outside its own boundaries.

---

## 2. Agnus Bus Mastership & Exclusive DMA Address Generation

In the Amiga architecture, **Agnus is the sole master DMA address generator and bus arbiter for Chip RAM**:
- **Sole DMA Address Master:** Agnus owns all pointer registers for autonomous Chip RAM DMA channels:
  - Bitplanes: `BPL1PTH/L` through `BPL6PTH/L`
  - Sprites: `SPR0PTH/L` through `SPR7PTH/L`
  - Audio: `AUD0PTH/L` through `AUD3PTH/L`
  - Floppy Disk: `DSKPTH/L`
  - Copper: `COP1LCH/L`, `COP2LCH/L`
  - Blitter: `BLTAPTH/L`, `BLTBPTH/L`, `BLTCPTH/L`, `BLTDPTH/L`
- **Address & RGA Bus Driving:** On each allocated DMA time slot on the horizontal scanline, Agnus places the memory address onto the Chip RAM address bus and simultaneously drives the internal Register Address bus (`RGA8..1`).
- **No DMA Pointers in Peer Chips:** No other custom chip (`Denise`, `Paula`, `CIA`) contains DMA address generators or autonomous bus master logic for Chip RAM.

---

## 3. Specialized Custom Chips as Passive Bus Latchers (Zero Direct Memory Reads)

Specialized custom chips (`Denise`, `Paula`) never access `PhysicalMemory` directly:
- **Zero Direct Memory Reads:** `Denise` and `Paula` must **never** call `memory.read()`, hold slices or references to `ChipRam`, or initiate autonomous memory accesses.
- **Passive Data Latching via Bus Strobes:**
  1. During a scheduled DMA slot (e.g. Bitplane, Sprite, Audio, Disk), **Agnus** issues the Chip RAM read cycle and sets the target custom register offset on the internal `RGA` bus.
  2. Physical Chip RAM places the 16-bit word onto the shared data bus.
  3. The target chip passively **latches the data word off the bus** into its holding register (e.g., `BPLxDAT`, `SPRxDAT`, `AUDxDAT`, `DSKDAT`) upon matching its assigned `RGA` strobe.
- **Bidirectional Disk DMA:** For floppy disk DMA, Agnus drives the bus cycle and address pointers, transferring data between Chip RAM and Paula's MFM holding register (`DSKDAT`) over the shared bus.

---

## 4. Modeling Discrete Control & Status Lines

Inter-chip coordination lines represent discrete electrical pins and copper traces:
1. **DMA Control & Timing Lines:**
   - `DMAL` (DMA Line): Agnus asserts `DMAL` to notify Paula that the current cycle is dedicated to audio or disk DMA.
   - `DMACON`: Agnus masters DMA gating across all channels; peripheral DMA enables are synchronized from master `DMACON`.
2. **Interrupt & Reset Lines:**
   - `_BLITINT`: Blitter asserts completion line to Paula, setting bit 6 in `INTREQ`.
   - `_VSYNC` / `_HSYNC`: Agnus beam counters generate vertical and horizontal sync strobes to Paula (`INTREQ` bit 5) and CIA TOD counters.
   - `AUDxDSR`: Paula pulses DMA reload strobe to Agnus when an audio channel sample period terminates.
   - `IPL1..6`: Paula and CIA interrupt lines are arbitrated into CPU interrupt priority levels in the machine loop.
3. **Peripheral & Port Lines:**
   - Port pins (CIA-A fire buttons, CIA-B floppy step/motor lines) are latched and sampled through explicit pin polling interfaces modeling ribbon cables and connectors.

---

## 5. Architectural Review Checklist

When implementing, modifying, or reviewing custom chip interaction code:
- [ ] Are custom chips completely decoupled, with zero direct references or method calls between them?
- [ ] Does Agnus exclusively own and increment all DMA pointers (`BPLxPT`, `SPRxPT`, `AUDxPT`, `DSKPT`, `COPxLC`, `BLTxPT`)?
- [ ] Do `Denise` and `Paula` receive DMA data exclusively through register bus writes/latches, with zero direct reads from `PhysicalMemory`?
- [ ] Are all cross-chip signals (interrupts, strobes, sync pulses) routed through the machine coordinator according to physical propagation delays?

---

## 6. Authoritative Hardware Specifications & Delegation

When implementing or refactoring custom chips, motherboard bus routing, or peripheral interfaces, agents must strictly adhere to the authoritative architectural specifications in `Obsidian/Amiga/Design/`:
- **Custom Chip Architecture:** [`Agnus.md`](../../Obsidian/Amiga/Design/Agnus.md), [`Denise.md`](../../Obsidian/Amiga/Design/Denise.md), [`Paula.md`](../../Obsidian/Amiga/Design/Paula.md), [`CIA.md`](../../Obsidian/Amiga/Design/CIA.md), [`Floppy.md`](../../Obsidian/Amiga/Design/Floppy.md).
- **Motherboard & Bus Routing:** [`MemoryBus.md`](../../Obsidian/Amiga/Design/MemoryBus.md), [`Main loop A500.md`](../../Obsidian/Amiga/Design/Main%20loop%20A500.md), [`Custom Chip Register Ownership and Access Matrix.md`](../../Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md), [`Cross-Chip Signals and Action Dispatch Catalog.md`](../../Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md), [`SaveState.md`](../../Obsidian/Amiga/Design/SaveState.md).
- **Peripherals & Controller Ports:** [`Keyboard.md`](../../Obsidian/Amiga/Design/Keyboard.md), [`Mouse.md`](../../Obsidian/Amiga/Design/Mouse.md), [`Joystick.md`](../../Obsidian/Amiga/Design/Joystick.md), [`Game Ports.md`](../../Obsidian/Amiga/Design/Game%20Ports.md).
