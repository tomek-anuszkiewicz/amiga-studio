# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance **Commodore Amiga 500** (OCS) emulator written in Rust, engineered for native desktop targets (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).

---

## 1. Project Purpose & Architecture

The goal of this repository is to build a modern, system-agnostic, cycle-exact Amiga 500 emulator:

- **Cycle-Exact Clock Granularity:** Synchronized to the Amiga Color Clock (**CCK**, ~3.54 MHz PAL / ~3.58 MHz NTSC). 1 M68000 bus cycle = 4 CPU clocks = 2 CCK cycles (CCK1 and CCK2).
- **Two-Phase Bus Contention:** Memory bus transactions observe bus availability via `MemoryBusResult`. Contention on Chip RAM caused by Agnus DMA naturally stalls the CPU without synthetic hacks.
- **Circuit Simulation:** Inter-chip signals and register writes propagate on subsequent clock phases or cycles, mimicking real physical hardware delay.
- **Zero Host Panics & Strict Endianness:** Safe Big-Endian decoding (`from_be_bytes`), wrapping arithmetic (`wrapping_add`), and zero panics on unmapped guest memory reads (simulating open bus `$FF`).
- **Decoupled Architecture:** No circular references between subsystems (`Cpu`, `MemoryBus`, `Agnus`, `Denise`, `Paula`, `CIAs`). All orchestration is driven by the top-level machine loop (`A500`).
- **WASM-Ready & Headless Core:** Core crate has zero platform dependencies (no `std::fs`, `std::thread`, or `std::time::Instant`). All ROMs, disk images, video buffers, and audio streams are passed across decoupled interfaces.

---

## 2. Repository Structure

```
├── .agents/                 <- Antigravity IDE configuration, rules, and skills
│   ├── rules/               <- Local rules (amiga-rag, graphify)
│   └── skills/              <- On-demand runbooks (add-m68k-instruction, m68k-singlestep-test)
├── Obsidian/                <- Complete technical knowledge base
│   └── Amiga/
│       ├── Design/          <- Component architecture specifications
│       │   ├── Configuration.md
│       │   ├── CPU Motorola M68000.md
│       │   ├── CPU SingleStepTests.md
│       │   ├── Debugger.md
│       │   ├── General Architecture.md
│       │   ├── GUI.md
│       │   ├── Main loop A500.md
│       │   ├── MemoryBus.md
│       │   ├── SaveState.md
│       │   └── Specialized Chips.md
│       └── Reference/       <- Official Commodore HRM, PRMs, Guru book (indexed by RAG)
├── ref_src/                 <- Local reference cores, emulators, HDL, and test suites
│   ├── SingleStepTests-m68000/
│   ├── Moira-3.0/
│   ├── Musashi/
│   ├── WinUAE-6030/
│   ├── vAmiga-4.5/
│   ├── fx68k/
│   └── ...
├── schematics/              <- Consolidated hardware schematics and IC datasheets
│   ├── a500/                <- Active Phase 1 schematics (Rev 5, Rev 6A/7, A501, IC datasheets)
│   └── other-models/        <- Other Commodore Amiga models (A1000, A1200, A2000, etc.)
├── tools/                   <- Developer tools and offline table generators
│   ├── BlebGenerator/       <- C# sinc BLEP table generator
│   └── winguide/            <- AmigaGuide viewer utility
├── tests/                   <- Test media, testbenches, and disk images
│   └── disks/               <- ADF test disk images (AmigaTestKit)
├── archive/                 <- Cold storage for digitized / inactive raw sources
│   ├── docs-original/       <- Raw sources digitized into Obsidian Reference
│   └── docs-non-a500/       <- Inactive manuals (68020+, AGA, etc.)
├── AGENTS.md                <- Authoritative project rules and Rust systems guidelines
├── ROADMAP.md               <- Project milestones, implementation steps, and agent testing strategy
└── README.md
```

---

## 3. Post-Clone Setup & Prerequisites

Follow these one-time preparation steps after cloning the repository:

### 3.1 Rust Toolchain
Ensure Rust stable is installed with the WASM target:
```powershell
rustup target add wasm32-unknown-unknown
```

### 3.2 Decode SingleStepTests JSON Files
The 127 M68000 test files in `ref_src/SingleStepTests-m68000` are stored as compressed binary dumps. Convert them to `.json`:
```powershell
cd ref_src/SingleStepTests-m68000
python decode.py
cd ../..
```

