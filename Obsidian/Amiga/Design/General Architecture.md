---
title: "Amiga 500 General System Architecture"
aliases: ["A500 Architecture", "General Architecture", "System Topology"]
tags: ["amiga", "design", "architecture", "crates", "topology"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-10-14
related: ["[Machine Loop.md](Machine%20Loop.md)", "[Game Ports.md](Game%20Ports.md)", "[MemoryBus.md](MemoryBus.md)", "[Physical Memory.md](Physical%20Memory.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Interrupts.md](Interrupts.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)", "[Testing Strategy and Quality Assurance.md](Verification/Testing%20Strategy%20and%20Quality%20Assurance.md)"]
---

# Amiga 500 General System Architecture

> [!NOTE]
> Detailed inter-chip signal propagation rules are codified in [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md) and [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md).

---

## 1. System Block Diagram & Ownership

The emulator is organized around a top-level machine struct named A500, which owns all subsystems and manages coordination without circular handles:

```mermaid
graph TD
    A500["A500 Machine Loop"] --> CPU["CPU (Motorola 68000)"]
    A500 --> BUS["MemoryBus (24-bit / 16-bit)"]
    A500 --> CLK["Master CCK Counter (u64)"]
    A500 --> AGNUS["Agnus (DMA / Copper / Blitter)"]
    A500 --> DENISE["Denise (Video / Bitplanes)"]
    A500 --> PAULA["Paula (Audio / UART / Interrupts)"]
    A500 --> CIAA["CIA-A (Keyboard / Ports / Timer)"]
    A500 --> CIAB["CIA-B (Disk / Timer)"]

    BUS <--> AGNUS
    BUS <--> CPU
```

- **Top-Level Machine (A500)**: Controls stepping (single CCK, multi-cycle, or full video frame), reset lines, and interrupt priority arbitration (IPL 1–6).
- **Decoupled Modules**: Subsystems do not hold references or callbacks to one another; signals and bus requests are driven in the main machine loop and MemoryBus.
- **Circuit Simulation & Signal Propagation**: Hardware components model physical circuit delay. Changes to register latches take effect on subsequent clock phases/cycles rather than propagating instantaneously across chips.

### 1.1 Physical Bus Topology & Inter-Chip Isolation Invariant

In the physical Commodore Amiga 500 architecture, custom chips do not share an open software memory bus, nor do they communicate via dynamic callbacks or mutual references. The system strictly adheres to three immutable physical hardware rules:

```mermaid
flowchart TD
    subgraph AGNUS_BLOCK ["Agnus (Exclusive Bus Master & Address Generator)"]
        AGNUS_DMA["DMA Channel Pointers\n(BPLxPT, SPRxPT, AUDxPT, DSKPT)"]
        RGA_GEN["Register Address Generator\n(RGA8..1 Bus Driver)"]
    end

    subgraph BUSES ["Physical Motherboard Bus Infrastructure"]
        ADDR_BUS["Agnus DMA Address<br/>$000000-$07FFFF"]
        RGA_BUS["Internal RGA Bus (RGA8..1)"]
        DATA_BUS["16-bit Bidirectional Data Bus (D15..0)"]
    end

    subgraph MEM ["Physical Memory"]
        CRAM["Chip RAM (512 KB)"]
    end

    subgraph RECEIVERS ["Specialized Custom Chips (Passive Bus Latchers)"]
        DENISE["Denise\nLatches BPLxDAT, SPRxDAT\n(Zero Direct Memory Reads)"]
        PAULA["Paula\nLatches AUDxDAT, DSKDAT\n(Zero Direct Memory Reads)"]
    end

    AGNUS_DMA -->|Drives Memory Address| ADDR_BUS
    RGA_GEN -->|Drives Register Offset| RGA_BUS
    ADDR_BUS --> CRAM
    CRAM -->|Places Word onto Bus| DATA_BUS
    RGA_BUS -->|Strobe Match| DENISE
    RGA_BUS -->|Strobe Match / DMAL| PAULA
    DATA_BUS -->|Latched by Strobe| DENISE
    DATA_BUS -->|Latched by Strobe| PAULA
```

1. **Strict Prohibition of Direct Cross-Chip Signal Smuggling:**
   - Custom chips (`Agnus`, `Denise`, `Paula`, `CIAs`, `CPU`) **must never** hold direct pointers or invoke mutating methods directly on each other.
   - All inter-chip interactions (DMA requests, interrupt lines, blanking, strobes) represent physical copper PCB traces coordinated through the machine loop and `MemoryBus`.
2. **Agnus as the Exclusive DMA Address Master:**
   - Agnus is the **sole bus master and address generator** for all autonomous Chip RAM DMA channels (Bitplanes, Sprites, Audio, Floppy Disk, Copper, and Blitter).
   - The supported 8370 and 8371 models address the lower 512 KB for DMA; all current machine presets install 512 KB of Chip RAM.
   - Agnus places the memory address onto the Chip RAM address bus and simultaneously asserts the target custom register offset onto the internal Register Address (`RGA`) bus.
3. **Specialized Custom Chips as Passive Bus Latchers (Zero Direct Memory Reads):**
   - Neither **Denise** nor **Paula** possesses DMA address generation circuits, and neither holds pointers to `PhysicalMemory`.
   - Specialized chips **never initiate autonomous memory reads**. During an assigned DMA slot, the memory system reads the 16-bit word from Chip RAM onto the shared data bus, and the receiving chip passively latches the word into its internal holding register (`BPLxDAT`, `SPRxDAT`, `AUDxDAT`, `DSKDAT`) upon matching its `RGA` strobe.
4. **`MachineLoop` as Motherboard PCB Simulator & Post-CCK `poll_*` Dispatch:**
   - The top-level machine struct (`MachineLoop` / `A500Machine`) acts as the physical motherboard simulator.
   - After each Color Clock cycle, the motherboard interrogates subsystem output pins via explicit `poll_*` methods (e.g. `agnus.poll_blitter_irq()`, `agnus.poll_copper_write()`, `agnus.poll_bpl_dma()`, `paula.poll_audio_restart()`, `floppy.poll_dskblk_irq()`, `cia_a.irq_pending()`).
   - The motherboard routes these events to the target subsystem inputs (e.g. `paula.set_interrupt_request()`, `denise.write_bpldat()`, `agnus.reload_audio_ptr()`). No chip ever notifies or mutates a peer chip directly.

---

## 2. Workspace Crate Architecture & Dependencies

The codebase is organized as a Cargo workspace with decoupled, single-responsibility crates located under crates/:

```mermaid
graph TD
    classDef top fill:#0f2537,stroke:#38ef7d,stroke-width:2px,color:#ffffff;
    classDef core fill:#1e3a5f,stroke:#4f9da6,stroke-width:2px,color:#ffffff;
    classDef chip fill:#2d1b4e,stroke:#9d4edd,stroke-width:2px,color:#ffffff;
    classDef subchip fill:#3b1e36,stroke:#f72585,stroke-width:1px,color:#ffffff;
    classDef periph fill:#1b3b22,stroke:#52b788,stroke-width:2px,color:#ffffff;
    classDef tool fill:#3d2c40,stroke:#d16ba5,stroke-width:2px,color:#ffffff;

    subgraph TopLevel["Top-Level Machine Chassis"]
        ML["machine_loop<br/><code>crates/machine_loop</code>"]:::top
    end

    subgraph MemoryAndCPU["Memory Storage & CPU Core"]
        CFG["config<br/><code>crates/config</code>"]:::core
        PMEM["physical_memory<br/><code>crates/physical_memory</code>"]:::core
        MBUS["memory_bus<br/><code>crates/memory_bus</code>"]:::core
        RTC["rtc<br/><code>crates/rtc</code>"]:::core
        CPU["m68000<br/><code>crates/cpu</code>"]:::core
    end

    subgraph CustomChips["Custom Chip Coordinators & Coprocessors"]
        AGNUS["agnus<br/><code>crates/agnus</code>"]:::chip
        COP["copper<br/><code>crates/copper</code>"]:::subchip
        BLT["blitter<br/><code>crates/blitter</code>"]:::subchip
        DMA["dma<br/><code>crates/dma</code>"]:::subchip

        DENISE["denise<br/><code>crates/denise</code>"]:::chip
        SPR["sprites<br/><code>crates/sprites</code>"]:::subchip
        FB["frame_builder<br/><code>crates/frame_builder</code>"]:::subchip

        PAULA["paula<br/><code>crates/paula</code>"]:::chip
        AUD["audio<br/><code>crates/audio</code>"]:::subchip
        INTR["interrupts<br/><code>crates/interrupts</code>"]:::subchip

        CIA["cia (A & B)<br/><code>crates/cia</code>"]:::chip
    end

    subgraph Peripherals["Peripherals & Input Devices"]
        GP["game_ports<br/><code>crates/game_ports</code>"]:::periph
        MOU["mouse<br/><code>crates/mouse</code>"]:::subchip
        JOY["joystick<br/><code>crates/joystick</code>"]:::subchip
        FLP["floppy<br/><code>crates/floppy</code>"]:::periph
        KBD["keyboard<br/><code>crates/keyboard</code>"]:::periph
    end

    subgraph Diagnostics["Diagnostics, Debugger & GUI"]
        DIS["disassembler<br/><code>crates/disassembler</code>"]:::tool
        DBG["debugger<br/><code>crates/debugger</code>"]:::tool
        GUI["gui<br/><code>crates/gui</code>"]:::tool
        TR["test_runner<br/><code>crates/test_runner</code>"]:::tool
    end

    %% Chassis Ownership & Routing
    ML --> CPU
    ML --> PMEM
    ML --> MBUS
    ML --> AGNUS
    ML --> DENISE
    ML --> PAULA
    ML --> CIA
    ML --> FLP
    ML --> KBD
    ML --> GP
    ML --> RTC

    %% Logical Containment / Re-exports
    AGNUS --> COP
    AGNUS --> BLT
    AGNUS --> DMA

    DENISE --> SPR
    DENISE --> FB

    PAULA --> AUD
    PAULA --> INTR

    GP --> MOU
    GP --> JOY

    %% Memory & Bus Connections
    PMEM --> CFG
    PMEM --> RTC
    MBUS --> PMEM
    CPU --> PMEM

    %% Tooling Dependencies
    DBG --> CPU
    DBG --> PMEM
    DBG --> DIS
    GUI --> DBG
    TR --> DBG
```

### Crate Descriptions & Responsibilities

| Crate | Path | Responsibility | Workspace Dependencies |
| :--- | :--- | :--- | :--- |
| **config** | [crates/config](../../../crates/config) | Encapsulated, read-only hardware presets (Bare512k, Standard1Mb, ExpandedPowerUser), chip models, RtcModel, video timings. | *None* |
| **copper** | [crates/copper](../../../crates/copper) | Agnus Copper coprocessor (MOVE, WAIT, SKIP, CDANG danger mode). | serde |
| **blitter** | [crates/blitter](../../../crates/blitter) | Agnus 4-channel DMA Blitter, minterm ALU, barrel shifters, and line drawer. | serde |
| **dma** | [crates/dma](../../../crates/dma) | Agnus DMA slot scheduler (227.5 CCK horizontal schedule), channel gating (`DMACON`), and Chip RAM contention. | serde |
| **agnus** | [crates/agnus](../../../crates/agnus) | Agnus (MOS 8370/8371) chip coordinator and beam counters (`VHPOSR`, `VPOSR`). | config, serde |
| **sprites** | [crates/sprites](../../../crates/sprites) | Denise 8 hardware sprite engines, coordinate comparators, attached pairs, multiplexing. | serde |
| **frame_builder** | [crates/frame_builder](../../../crates/frame_builder) | Denise raster scanline compositor, display window clipping, and 32-bit ARGB frame buffer generation. | serde |
| **mouse** | [crates/mouse](../../../crates/mouse) | Amiga 2/3-button quadrature mouse and `JOY0DAT` encoding. | serde |
| **joystick** | [crates/joystick](../../../crates/joystick) | Digital Atari 9-pin standard joystick and `JOYxDAT` direction switch XOR encoding. | serde |
| **game_ports** | [crates/game_ports](../../../crates/game_ports) | Amiga dual 9-pin controller ports (Port 1 & Port 2), pluggable device slots (`Mouse`, `Joystick`), Denise/Paula/CIA-A decoders. | mouse, joystick, serde |
| **denise** | [crates/denise](../../../crates/denise) | Denise (MOS 8362) video processor, bitplanes, palette (`COLOR00`–`COLOR31`), and collision registers. | config, serde |
| **audio** | [crates/audio](../../../crates/audio) | Paula 4-channel 8-bit DMA audio engine, volume scaling (0..64), and stereo panning. | config, serde |
| **floppy** | [crates/floppy](../../../crates/floppy) | 3.5" DD floppy drive mechanics (80 cylinders, 2 heads) and Paula MFM DMA controller. | serde |
| **interrupts** | [crates/interrupts](../../../crates/interrupts) | Central 14-source interrupt priority controller (`INTENA`, `INTREQ`), IPL 1..6 encoder, and atomic SET/CLR bit 15 logic. | serde |
| **paula** | [crates/paula](../../../crates/paula) | Paula (MOS 8364) chip coordinator, serial UART transceiver, and audio/interrupt staging. | audio, config, interrupts, serde |
| **keyboard** | [crates/keyboard](../../../crates/keyboard) | MOS 6500/1 keyboard microcontroller, scancode matrix, serial stream, and Ctrl-Amiga-Amiga reset. | serde |
| **cia** | [crates/cia](../../../crates/cia) | MOS 8520 Complex Interface Adapter (Timers A & B, Ports A & B / parallel Centronics lines, TOD, SDR, ICR). | serde |
| **rtc** | [crates/rtc](../../../crates/rtc) | OKI MSM6242B Real-Time Clock & Calendar emulation, BCD latches, civil calendar arithmetic. | config, serde |
| **physical_memory** | [crates/physical_memory](../../../crates/physical_memory) | 24-bit physical address space, 256-entry 64KB bank table (`addr >> 16`), 2-phase CCK arbitration, open bus emulation. | config, rtc |
| **memory_bus** | [crates/memory_bus](../../../crates/memory_bus) | Motherboard address router (`MemoryBus<'a>`), dispatching 24-bit address space live to physical storage, custom chips, and peripherals. | agnus, audio, blitter, cia, copper, denise, dma, floppy, frame_builder, paula, physical_memory, rtc, sprites |
| **m68000** | [crates/cpu](../../../crates/cpu) | Cycle-exact Motorola 68000 CPU core, 65,536-entry compile-time static dispatch table, registers, ALU, prefetch queue. | physical_memory |
| **machine_loop** | [crates/machine_loop](../../../crates/machine_loop) | Tier 0 top-level machine facade owning CPU, memory bus router, monotonic `u64` CCK counter, custom chips, coprocessors, and devices in a flat structure with parameter-based cycle stepping. | agnus, audio, blitter, cia, config, copper, denise, dma, floppy, frame_builder, game_ports, joystick, keyboard, m68000, memory_bus, mouse, paula, physical_memory, rtc, sprites, serde |
| **disassembler** | [crates/disassembler](../../../crates/disassembler) | Cycle-exact M68000 instruction disassembler. | *None* |
| **debugger** | [crates/debugger](../../../crates/debugger) | Headless inspection and debugging subsystem, temporal time-travel engine, breakpoints, and watchpoints. | m68000, physical_memory, disassembler |
| **test_runner** | [crates/test_runner](../../../crates/test_runner) | Aggregate SingleStep verification with shared reporting helpers, automated validation against Tom Harte SingleStepTests, and cycle-exact benchmarking. | cpu, physical_memory, serde, serde_json, flate2, sha2 (development dependencies) |
| **gui** | [crates/gui](../../../crates/gui) | Immediate-mode Developer Studio & Standalone GUI (egui/eframe), live CPU/memory inspection, temporal time-travel scrubber. | m68000, physical_memory, debugger, config, rtc |

---

## 3. External Interfaces & Data Flow

- **ROM & Disk Injection**: Kickstart ROM images and disk buffers are injected from the outside as raw byte slices (&[u8]).
- **Decoupled Host I/O**:
  - Video rendering outputs into a dedicated frame buffer.
  - Audio outputs into decoupled sample ring buffers.
  - Save states serialize full machine state to/from JSON (see [SaveState.md](Application/SaveState.md)).

---

## 4. Subsystem Reference Links

Hardware explanations in the subsystem notes describe the behavior to
reconstruct from the cited Amiga manuals. Links to Rust source show the
current implementation, whose coverage must be checked separately; a source
link does not establish that every described hardware path is implemented.

- [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md): External CPU state, bus interface, prefetch, exceptions, interrupts, and startup.
- [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md): Internal microsteps, CCK bus phases, waits, and execution state.
- [CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md): Verification suite against Tom Harte physical silicon test vectors.
- [CPU Instruction Benchmarking.md](Performance/CPU%20Instruction%20Benchmarking.md): CPU benchmark purpose, workload strategies, 108-case catalog, measurement analysis, and validation limits.
- [Machine Loop.md](Machine%20Loop.md): Machine stepping, reset sequence, and interrupt arbitration.
- [MemoryBus.md](MemoryBus.md): 2-phase CCK arbitration, address decoding, and DMA contention.
- [Physical Memory.md](Physical%20Memory.md): Installed RAM and ROM banks, boot overlay, open-bus behavior, and CPU memory contention.
- [Agnus.md](Agnus.md): Master beam counters, DMA arbiter, Copper, and 4-channel Blitter.
- [Denise.md](Denise.md): Video pixel serializer, bitplanes, sprites, palette, and collisions.
- [Paula.md](Paula.md): 4-channel DMA audio, floppy disk MFM controller, serial UART, and central interrupt multiplexer.
- [CIA.md](CIA.md): Dual MOS 8520 Complex Interface Adapters (timers, TOD, SDR, parallel & control ports).
- [SaveState.md](Application/SaveState.md): State serialization model.
- [Configuration.md](Configuration.md): Machine configuration, RAM sizes, chipset models, and ROM injection.
- [Debugger.md](Application/Debugger.md): Headless debugger backend, stepping, breakpoints, and disassembler.
- [GUI.md](Application/GUI.md): Frontend role, game and debugger views, intended input and output paths, scaling, and themes.
- [Joystick.md](Joystick.md): Game port digital/analog joysticks, 4-player parallel adapter, and host gamepad mapping.
- [Game Ports.md](Game%20Ports.md): Dual 9-pin controller ports (Port 1 & Port 2), signal routing across Denise, Paula, and CIA-A, and host event routing.
- [Mouse.md](Mouse.md): Amiga 500 mouse connected to a game port.
- [Keyboard.md](Keyboard.md): Independent keyboard controller, CIA-A serial reading, A500 reset, and the power LED's audio-filter connection.
- [Floppy.md](Floppy.md): Drive behavior integrated with disk-related registers and signals from CIA-A, CIA-B, Paula, and Agnus.
- [RTC.md](RTC.md): OKI MSM6242B real-time clock, battery-backed registers, BCD calendar, and 24/12h timing.
- [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md): Centralized master catalog of hardware idiosyncrasies, silicon timing traps, and anti-tamper invariants across CPU, chipset, and memory bus.
- [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md): Closed silicon signal space, inter-chip bus propagation latencies, conflict resolution, and action dispatch matrix.
- [Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Column definitions and table format for documenting custom register ownership, access, propagation, and timing.
- [Testing Strategy and Quality Assurance.md](Verification/Testing%20Strategy%20and%20Quality%20Assurance.md): 3-Tier Testing Architecture, external tests/ architecture, and verification tooling.


