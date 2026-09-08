# Amiga 500 Master CycleCounter & Hardware Clock Hierarchy

> [!NOTE]
> System execution constraints, zero-allocation hot path rules, and 2-phase Color Clock guidelines are defined in [AGENTS.md](../../../AGENTS.md).
> Physical bus arbitration, the transparent read latch buffer, and unbuffered write cycles are specified in [MemoryBus.md](MemoryBus.md).
> Master beam position counters (`VHPOSR`, `VPOSR`) are implemented in Agnus ([Agnus.md](Agnus.md)).
> CIA E-Clock division is implemented in [CIA.md](CIA.md).

---

## 1. System Clock Generation & Master Hierarchy

The Amiga 500 derives all system clock frequencies from a single master crystal oscillator. All processing units operate in synchronous harmonic lock:

```mermaid
flowchart TD
    XTAL["Master Crystal Oscillator\nPAL: 28.37516 MHz | NTSC: 28.63636 MHz"] --> DIV4["Divide by 4"]
    XTAL --> DIV8["Divide by 8"]
    
    DIV4 --> CPU_CLK["CPU Clock (7.09379 MHz PAL / 7.15909 MHz NTSC)\n1 Cycle ≈ 140.97 ns (PAL) / 139.68 ns (NTSC)"]
    DIV8 --> CCK["Color Clock / CCK (3.546895 MHz PAL / 3.579545 MHz NTSC)\n1 CCK ≈ 281.94 ns (PAL) / 279.37 ns (NTSC)"]
    
    CPU_CLK --> CPU["Motorola 68000 CPU\n(4 Clocks per Bus Cycle = 2 CCKs)"]
    CCK --> AGNUS["Agnus (Master Beam Counter / Copper / Blitter DMA)"]
    CCK --> DENISE["Denise (Pixel Serializer / Sprites / Palette)"]
    CCK --> PAULA["Paula (Audio Periods / Floppy MFM / UART)"]
    
    CPU_CLK --> DIV10["Divide by 10 (E-Clock Divider in CIAs)\n6 Clocks Low / 4 Clocks High"]
    DIV10 --> ECLK["Motorola E-Clock (709.379 kHz PAL / 715.909 kHz NTSC)\n1 E-Clock = 5 CCKs ≈ 1.4097 µs (PAL)"]
    ECLK --> CIAA["CIA-A (Timers A/B, TOD, Keyboard SDR)"]
    ECLK --> CIAB["CIA-B (Timers A/B, TOD, Floppy Control)"]
```

### 1.1 Master Frequency Standards

| Standard | Master Oscillator ($f_{\text{master}}$) | CPU Clock ($f_{\text{cpu}} = \frac{f_{\text{master}}}{4}$) | Color Clock ($f_{\text{cck}} = \frac{f_{\text{master}}}{8}$) | E-Clock ($f_{\text{e}} = \frac{f_{\text{cpu}}}{10}$) | CCK Period ($T_{\text{cck}}$) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **PAL** | **$28.375160\ \text{MHz}$** | **$7.093790\ \text{MHz}$** | **$3.546895\ \text{MHz}$** | **$709.379\ \text{kHz}$** | $\approx 281.9368\ \text{ns}$ |
| **NTSC** | **$28.636360\ \text{MHz}$** | **$7.159090\ \text{MHz}$** | **$3.579545\ \text{MHz}$** | **$715.909\ \text{kHz}$** | $\approx 279.3651\ \text{ns}$ |

---

## 2. 2-Phase Bus Timing & Transparent Memory Interleaving

A critical architectural triumph of the Amiga is its ability to share Chip RAM between the Motorola 68000 CPU and custom chip DMA channels:

```
CPU Clock:  | S0 | S1 | S2 | S3 | S4 | S5 | S6 | S7 |
            |-------------------|-------------------|
CCK Slot:   |       CCK1        |       CCK2        |
READ:       | Address & Strobes | Sample Data (S6)  |
            | (Bus Arbitrated)  | (CPU Samples Bus) |
            |-------------------|-------------------|
WRITE:      | Address & Control | Commit Write (S4) |
            | (Bus Arbitrated)  | (Physical RAM)    |
```

### 2.1 The Hardware Timing: 2-Clock CPU vs 1-Clock RAM
- **Motorola 68000 Bus Cycle:** A standard 68000 read or write cycle spans **4 CPU clocks** ($S_0$ through $S_7$), which equals **2 Color Clocks (CCK1 and CCK2)**.
- **Amiga Chip RAM Speed:** The Amiga DRAM memory bus completes a physical memory transfer in **1 Color Clock (280 ns)**.

