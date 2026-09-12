# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance **Commodore Amiga 500** (OCS) emulator written in Rust, engineered for native desktop platforms and WebAssembly.

Focused on the authentic floppy disk gaming experience (`DF0:`, `.adf`) without hard drives or bulky expansion clutter.

---

## Hardware Configurations Supported

- **Basic A500:** 512 KB Chip RAM (early OCS revision 5 motherboard).
- **Classic A500:** 512 KB Chip RAM + 512 KB Trapdoor Slow RAM at `$C00000` (the standard European 1 MB gaming configuration).
- **Expanded A500:** 4 MB Fast RAM.
- **Target Platforms:** Native desktop executable (Windows, Linux, macOS) and WebAssembly for direct browser play.

---

## 1. For Players (Quick Start)

> Prebuilt desktop executables (`.exe`) and WebAssembly browser bundles will be published under **Releases**.
> To run directly from source (requires the [Rust toolchain](https://rustup.rs)):

### Launching the Emulator

Run in **Clean Standalone Game Mode** (clean 50 Hz PAL display without toolbars or dock windows):
```powershell
cargo run -p gui -- --game
```

Or open the **WebAssembly Browser Canvas** (via Trunk):
```powershell
trunk serve crates/gui/index.html --open
```

### Screen Modes & Clean Toggle (`F12`)
- Press **`F12`** at any time to toggle between **Clean Game Mode** and the **Developer Studio / Debugger**.
- Switching into Developer Studio pauses emulation immediately, allowing instant inspection of registers, custom chip states, and memory.

### Loading Games
- **Floppy Disks (`.adf`):** Drag and drop any `.adf` image directly onto the emulator window to insert it into `DF0:` and start playing.

### Player Keybindings & Controls

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`F5` / `Space`** | **Run / Pause** | Toggles continuous emulation. |
| **`Ctrl + R`** | **Reset / Restart** | Performs cold reset, restoring initial hardware state and rebooting. |
| **`F12`** | **Toggle Screen Mode** | Switches between Clean Game Display and the Developer Studio. |
| **Drag & Drop** | **Insert Floppy (`DF0:`)** | Drag any `.adf` disk image directly onto the window. |

*(For advanced developer controls, instruction stepping, live memory editing, and time-travel rewind, see the [Developer Studio & Debugger Guide](docs/debugger.md).)*

---

## 2. For Developers (Build & Bootstrap)

### Zero-Setup Build & Execution
A freshly cloned repository is **100% self-contained for compilation and execution** using the standard stable Rust toolchain ([rustup.rs](https://rustup.rs)). No bootstrapping, external downloads, or database services are required to build and run the emulator:

```powershell
# Build entire workspace (debug profile)
cargo build

# Build optimized native release binary for GUI
cargo build --release -p gui

# Run core subsystem unit tests
cargo test -p m68000
cargo test -p memory_bus
```

### Optional Bootstrapping (`tools/bootstrap.ps1`)

Bootstrapping is **strictly optional** and only needed for specialized development tasks:

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Verification Testbed** | `-Test` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Provisions Tom Harte physical silicon test vectors (auto-decompressing `.gz`/`.zip` archives in `ref_src/SingleStepTests-680x0/`, 124 suites), verifies vAmiga/vAmigaTS reference suites, and diagnostic disks |
| **Code Knowledge Graph** | `-Graph` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | AST-level code knowledge graph (`graphify-out/`), mapping crates, structs, functions, and cross-module relationships |
| **Documentation & RAG** | `-Doc` | AI agent pair-programming, hardware research, architecture design | Local Qdrant vector database (`http://localhost:6333`), indexing Commodore HRM, 68000 PRMs, Guru book, and design specs |
| **Full Setup** | `-All` | Complete initial development setup | Provisions all components (hardware test vectors -> Graphify AST -> RAG documentation) |

```powershell
# Hardware test vectors verification setup:
.\tools\bootstrap.ps1 -Test

# Code AST knowledge graph setup:
.\tools\bootstrap.ps1 -Graph

# Documentation & AI pair-programming setup:
.\tools\bootstrap.ps1 -Doc

# Complete setup (tests -> Graphify AST -> RAG docs):
.\tools\bootstrap.ps1 -All
```

---

## 3. Documentation Cheat Sheet & Technical Index

### Repository Architecture & Verification Guides (`docs/`)

- [**Core Architecture & Hardware Execution Model**](docs/architecture.md): Color Clock phases (CCK1/CCK2), Gary bus arbitration, Agnus DMA contention, circuit simulation, Big-Endian invariance, and decoupled ownership.
- [**Developer Studio & Time-Travel Debugger**](docs/debugger.md): Developer controls, time-travel rewind ring buffer, live register inspection, memory hex grid editing, disassembly patching, and breakpoints.
- [**Test Suite & Verification Framework**](docs/testing.md): Physical hardware single-step test options (`SINGLESTEP_FULL`, `SINGLESTEP_LIMIT`), Cartesian DMA contention math ($2^k \times 2^M$), and CLI regression diagnostics.
- [**AI Agent Engineering & Pair-Programming Guide**](docs/ai_agents.md): Autonomous AI agent pairing guidelines, rules adherence, RAG knowledge base, Graphify AST, and specialized skills.
- [**Parallel Development with Git Worktrees**](docs/worktrees.md): Multi-branch parallel workflows and isolated build contexts.

### Subsystem Design Specifications (`Obsidian/Amiga/Design/`)

| Category | Component Specifications |
| :--- | :--- |
| **System & Bus** | [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) • [Main Loop (A500)](Obsidian/Amiga/Design/Main%20loop%20A500.md) • [Memory Bus & Gary](Obsidian/Amiga/Design/MemoryBus.md) • [Cycle Counter](Obsidian/Amiga/Design/CycleCounter.md) • [Save States](Obsidian/Amiga/Design/SaveState.md) • [Configuration](Obsidian/Amiga/Design/Configuration.md) |
| **CPU (M68000)** | [CPU Motorola M68000](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md) • [Micro-Step State Machine](Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md) • [CPU SingleStepTests](Obsidian/Amiga/Design/CPU%20SingleStepTests.md) • [CPU Instruction Benchmarking](Obsidian/Amiga/Design/CPU%20Instruction%20Benchmarking.md) |
| **Custom Chipset** | [Agnus (Copper & Blitter)](Obsidian/Amiga/Design/Agnus.md) • [Denise (Video & Sprites)](Obsidian/Amiga/Design/Denise.md) • [Paula (Audio & Floppy DMA)](Obsidian/Amiga/Design/Paula.md) • [CIA (Timers & Serial Ports)](Obsidian/Amiga/Design/CIA.md) • [Floppy Disk Controller](Obsidian/Amiga/Design/Floppy.md) |
| **Peripherals & I/O** | [Keyboard Controller](Obsidian/Amiga/Design/Keyboard.md) • [Mouse Controller](Obsidian/Amiga/Design/Mouse.md) • [Joystick Controller](Obsidian/Amiga/Design/Joystick.md) • [Real-Time Clock (RTC)](Obsidian/Amiga/Design/RTC.md) |
| **Frontend & UI** | [GUI Specification](Obsidian/Amiga/Design/GUI%20Specification.md) • [GUI Architecture](Obsidian/Amiga/Design/GUI.md) • [Debugger Engine](Obsidian/Amiga/Design/Debugger.md) • [egui Guidelines](Obsidian/Amiga/Design/egui%20Guidelines.md) • [Rust Coding Guidelines](Obsidian/Amiga/Design/Rust%20Guidelines.md) |
