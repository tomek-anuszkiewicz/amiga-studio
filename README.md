# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance **Commodore Amiga 500** (OCS) emulator written in Rust, engineered for native desktop platforms and WebAssembly.

Focused on the authentic floppy disk gaming experience (`DF0:`, `.adf`) without hard drives or bulky expansion clutter.

---

## Hardware Configurations Supported

- **Basic A500:** 512 KB Chip RAM (early stock model without memory expansion).
- **Classic A500 (Recommended):** 512 KB Chip RAM + 512 KB Slow RAM (1 MB total) — the default standard configuration for the vast majority of Amiga games.
- **Expanded A500:** 1 MB base RAM + 4 MB Fast RAM — power-user and professional configuration for productivity, Workbench multitasking, and demanding demos.
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
```

### Optional Bootstrapping (`tools/bootstrap/bootstrap.ps1`)

Bootstrapping is **strictly optional** and only needed for specialized development tasks:

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Verification Testbed** | `-Sources` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Provisions Tom Harte physical silicon test vectors (auto-decompressing `.gz`/`.zip` archives in `ref_src/SingleStepTests-680x0/`, 124 suites), verifies vAmiga/vAmigaTS reference suites, and diagnostic disks |
| **Code Knowledge Graph** | `-Graphify` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | AST-level code knowledge graph (`graphify-out/`), mapping crates, structs, functions, and cross-module relationships |
| **Documentation & RAG** | `-Rag` | AI agent pair-programming, hardware research, architecture design | Local Qdrant vector database (`http://localhost:6333`), indexing Commodore HRM, 68000 PRMs, technical specs, and design specs |
| **Reference Scans** | `-Documentation` | External reference scans and manual archives | Provisions raw reference manuals and PDF scans into `temp/` |
| **Full Setup** | `-All` | Complete initial development setup | Provisions all primary components (sources -> Graphify AST -> RAG documentation) |

```powershell
# Hardware test vectors verification setup:
.\tools\bootstrap\bootstrap.ps1 -Sources

# Code AST knowledge graph setup:
.\tools\bootstrap\bootstrap.ps1 -Graphify

# Documentation & AI pair-programming setup:
.\tools\bootstrap\bootstrap.ps1 -Rag

# External reference manuals and scans:
.\tools\bootstrap\bootstrap.ps1 -Documentation

# Complete setup (sources -> Graphify AST -> RAG docs):
.\tools\bootstrap\bootstrap.ps1 -All
```

---

## 3. Documentation Cheat Sheet & Technical Index

### Repository Architecture & Verification Guides (`docs/`)

- [**Core Architecture & Hardware Execution Model**](docs/architecture.md): Color Clock phases (CCK1/CCK2), Gary bus arbitration, Agnus DMA contention, circuit simulation, Big-Endian invariance, and decoupled ownership.
- [**Developer Studio & Time-Travel Debugger**](docs/debugger.md): Developer controls, time-travel rewind ring buffer, live register inspection, memory hex grid editing, disassembly patching, and breakpoints.
- [**Test Suite & Verification Framework**](docs/testing.md): Physical hardware single-step test options (`SINGLESTEP_FULL`, `SINGLESTEP_LIMIT`), Cartesian DMA contention math ($2^k \times 2^M$), and CLI regression diagnostics.
- [**How This Emulator Was Written: Pair-Programming with an AI Agent**](docs/how_this_emulator_was_written.md): Engineering methodology, zero-code human steering, architectural sparring, minimal frame prototyping, and the evolutionary harness.
- [**AI Agent Engineering & Pair-Programming Guide**](docs/ai_agents.md): Autonomous AI agent pairing guidelines, rules adherence, RAG knowledge base, Graphify AST, and specialized skills.
- [**Technical Reference Library & AI RAG Guide**](docs/reference.md): Ingested hardware reference manuals, automated reference bootstrapper, raw document conversion toolchain, and local Qdrant RAG search.
- [**Git Worktree Lifecycle & Asset Linking**](.agents/skills/git-worktree/SKILL.md): Isolated Cargo build caches, sibling directory placement, automated NTFS junction asset linking (`tools/git/worktree.ps1`), and clean teardown.

### Subsystem Design Specifications (`Obsidian/Amiga/Design/`)

| Category | Component Specifications |
| :--- | :--- |
| **System & Bus** | [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) • [Main Loop (A500)](Obsidian/Amiga/Design/Main%20loop%20A500.md) • [Memory Bus & Gary](Obsidian/Amiga/Design/MemoryBus.md) • [Save States](Obsidian/Amiga/Design/SaveState.md) • [Configuration](Obsidian/Amiga/Design/Configuration.md) |
| **CPU (M68000)** | [CPU Motorola M68000](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md) • [Micro-Step State Machine](Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md) • [CPU SingleStepTests](Obsidian/Amiga/Design/CPU%20SingleStepTests.md) • [CPU Instruction Benchmarking](Obsidian/Amiga/Design/CPU%20Instruction%20Benchmarking.md) |
| **Custom Chipset** | [Agnus (Copper & Blitter)](Obsidian/Amiga/Design/Agnus.md) • [Denise (Video & Sprites)](Obsidian/Amiga/Design/Denise.md) • [Paula (Audio & Floppy DMA)](Obsidian/Amiga/Design/Paula.md) • [CIA (Timers & Serial Ports)](Obsidian/Amiga/Design/CIA.md) • [Floppy Disk Controller](Obsidian/Amiga/Design/Floppy.md) |
| **Peripherals & I/O** | [Keyboard Controller](Obsidian/Amiga/Design/Keyboard.md) • [Mouse Controller](Obsidian/Amiga/Design/Mouse.md) • [Joystick Controller](Obsidian/Amiga/Design/Joystick.md) • [Real-Time Clock (RTC)](Obsidian/Amiga/Design/RTC.md) |
| **Frontend & UI** | [GUI Specification](Obsidian/Amiga/Design/GUI%20Specification.md) • [GUI Architecture](Obsidian/Amiga/Design/GUI.md) • [Debugger Engine](Obsidian/Amiga/Design/Debugger.md) • [egui Guidelines](Obsidian/Amiga/Design/egui%20Guidelines.md) • [Rust Coding Guidelines](Obsidian/Amiga/Design/Rust%20Guidelines.md) |