### 3.3 Musashi Source Generation (Optional Reference)
To compile or generate sources for the Musashi reference core:
```powershell
docker run --rm -v "${PWD}/ref_src/Musashi:/src" -w /src gcc:latest sh -c "gcc -o m68kmake m68kmake.c && ./m68kmake"
```

### 3.4 Trim MAME Repository (Optional Reference)
If using MAME's 68000 microcode implementation as a reference, you can prune unused subdirectories to save disk space:
```powershell
cd ref_src/mame-mame0289
Move-Item -Path "src\devices\cpu\m68000" -Destination "m68000_temp"
Get-ChildItem -Exclude "m68000_temp" | Remove-Item -Recurse -Force
New-Item -ItemType Directory -Path "src\devices\cpu" -Force | Out-Null
Move-Item -Path "m68000_temp" -Destination "src\devices\cpu\m68000"
cd ../..
```

### 3.5 AI Code Intelligence & Tooling
To drive autonomous agents and semantic navigation across this codebase:
```powershell
# Install AST-Grep for syntax-aware pattern searches
winget install ast-grep

# Update shell if using uv / Python tooling
uv tool update-shell
```

---

## 4. Local Reference Repositories & Tooling Catalog

The `ref_src/` directory houses 17 local reference implementations, testbenches, and hardware descriptions:

