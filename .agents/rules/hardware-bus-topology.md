# Strict Hardware Bus Topology & Inter-Chip Signal Isolation Rule

This rule governs the physical electronic bus model, DMA address mastership, and signal propagation boundaries between custom chips, CPU, and memory across the emulator.

---

## 1. The Core Invariant: Zero Direct Inter-Chip Shortcuts ("No Signal Smuggling")

In the physical Amiga 500 motherboard architecture, custom chips do not share memory handles, invoke methods on peer chips, or maintain dynamic software callbacks:
- **Strict Prohibition of Direct Inter-Chip Calls:** Subsystems (`Agnus`, `Denise`, `Paula`, `CiaA`, `CiaB`, `Cpu`) must **never** hold direct pointers or invoke mutating methods directly on each other.
- **Strict Prohibition of Ad-Hoc Inter-Chip Backchannels:** Passing state, event notifications, or triggers between chips via private struct fields or synthetic backchannels is forbidden.
- **Physical Wire & Bus Routing:** All inter-chip interactions—including DMA channel triggers, interrupt lines (`_INTREQ`, IPL 1..6), reset strobes, raster beam synchronization, and clock ticks—must be routed exclusively through the top-level machine loop and `MemoryBus` modeling physical PCB copper traces.

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
