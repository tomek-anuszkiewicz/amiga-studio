---
title: "Amiga 500 General System Architecture"
aliases: ["A500 Architecture", "General Architecture", "System Topology"]
tags: ["amiga", "design", "architecture", "crates", "topology"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-09-12
related: ["[Main loop A500.md](Main%20loop%20A500.md)", "[MemoryBus.md](MemoryBus.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)"]
---

# Amiga 500 General System Architecture

> [!NOTE]
> Project-wide engineering constraints, Rust coding guidelines, and WASM requirements are defined in [AGENTS.md](../../../AGENTS.md).

---

## 1. System Block Diagram & Ownership

The emulator is organized around a top-level machine struct named A500, which owns all subsystems and manages coordination without circular handles:

`mermaid
graph TD
    A500["A500 Machine Loop"] --> CPU["CPU (Motorola 68000)"]
    A500 --> BUS["MemoryBus (24-bit / 16-bit)"]
    A500 --> CLK["CycleCounter (64-bit CCK)"]
    A500 --> AGNUS["Agnus (DMA / Copper / Blitter)"]
    A500 --> DENISE["Denise (Video / Bitplanes)"]
    A500 --> PAULA["Paula (Audio / UART / Interrupts)"]
    A500 --> CIAA["CIA-A (Keyboard / Ports / Timer)"]
    A500 --> CIAB["CIA-B (Disk / Timer)"]

    BUS <--> AGNUS
    BUS <--> CPU
`

- **Top-Level Machine (A500)**: Controls stepping (single CCK, multi-cycle, or full video frame), reset lines, and interrupt priority arbitration (IPL 1–6).
- **Decoupled Modules**: Subsystems do not hold references or callbacks to one another; signals and bus requests are driven in the main machine loop and MemoryBus.
- **Circuit Simulation & Signal Propagation**: Hardware components model physical circuit delay. Changes to register latches take effect on subsequent clock phases/cycles rather than propagating instantaneously across chips.

---

## 2. Workspace Crate Architecture & Dependencies

The codebase is organized as a Cargo workspace with decoupled, single-responsibility crates located under crates/:

`mermaid
graph TD
    classDef core fill:#1e3a5f,stroke:#4f9da6,stroke-width:2px,color:#ffffff;
    classDef tool fill:#3d2c40,stroke:#d16ba5,stroke-width:2px,color:#ffffff;
    classDef ext fill:#1c2321,stroke:#5e6472,stroke-width:1px,stroke-dasharray: 5 5,color:#e0e0e0;

    subgraph WorkspaceCrates["Cargo Workspace Crates (crates/*)"]
        CFG["config<br/><code>crates/config</code>"]:::core
        RTC["rtc<br/><code>crates/rtc</code>"]:::core
        MEM["memory_bus<br/><code>crates/memory_bus</code>"]:::core
        CPU["m68000<br/><code>crates/m68000</code>"]:::core
        DIS["disassembler<br/><code>crates/disassembler</code>"]:::tool
        DBG["debugger<br/><code>crates/debugger</code>"]:::tool
        TR["test_runner<br/><code>crates/test_runner</code>"]:::tool
        GUI["gui<br/><code>crates/gui</code>"]:::tool
    end

    subgraph ExternalDeps["Key External Crates"]
        SERDE["serde / serde_json<br/>(no_std + alloc)"]:::ext
        BF["bitflags"]:::ext
        GZ["flate2"]:::ext
        EGF["eframe / egui"]:::ext
        RFD["rfd"]:::ext
    end

    %% Internal Dependencies
    RTC -->|depends on| CFG
    MEM -->|depends on| CFG
    MEM -->|depends on| RTC
    CPU -->|depends on| MEM
    DBG -->|depends on| CPU
    DBG -->|depends on| MEM
    DBG -->|depends on| DIS
    TR -->|depends on| CPU
    TR -->|depends on| MEM
    TR -->|depends on| DIS
    TR -->|depends on| DBG
    GUI -->|depends on| CPU
    GUI -->|depends on| MEM
    GUI -->|depends on| DBG
    GUI -->|depends on| CFG

    %% External Dependencies
    CFG -.-> SERDE
    RTC -.-> SERDE
    MEM -.-> SERDE
    CPU -.-> SERDE
    CPU -.-> BF
    DBG -.-> SERDE
    TR -.-> SERDE
    TR -.-> GZ
    GUI -.-> EGF
    GUI -.-> RFD
`

### Crate Descriptions & Responsibilities

| Crate | Path | Responsibility | Workspace Dependencies |
| :--- | :--- | :--- | :--- |
| **config** | [crates/config](../../../crates/config) | Encapsulated, read-only hardware presets (Bare512k, Standard1Mb, ExpandedPowerUser), RtcModel, video timings. | *None* |
| **rtc** | [crates/rtc](../../../crates/rtc) | OKI MSM6242B Real-Time Clock & Calendar emulation, BCD latches, standalone civil calendar arithmetic, and CCK cycle stepping. | config |
| **memory_bus** | [crates/memory_bus](../../../crates/memory_bus) | 24-bit physical address space, 256-entry 64KB bank table (`addr >> 16`), 2-phase CCK arbitration, open bus emulation. | config, rtc |
| **m68000** | [crates/m68000](../../../crates/m68000) | Cycle-exact Motorola 68000 CPU core, 65,536-entry compile-time static dispatch table, registers, ALU, prefetch queue. | memory_bus |
| **debugger** | [crates/debugger](../../../crates/debugger) | Headless inspection and debugging subsystem, disassembler, mini-assembler, temporal time-travel engine, breakpoints, and watchpoints. | m68000, memory_bus |
| **test_runner** | [crates/test_runner](../../../crates/test_runner) | Automated validation against Tom Harte SingleStepTests physical silicon vectors, cycle-exact instruction benchmarking engine, execution trace audit logger (--dump-traces), and Cartesian DMA contention suite. | m68000, memory_bus, debugger |
| **gui** | [crates/gui](../../../crates/gui) | Immediate-mode Developer Studio & Standalone GUI (egui/eframe for Desktop + WASM), live CPU/memory inspection, temporal time-travel scrubber, and breakpoints manager. | m68000, memory_bus, debugger, config, rtc |

---

## 3. External Interfaces & Data Flow

- **ROM & Disk Injection**: Kickstart ROM images and disk buffers are injected from the outside as raw byte slices (&[u8]).
- **Decoupled Host I/O**:
  - Video rendering outputs into a dedicated frame buffer.
  - Audio outputs into decoupled sample ring buffers.
  - Save states serialize full machine state to/from JSON (see [SaveState.md](SaveState.md)).

---

## 4. Subsystem Reference Links

- [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md): Register architecture, condition codes, exception vectors, and instruction set.
- [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md): Cycle-exact micro-operations, bus strobes, pipeline refills, and micro-step traces.
- [CPU SingleStepTests.md](CPU%20SingleStepTests.md): Verification suite against Tom Harte physical silicon test vectors.
- [CPU Instruction Benchmarking.md](CPU%20Instruction%20Benchmarking.md): Cycle-exact benchmarking engine, memory map, execution trace dumping (--dump-traces), and timing anomaly detection.
- [CPU Instruction Benchmark Catalog.md](CPU%20Instruction%20Benchmark%20Catalog.md): Comprehensive test case catalog across all instructions, addressing modes, and operand permutations.
- [CPU Instruction Benchmark Strategies.md](CPU%20Instruction%20Benchmark%20Strategies.md): Loop construction patterns, PRNG data synthesis, and cycle timing strategies.
- [Main loop A500.md](Main%20loop%20A500.md): Machine stepping, reset sequence, and interrupt arbitration.
- [MemoryBus.md](MemoryBus.md): 2-phase CCK arbitration, address decoding, and DMA contention.
- [Agnus.md](Agnus.md): Master beam counters, DMA arbiter, Copper, and 4-channel Blitter.
- [Denise.md](Denise.md): Video pixel serializer, bitplanes, sprites, palette, and collisions.
- [Paula.md](Paula.md): 4-channel DMA audio, floppy disk MFM controller, serial UART, and central interrupt multiplexer.
- [CIA.md](CIA.md): Dual MOS 8520 Complex Interface Adapters (timers, TOD, SDR, parallel & control ports).
- [CycleCounter.md](CycleCounter.md): Master Color Clock counter.
- [SaveState.md](SaveState.md): State serialization model.
- [Configuration.md](Configuration.md): Machine configuration, RAM sizes, chipset models, and ROM injection.
- [Debugger.md](Debugger.md): Headless debugger backend, stepping, breakpoints, and disassembler.
- [GUI.md](GUI.md): Frontend architecture, video viewport, audio sink, DPI/theme adaptation, and decoupled interfaces.
- [GUI Specification.md](GUI%20Specification.md): Concrete layout and operational specification for developer GUI, panels, memory editor, and 1:1 file hierarchy.
- [Joystick.md](Joystick.md): Game port digital/analog joysticks, 4-player parallel adapter, and host gamepad mapping.
- [Mouse.md](Mouse.md): Port 1 quadrature counters (JOY0DAT), buttons, pointer locking, and touchscreen mapping.
- [Keyboard.md](Keyboard.md): Microcontroller serial protocol, scancode matrix, Ctrl-Amiga-Amiga reset, and host layout-independent key mapping.
- [Floppy.md](Floppy.md): 3.5" DD drive mechanics, multi-chip interface (CIA-A, CIA-B, Paula, Agnus), MFM track layout, and ADF ingestion.
- [RTC.md](RTC.md): OKI MSM6242B real-time clock, battery-backed registers, BCD calendar, and 24/12h timing.
