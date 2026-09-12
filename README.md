# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance **Commodore Amiga 500** (OCS) emulator written in Rust, engineered for native desktop platforms and WebAssembly (`wasm32-unknown-unknown`).

Focused on the authentic floppy disk gaming experience (`DF0:`, `.adf`, and direct binary injection) without hard drives or bulky expansion clutter.

---

## Hardware Configurations Supported

- **Basic A500:** 512 KB Chip RAM (early OCS revision 5 motherboard).
- **Classic A500:** 512 KB Chip RAM + 512 KB Trapdoor Slow RAM at `$C00000` (the standard European 1 MB gaming configuration).
- **Expanded A500:** Optional Auto-Config Fast RAM ($200000..$9FFFFF) with non-contended zero wait-state execution.
- **Target Platforms:** Native desktop executable (Windows, Linux, macOS) and WebAssembly for direct browser play.

---

## 1. For Players (Quick Start)

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

### Loading Games & Software
- **Floppy Disks (`.adf`):** Drag and drop any `.adf` image onto the emulator window, or select floppy drive `DF0:` from the top menu.
- **Machine Code Binaries:** Drag and drop compiled binaries (`.bin`, `.rom`) onto the window, or launch directly at a target RAM address:
  ```powershell
  cargo run -p gui -- --load path/to/game.bin --addr 001000
  ```

### Keybindings & Controls

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`F12`** | **Toggle GUI / Game Mode** | Switches between Developer Studio and Clean Screen. Pauses emulation on entry. |
| **`F5` / `Space`** | **Run / Pause** | Toggles continuous free-running emulation. |
| **`F10`** | **Step Instruction** | Executes exactly 1 M68000 instruction, recording trace and history. |
| **`Shift + F10`** | **Step Backward (Rewind)** | Restores previous execution snapshot from the high-capacity temporal buffer. |
| **`F11`** | **Step CCK** | Steps 1 Color Clock phase (2 CPU clocks, CCK1 / CCK2). |
| **`Alt + T`** | **Toggle Recording** | Activates or pauses temporal execution history recording. |
| **`Ctrl + O`** | **Load Binary** | Opens file dialog to inject compiled machine code at any arbitrary RAM address. |
| **`Ctrl + R`** | **Reset Cold** | Restores initial hardware state and reloads CPU vectors. |
| **Drag & Drop** | **Quick File Injection** | Drag any `.adf`, `.bin`, or `.rom` directly onto the window. |

### Developer Studio & Time-Travel Debugger
When the Developer Studio is active (`F12` or running `cargo run -p gui` without `--game`), the interface provides:
- **Time-Travel Rewind ($\ge 1.0\text{s}$ PAL History):** High-capacity ring buffer recording cycle-exact CPU states with zero runtime heap allocations. Scrub backwards and forwards by instruction, frame (~70,824 CCK), or specific CCK cycle timestamp.
- **Live Register Inspector:** Interactive view of all 68000 registers ($D_0-D_7, A_0-A_7, PC, SR, USP, SSP$) with diff highlighting (cyan glow on changed values). Click any register to edit in hex.
- **Memory Hex Grid:** Live memory viewer with inline byte editing (Tab/Enter advances, Esc cancels). Mutated bytes glow in amber/cyan.
- **Disassembly In-Place Editing:** Click the pencil icon (`✏`) to edit instructions using standard assembly (e.g. `NOP`, `MOVE.W D0, D1`) with byte-size safety verification.
- **Breakpoints & Watchpoints:** Execution breakpoints on Program Counter and memory watchpoints on `Read`, `Write`, or `Any` access across arbitrary address blocks.

---

## 2. For Developers (Build & Bootstrap)

### Zero-Setup Build & Execution
A freshly cloned repository is **100% self-contained for compilation and execution**. No bootstrapping, external downloads, or database services are required to build and run the emulator:

```powershell
# Build entire workspace (debug profile)
cargo build

# Build optimized native release binary for GUI
cargo build --release -p gui

# Run core subsystem unit tests
cargo test -p m68000
cargo test -p memory_bus
```

### Optional Two-Tier Bootstrapping (`tools/bootstrap.ps1`)

Bootstrapping is **strictly optional** and only needed for specialized development tasks:

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Knowledge & AI Docs** | `-Doc` | AI agent pair-programming, hardware research, architecture design | Local Qdrant vector database (`http://localhost:6333`), indexing Commodore HRM, 68000 PRMs, Guru book, and design specs |
| **Verification Testbed** | `-Test` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Prepares Tom Harte physical silicon test vectors (`ref_src/SingleStepTests-680x0/`, 124 suites, ~1,000,000 vectors) and regression test disks |
| **Full Setup** | `-All` | Complete initial development setup | Provisions both documentation knowledge bases and verification test vectors |

```powershell
# Documentation & AI pair-programming setup:
.\tools\bootstrap.ps1 -Doc

# Hardware test vectors verification setup:
.\tools\bootstrap.ps1 -Test

# Complete setup:
.\tools\bootstrap.ps1 -All
```

---

## 3. Documentation Cheat Sheet & Technical Index

### Repository Architecture & Verification Guides (`docs/`)

- [**Core Architecture & Hardware Execution Model**](docs/architecture.md): Color Clock phases (CCK1/CCK2), Gary bus arbitration, Agnus DMA contention, circuit simulation, Big-Endian invariance, and decoupled ownership.
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