### 2.2 Read Cycle Execution
1. **During CCK1 ($S_0$–$S_3$):**
   - The CPU presents the target address and asserts strobes (`_AS`, `_UDS`/`_LDS`).
   - If target is Chip RAM and Agnus DMA is active (`chip_ram_blocked == true`), Gary withholds `_DTACK` and the CPU stalls (wait states).
   - If unblocked, arbitration passes and the CPU transitions to CCK2.
2. **During CCK2 ($S_4$–$S_7$):**
   - At $S_6$, data driven on $D_0$–$D_{15}$ is sampled directly by the CPU into its internal input register (`CpuMicroState.last_read`).
   - Gary asserts `_DTACK`, the CPU completes its cycle, and the bus transaction finishes without an intermediate bus latch.

### 2.3 Write Cycle Execution
1. **During CCK1 ($S_0$–$S_3$):**
   - The CPU presents the target address, asserts `_AS`, and arbitrates for bus readiness.
   - If target is Chip RAM and occupied by DMA, the CPU stalls before $S_4$.
2. **During CCK2 ($S_4$–$S_7$):**
   - The CPU drives write data onto `D0`–`D15` and asserts data strobes during $S_4$.
   - The write is committed directly to physical Chip RAM.
   - `_DTACK` is acknowledged and the cycle completes.

### 2.4 Conclusion: Interleaved Time-Slot Architecture
In normal operating mode (standard display modes, Blitter Nasty disabled), memory access is cleanly arbitrated across odd/even clock slots:
- DMA channels (Audio, Disk, Copper, Bitplane) take priority during their allocated odd slots.
- The CPU runs smoothly in interleaved slots without unnecessary contention.

---

## 3. Strict Separation of Responsibilities

To prevent tight coupling and synchronization bugs, responsibilities are cleanly isolated across subsystems:

| Subsystem | Dedicated Responsibility | Implementation Location |
| :--- | :--- | :--- |
| **`CycleCounter`** | **Pure 64-bit CCK Cycle Counter:** Tracks elapsed global Color Clocks (`total_cck: u64`). Does not track bus phases, beam coordinates, or E-Clock dividers. | `cycle_counter.rs` |
| **`Agnus` (Beam)** | **Master Raster Beam Tracking:** Coordinates horizontal beam position (`HPOS`), vertical scanlines (`VPOS`), `LOF` interlace field bit, and display timing registers `VHPOSR` / `VPOSR`. | `chips/agnus/beam.rs` (see [Agnus.md](Agnus.md)) |
| **`CIA`** | **E-Clock Division & Prescalers:** Tracks internal E-Clock sub-phase divider ($0..4$) to step Timers A and B every 5 CCKs. | `chips/cia/mod.rs` (see [CIA.md](CIA.md)) |
| **`MemoryBus` / CPU** | **Bus Phase State Machine:** Manages CCK1 vs CCK2 arbitration, wait states, and data bus transfer. | `memory_bus/mod.rs` (see [MemoryBus.md](MemoryBus.md)) |

---

## 4. Rust Engine Implementation

The `CycleCounter` is an ultra-lean, copyable, zero-allocation struct whose sole responsibility is counting master Color Clocks:

```rust
use serde::{Deserialize, Serialize};

/// Master hardware cycle counter tracking elapsed Color Clocks (CCK).
/// At ~3.55 MHz, a 64-bit integer will run for over 164,000 years without overflowing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CycleCounter {
    /// Monotonically increasing Color Clock count.
    total_cck: u64,
}

impl CycleCounter {
    pub const PAL_CCK_PER_FRAME: u64 = 70_937;
    pub const NTSC_CCK_PER_FRAME: u64 = 59_605;

    #[inline(always)]
    pub fn new() -> Self {
        Self { total_cck: 0 }
    }

    /// Advance global emulation time by exactly 1 Color Clock (~280 ns).
    #[inline(always)]
    pub fn step_cck(&mut self) {
        self.total_cck = self.total_cck.wrapping_add(1);
    }

    /// Advance global emulation time by a designated number of Color Clocks.
    #[inline(always)]
    pub fn advance_cck(&mut self, count: u64) {
        self.total_cck = self.total_cck.wrapping_add(count);
    }

    #[inline(always)]
    pub fn total_cck(&self) -> u64 {
        self.total_cck
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.total_cck = 0;
    }
}
```