### 4.1 Motorola 68000 CPU Cores
- **[Moira 3.0](file:///d:/Programowanie/Amiga/ref_src/Moira-3.0)** (`C++`) — [GitHub](https://github.com/dirkwhoffmann/Moira): Cycle-exact, micro-operation based MC68000 core by Dirk W. Hoffmann. Primary behavioral standard for bus cycle phases ($S_0-S_7$), instruction prefetch, and CCK clock synchronization.
- **[Musashi](file:///d:/Programowanie/Amiga/ref_src/Musashi)** (`C`) — [GitHub](https://github.com/kstenerud/Musashi): Industry-standard portable 680x0 emulator core by Karl Stenerud. Reference for complete opcode decoding, CCR flags, and exception frames.
- **[m68k-rs](file:///d:/Programowanie/Amiga/ref_src/m68k-rs-m68k-v0.11.6)** (`Rust`) — [GitHub](https://github.com/benletchford/m68k-rs): Pure Rust M68000–M68060 core featuring clean bus abstraction (`AddressBus`).
- **[EASy68K](file:///d:/Programowanie/Amiga/ref_src/EASy68K-master)** — [GitHub](https://github.com/EASy68K/EASy68K) / [Web](http://www.easy68k.com): 68000 assembly editor, assembler, and simulator toolchain for authoring bare-metal test routines.

### 4.2 Hardware & FPGA Descriptions (HDL)
- **[fx68k](file:///d:/Programowanie/Amiga/ref_src/fx68k)** (`Verilog`) — [GitHub](https://github.com/ijor/fx68k): Cycle-exact, microcode-level 68000 hardware description by Jorge Cwik (ijor). Ground-truth reference for silicon-level microcode, prefetch refills, and bus wait states.
- **[TG68K.C](file:///d:/Programowanie/Amiga/ref_src/TG68K.C)** (`VHDL`) — [GitHub](https://github.com/TobiFlex/TG68K.C): Synthesizable 68000 FPGA core by Tobias Gubener.
- **[deniser](file:///d:/Programowanie/Amiga/ref_src/deniser-1.0.0)** (`VHDL`) — [GitHub](https://github.com/endofexclusive/deniser): Drop-in FPGA replacement for the Amiga Denise video chip detailing planar-to-chunky conversion, sprite multiplexing, and HAM/EHB modes.
- **[Minimig-AGA_MiSTer](file:///d:/Programowanie/Amiga/ref_src/Minimig-AGA_MiSTer)** (`Verilog`) — [GitHub](https://github.com/MiSTer-devel/Minimig-AGA_MiSTer): Full Amiga OCS/ECS/AGA hardware implementation on MiSTer FPGA. Reference for DMA bus slot arbitration across Agnus, Denise, and Paula.

### 4.3 Test Suites & Verification
- **[SingleStepTests-m68000](file:///d:/Programowanie/Amiga/ref_src/SingleStepTests-m68000)** — [GitHub](https://github.com/SingleStepTests/m68000): 127 exhaustive per-instruction JSON validation test suites generated from MAME's microcoded core. Provides register/memory/prefetch starting conditions and expected cycle-by-cycle output states.
- **[SingleStepTests-680x0](file:///d:/Programowanie/Amiga/ref_src/SingleStepTests-680x0)** — [GitHub](https://github.com/SingleStepTests/680x0): Tom Harte's single-step processor test vectors.
- **[amiga-stuff-testkit](file:///d:/Programowanie/Amiga/ref_src/amiga-stuff-testkit-v1.21)** — [GitHub](https://github.com/keirf/amiga-test-kit): Keir Fraser's Amiga Test Kit (ADF boot disk) for testing CIA timers, floppy PLL decoding, memory autoconfig, and chipset interrupts.
- **[vAmigaTS](file:///d:/Programowanie/Amiga/ref_src/vAmigaTS)** — [GitHub](https://github.com/dirkwhoffmann/vAmigaTS): Automated regression test suite consisting of ADF test disks and reference video renders for Copper lists, Blitter fills, and raster effects.

### 4.4 Reference System Emulators
- **[WinUAE](file:///d:/Programowanie/Amiga/ref_src/WinUAE-6030)** — [GitHub](https://github.com/tonioni/WinUAE): Most comprehensive cycle-exact Amiga emulator by Toni Wilen. Ultimate reference for edge cases (floppy MFM sync, CIA TOD timers, Gary/Agnus bus contention).
- **[vAmiga](file:///d:/Programowanie/Amiga/ref_src/vAmiga-4.5)** — [GitHub](https://github.com/dirkwhoffmann/vAmiga): Clean, object-oriented C++ A500/A1000/A2000 emulator by Dirk W. Hoffmann. Reference for decoupling Agnus, Denise, and Paula across a unified CCK grid.
- **[ScriptedAmigaEmulator](file:///d:/Programowanie/Amiga/ref_src/ScriptedAmigaEmulator)** — [GitHub](https://github.com/naTmeg/ScriptedAmigaEmulator): High-level JavaScript Amiga emulator by Rupert Hausberger.
- **[MAME](file:///d:/Programowanie/Amiga/ref_src/mame-mame0289)** — [GitHub](https://github.com/mamedev/mame): Reference implementations for shared peripheral chips (MOS 8520 CIA, M68000 CPU).

### 4.5 Visual Post-Processing
- **[RetroVisor.app](file:///d:/Programowanie/Amiga/ref_src/RetroVisor.app)** — [GitHub](https://github.com/dirkwhoffmann/RetroVisor): CRT shader pipeline reference (scanlines, phosphor bloom, curvature, shadow mask) by Dirk W. Hoffmann.

---

## 5. Compiling & Running Tests

### Standard Compilation & Checks
```powershell
# Build emulator core
cargo build

# Typecheck for WebAssembly
cargo check --target wasm32-unknown-unknown

# Run all standard unit tests
cargo test
```

### Running M68000 SingleStepTests
Execute instruction test suites individually or by group:
```powershell
# Run only NOP tests
cargo test tests::cpu::test_nop

# Run arithmetic suites
cargo test tests::cpu::test_add_b
cargo test tests::cpu::test_move_w

# Run exception and trap tests
cargo test tests::cpu::test_illegal_linea
cargo test tests::cpu::test_trap

# Run all CPU single step tests
cargo test tests::cpu
```

---

## 6. Driving Future Development with AI Agents

This repository is configured for autonomous pair-programming with AI agents:

1. **Strict Guardrails:** All agent interactions must follow [AGENTS.md](file:///d:/Programowanie/Amiga/AGENTS.md) (no panics, wrapping arithmetic, big-endian conversions, zero allocations in hot paths).
2. **Domain Knowledge RAG:** Use the `amiga-rag` tool (`rag_search`) to query official Commodore Hardware Reference Manuals and PRMs in `Obsidian/Amiga/Reference/`.
3. **AST & Code Knowledge Graph:** Use `graphify` (`graphify query`, `graphify explain`) to inspect code relationships, types, and architectural hierarchies.
4. **Instruction Implementation:** Activate the `add-m68k-instruction` skill for a step-by-step checklist (decoding, CCK micro-steps, CCR flag updates, prefetch pipeline, and test harness integration).
5. **Test Failure Diagnosis:** Activate the `m68k-singlestep-test` skill to diagnose CCR mismatches ($X, N, Z, V, C$), prefetch queue offsets, and Address Error stack frames.
