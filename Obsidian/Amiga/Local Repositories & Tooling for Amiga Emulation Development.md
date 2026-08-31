---
title: Amiga & MC68000 Emulation Workspace Reference
created: 2026-08-31
tags:
  - amiga
  - m68000
  - emulation
  - fpga
  - dev-setup
---
This document catalogs all 17 local components in the development workspace. It maps out CPU cores, hardware descriptions (FPGA), complete system emulators, and verification testbenches needed to write and validate a cycle-accurate Amiga emulator (A500/OCS/ECS/AGA) and its Motorola 68000 subsystem.

---

## 1. Motorola 68000 CPU Cores & Development Tools

### [[Moira-3.0]]
- **Repository:** `https://github.com/dirkwhoffmann/Moira`
- **Role:** Cycle-exact, micro-operation based MC68000 CPU core (C++).
- **Why it is needed:** Implements cycle-exact bus cycles ($S_0-S_7$), instruction prefetch queues, and interrupt acknowledgment. Serves as the primary behavioral standard for clock synchronization between the CPU and Amiga custom chips.

### [[Musashi]]
- **Repository:** `https://github.com/kstenerud/Musashi`
- **Role:** Industry-standard portable C MC680x0 CPU emulator core.
- **Why it is needed:** Reference implementation for complete opcode decoding, accurate Condition Code Register (`CCR`/`SR`) calculations, and exception handling logic. Ideal for differential execution testing against your core.

### [[m68k-rs]] (`m68k-rs-m68k-v0.11.6`)
- **Repository:** `https://github.com/benletchford/m68k-rs`
- **Role:** Pure Rust implementation of the M68000–M68060 CPU family.
- **Why it is needed:** Idiomatic Rust reference if writing the emulator in Rust. Features structured memory bus abstraction (`AddressBus`), step debugging, and built-in integration with `SingleStepTests`.

### [[EASy68K]] (`EASy68K-master`)
- **Repository:** `https://github.com/EASy68K/EASy68K` (also hosted at `http://www.easy68k.com`)
- **Role:** 68000 assembly language editor, cross-assembler, and simulator toolchain.
- **Why it is needed:** Essential for writing, assembling, and sanity-testing custom M68k assembly test programs, bare-metal hardware routines, and interrupt handlers before running them on the emulator.

---

## 2. Hardware-Level & FPGA Hardware Description (HDL)

### [[fx68k]]
- **Repository:** `https://github.com/ijor/fx68k`
- **Role:** Cycle-exact, microcode-level MC68000 core in Verilog HDL.
- **Why it is needed:** Ground-truth reference for undocumented micro-architectural behavior, exact bus transaction wait states, prefetch timing, and unaligned word/long-word bus access cycles.

### [[TG68K.C]]
- **Repository:** `https://github.com/TobiFlex/TG68K.C`
- **Role:** Configurable, resource-efficient 68000/68010/68020 FPGA IP core (VHDL).
- **Why it is needed:** Hardware-level state machine reference demonstrating how M68k ALU operations, registers, and memory decoding are implemented in synthesizable logic.

### [[deniser]] (`deniser-1.0.0`)
- **Repository:** `https://github.com/endofexclusive/deniser`
- **Role:** Drop-in FPGA replacement for the Amiga Denise video chip (VHDL).
- **Why it is needed:** Hardware design files detailing how Denise decodes bitplane registers, planar-to-chunky conversion, sprite multiplexing, and Hold-And-Modify (HAM) / Extra Half-Brite (EHB) modes into display output.

### [[Minimig-AGA_MiSTer]]
- **Repository:** `https://github.com/MiSTer-devel/Minimig-AGA_MiSTer`
- **Role:** Full FPGA implementation of OCS/ECS/AGA Amiga hardware for MiSTer.
- **Why it is needed:** Complete HDL architecture reference for bus arbitration between Agnus (DMA slots, Copper, Blitter), Paula (Audio/Floppy), and Denise across PAL/NTSC clock timings.

---

## 3. Test Suites & Hardware Validation

### [[SingleStepTests-m68000]] & [[SingleStepTests-680x0]]
- **Repository:** `https://github.com/SingleStepTests/m68000` / `https://github.com/SingleStepTests/680x0`
- **Role:** Exhaustive per-instruction JSON validation test vectors.
- **Why it is needed:** Provides starting register/memory states and expected cycle-by-cycle output states (including undocumented flags and bus touches). Essential for automated unit-testing / TDD of your CPU interpreter.

### [[amiga-stuff-testkit]] (`amiga-stuff-testkit-v1.21`)
- **Repository:** `https://github.com/keirf/amiga-test-kit`
- **Role:** Amiga diagnostic test suite by Keir Fraser (runs as ADF boot disk).
- **Why it is needed:** End-to-end testing of CIA 8520 timers, MFM floppy disk controller routines, memory auto-configuration, and custom chipset interrupts on a running emulator.

### [[vAmigaTS]]
- **Repository:** `https://github.com/dirkwhoffmann/vAmigaTS`
- **Role:** Amiga regression test suite consisting of ADF test disks and reference frame renders.
- **Why it is needed:** Automated validation of tricky OCS/ECS edge cases, beam-synchronized Copper lists, Blitter fills/lines, and pixel-exact display alignment against hardware snapshots.

---

## 4. Reference Amiga Emulators & System Architectures

### [[WinUAE]] (`WinUAE-6030`)
- **Repository:** `https://github.com/tonioni/WinUAE`
- **Role:** Gold-standard, most comprehensive cycle-exact Amiga emulator.
- **Why it is needed:** Comprehensive reference for hardware edge cases: CIA-A/B timing, floppy PLL decoding, Agnus/CPU bus slot contention, autoconfig Zorro space, and DMA channel scheduling.

### [[vAmiga]] (`vAmiga-4.5`)
- **Repository:** `https://github.com/dirkwhoffmann/vAmiga`
- **Role:** Modern, clean, modular C++ A500/A1000/A2000 cycle-exact emulator.
- **Why it is needed:** Clean object-oriented architectural layout showing how to decouple Agnus, Denise, and Paula across a unified 7.09/7.16 MHz Color Clock DMA grid.

### [[ScriptedAmigaEmulator]]
- **Repository:** `https://github.com/naTmeg/ScriptedAmigaEmulator`
- **Role:** Full Amiga system emulator written in pure JavaScript / HTML5.
- **Why it is needed:** High-level, readable reference for understanding the overall system pipeline, Kickstart ROM bootstrap sequence, and event scheduling without low-level C++ overhead.

### [[MAME]] (`mame-mame0289`)
- **Repository:** `https://github.com/mamedev/mame`
- **Role:** Massive multi-system hardware and CPU emulation framework.
- **Why it is needed:** Reference implementations for shared peripheral chips used in the Amiga (e.g., MOS 6526 / 8520 CIAs, M68k CPU sub-variants, serial/parallel interfaces).

---

## 5. Visual Post-Processing & Presentation

### [[RetroVisor.app]]
- **Repository:** `https://github.com/dirkwhoffmann/RetroVisor`
- **Role:** CRT display and shader pipeline overlay application (by Dirk W. Hoffmann).
- **Why it is needed:** Reference for retro CRT shaders (scanlines, phosphor bloom, shadow mask, chroma smear, curvature) to render authentic Amiga video signals.