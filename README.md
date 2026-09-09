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
│       │   ├── Agnus.md
│       │   ├── CIA.md
│       │   ├── Configuration.md
│       │   ├── CPU Motorola M68000.md
│       │   ├── CPU SingleStepTests.md
│       │   ├── CycleCounter.md
│       │   ├── Debugger.md
│       │   ├── Denise.md
│       │   ├── Floppy.md
│       │   ├── General Architecture.md
│       │   ├── GUI.md
│       │   ├── Joystick.md
│       │   ├── Keyboard.md
│       │   ├── Main loop A500.md
│       │   ├── MemoryBus.md
│       │   ├── Mouse.md
│       │   ├── Paula.md
│       │   └── SaveState.md
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
├── tools/                   <- Developer tools, offline table generators, and AI tooling
│   ├── BlebGenerator/       <- C# sinc BLEP table generator
│   ├── rag/                 <- Local RAG ingestion pipeline, CLI indexer (amiga_rag), and FastMCP server
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
- **[Moira 3.0](ref_src/Moira-3.0)** (`C++`) — [GitHub](https://github.com/dirkwhoffmann/Moira): Cycle-exact, micro-operation based MC68000 core by Dirk W. Hoffmann. Primary behavioral standard for bus cycle phases ($S_0-S_7$), instruction prefetch, and CCK clock synchronization.
- **[Musashi](ref_src/Musashi)** (`C`) — [GitHub](https://github.com/kstenerud/Musashi): Industry-standard portable 680x0 emulator core by Karl Stenerud. Reference for complete opcode decoding, CCR flags, and exception frames.
- **[m68k-rs](ref_src/m68k-rs-m68k-v0.11.6)** (`Rust`) — [GitHub](https://github.com/benletchford/m68k-rs): Pure Rust M68000–M68060 core featuring clean bus abstraction (`AddressBus`).
- **[EASy68K](ref_src/EASy68K-master)** — [GitHub](https://github.com/EASy68K/EASy68K) / [Web](http://www.easy68k.com): 68000 assembly editor, assembler, and simulator toolchain for authoring bare-metal test routines.

### 4.2 Hardware & FPGA Descriptions (HDL)
- **[fx68k](ref_src/fx68k)** (`Verilog`) — [GitHub](https://github.com/ijor/fx68k): Cycle-exact, microcode-level 68000 hardware description by Jorge Cwik (ijor). Ground-truth reference for silicon-level microcode, prefetch refills, and bus wait states.
- **[TG68K.C](ref_src/TG68K.C)** (`VHDL`) — [GitHub](https://github.com/TobiFlex/TG68K.C): Synthesizable 68000 FPGA core by Tobias Gubener.
- **[deniser](ref_src/deniser-1.0.0)** (`VHDL`) — [GitHub](https://github.com/endofexclusive/deniser): Drop-in FPGA replacement for the Amiga Denise video chip detailing planar-to-chunky conversion, sprite multiplexing, and HAM/EHB modes.
- **[Minimig-AGA_MiSTer](ref_src/Minimig-AGA_MiSTer)** (`Verilog`) — [GitHub](https://github.com/MiSTer-devel/Minimig-AGA_MiSTer): Full Amiga OCS/ECS/AGA hardware implementation on MiSTer FPGA. Reference for DMA bus slot arbitration across Agnus, Denise, and Paula.

### 4.3 Test Suites & Verification
- **[SingleStepTests-m68000](ref_src/SingleStepTests-m68000)** — [GitHub](https://github.com/SingleStepTests/m68000): 127 exhaustive per-instruction JSON validation test suites generated from MAME's microcoded core. Provides register/memory/prefetch starting conditions and expected cycle-by-cycle output states.
- **[SingleStepTests-680x0](ref_src/SingleStepTests-680x0)** — [GitHub](https://github.com/SingleStepTests/680x0): Tom Harte's single-step processor test vectors.
- **[amiga-stuff-testkit](ref_src/amiga-stuff-testkit-v1.21)** — [GitHub](https://github.com/keirf/amiga-test-kit): Keir Fraser's Amiga Test Kit (ADF boot disk) for testing CIA timers, floppy PLL decoding, memory autoconfig, and chipset interrupts.
- **[vAmigaTS](ref_src/vAmigaTS)** — [GitHub](https://github.com/dirkwhoffmann/vAmigaTS): Automated regression test suite consisting of ADF test disks and reference video renders for Copper lists, Blitter fills, and raster effects.

### 4.4 Reference System Emulators
- **[WinUAE](ref_src/WinUAE-6030)** — [GitHub](https://github.com/tonioni/WinUAE): Most comprehensive cycle-exact Amiga emulator by Toni Wilen. Ultimate reference for edge cases (floppy MFM sync, CIA TOD timers, Gary/Agnus bus contention).
- **[vAmiga](ref_src/vAmiga-4.5)** — [GitHub](https://github.com/dirkwhoffmann/vAmiga): Clean, object-oriented C++ A500/A1000/A2000 emulator by Dirk W. Hoffmann. Reference for decoupling Agnus, Denise, and Paula across a unified CCK grid.
- **[ScriptedAmigaEmulator](ref_src/ScriptedAmigaEmulator)** — [GitHub](https://github.com/naTmeg/ScriptedAmigaEmulator): High-level JavaScript Amiga emulator by Rupert Hausberger.
- **[MAME](ref_src/mame-mame0289)** — [GitHub](https://github.com/mamedev/mame): Reference implementations for shared peripheral chips (MOS 8520 CIA, M68000 CPU).

### 4.5 Visual Post-Processing
- **[RetroVisor.app](ref_src/RetroVisor.app)** — [GitHub](https://github.com/dirkwhoffmann/RetroVisor): CRT shader pipeline reference (scanlines, phosphor bloom, curvature, shadow mask) by Dirk W. Hoffmann.

---

## 5. Compiling & Running Tests

The emulator features a multi-tiered test architecture: standard subsystem unit tests, automated architecture rule validation, dual-suite M68000 single-step instruction verification (MAME + Tom Harte hardware vectors), Cartesian DMA contention stress tests, and a dedicated diagnostic CLI.

### 5.1 Standard Compilation & Subsystem Unit Tests
```powershell
# Build emulator core and tools
cargo build

# Typecheck for WebAssembly (WASM target)
cargo check --target wasm32-unknown-unknown

# Run all standard workspace unit and integration tests
cargo test

# Run tests for specific subsystem crates
cargo test -p m68000        # CPU core (addressing modes, micro-archetypes, CCK bus)
cargo test -p memory_bus    # Memory bus mapping, Gary logic, and autoconfig
cargo test -p rtc           # MSM6242B Real-Time Clock
cargo test -p debugger      # Interactive disassembly and breakpoint engine
```

### 5.2 Automated Architecture Rules Compliance
Enforces architectural rules and quality constraints defined in [AGENTS.md](AGENTS.md) (formatting, file size limits $\le 800$ lines, zero runtime panics/unwraps, path privacy, zero custom macros, and inlining rules):
```powershell
cargo test -p test_runner --test test_architecture_rules
```

### 5.3 M68000 SingleStepTests (Dual-Suite Hardware Verification)
Validates CPU instruction execution against two independent, complementary test suites:
1. **MAME SingleStepTests:** [`ref_src/SingleStepTests-m68000/v1/`](ref_src/SingleStepTests-m68000/v1/) (127 suites, includes Line-A, Line-F, STOP).
2. **Tom Harte SingleStepTests-680x0:** [`ref_src/SingleStepTests-680x0/68000/v1/`](ref_src/SingleStepTests-680x0/68000/v1/) (124 suites, ~1,000,000 test vectors, ground truth for `TAS` RMW cycles).

> [!NOTE]
> Ensure test JSON files are decoded before running (see [Section 3.2](#32-decode-singlesteptests-json-files)).

#### Default Sample Run (Fast Smoke Test)
By default, each instruction suite runs a sampled subset of 50 test cases (~5–6 seconds total):
```powershell
# Run sampled SingleStepTests across all implemented opcodes
cargo test -p test_runner --test test_singlestep

# Run tests for a specific instruction or group
cargo test -p test_runner --test test_singlestep test_nop
cargo test -p test_runner --test test_singlestep test_add_b
cargo test -p test_runner --test test_singlestep test_move_w
```

#### Full Exhaustive Verification (`SINGLESTEP_FULL`)
Setting `SINGLESTEP_FULL=1` (or `true`) disables sampling limits and executes **100% of all ~300,000 test vectors** across all 127 suites from both MAME and Tom Harte in parallel (typically completes in 12–15 seconds).

- **PowerShell (Windows):**
  ```powershell
  # Full exhaustive run across all implemented opcodes (~300,000 vectors)
  $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep

  # Full exhaustive run for a single instruction suite
  $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep -- test_add_b
  ```

- **Bash / Linux / macOS / WSL:**
  ```bash
  # Full exhaustive run across all implemented opcodes
  SINGLESTEP_FULL=1 cargo test -p test_runner --test test_singlestep

  # Full exhaustive run for a single instruction suite
  SINGLESTEP_FULL=1 cargo test -p test_runner --test test_singlestep -- test_add_b
  ```

#### Custom Sample Limit (`SINGLESTEP_LIMIT`)
To evaluate an arbitrary sample size (e.g. 200 or 500 test cases per suite):
- **PowerShell:**
  ```powershell
  $env:SINGLESTEP_LIMIT = "500"; cargo test -p test_runner --test test_singlestep
  ```
- **Bash:**
  ```bash
  SINGLESTEP_LIMIT=500 cargo test -p test_runner --test test_singlestep
  ```

### 5.4 Cartesian DMA Contention Verification
Validates cycle-exact M68000 micro-stepping and wait-state handling under Agnus DMA bus contention across the full combinatorial Cartesian product:
- **Address Permutations ($2^k$):** Sweeps all role assignments of memory cells touched by the instruction (`ChipRam` vs `FastRam`).
- **DMA Schedule Permutations ($2^M$):** Sweeps every bit pattern of stalled vs free CCK slots across the execution window.
- **Asserted Invariants:**
  1. *Cycle Invariance:* $C = C_0 + 2 \times \text{wait\_states}$
  2. *Fast RAM Immunity:* $C = C_0$ without wait states when only Fast RAM is accessed.
  3. *State Invariance:* CPU registers and RAM are 100% bit-identical to the uncontended golden run.

```powershell
# Run full Cartesian DMA contention test suite
cargo test -p test_runner --test test_dma_cartesian

# Run Cartesian tests for specific category
cargo test -p test_runner --test test_dma_cartesian test_dma_cartesian_system_and_traps
```

### 5.5 CLI Test Diagnostics, Coverage & Regression Tracker
The `test_runner` crate includes a standalone CLI tool for inspecting coverage matrices, viewing live failure diagnostics, and detecting regressions:

```powershell
# Display global pass/fail matrix and coverage summary across all opcodes
cargo run -p test_runner -- --summary

# Detect regressions and fixed tests compared to previous run (via .test_results/)
cargo run -p test_runner -- --diff

# Execute a single opcode suite directly with live diagnostic failure output
cargo run -p test_runner -- --suite ADD.b
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## 6. Driving Future Development with AI Agents

This repository is configured for autonomous pair-programming with AI agents:

1. **Strict Guardrails:** All agent interactions must follow [AGENTS.md](AGENTS.md) (no panics, wrapping arithmetic, big-endian conversions, zero allocations in hot paths).
2. **Domain Knowledge RAG:** Use the `amiga-rag` tool (`rag_search`) to query official Commodore Hardware Reference Manuals and PRMs in `Obsidian/Amiga/Reference/`. The RAG pipeline and FastMCP server reside in [`tools/rag/`](tools/rag/), backed by the local Qdrant vector database (`amiga` collection, incremental cache configured via `RAG_CACHE_FILE` in `.env`).
3. **AST & Code Knowledge Graph:** Use `graphify` (`graphify query`, `graphify explain`) to inspect code relationships, types, and architectural hierarchies.
4. **Instruction Implementation:** Activate the `add-m68k-instruction` skill for a step-by-step checklist (decoding, CCK micro-steps, CCR flag updates, prefetch pipeline, and test harness integration).
5. **Test Failure Diagnosis:** Activate the `m68k-singlestep-test` skill to diagnose CCR mismatches ($X, N, Z, V, C$), prefetch queue offsets, and Address Error stack frames.
