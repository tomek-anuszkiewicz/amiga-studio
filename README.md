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

### Developer Studio & In-Game Debugging

Press **`F12`** at any time to pause emulation and switch into the Developer Studio:
- **Live State Inspection:** Inspect and live-edit M68000 registers, custom chip states, and Chip RAM in real time.
- **Time-Travel Rewind:** Step backward through recent execution history using the rolling cycle trace buffer.
- **Disassembly & Breakpoints:** Set PC breakpoints, memory watchpoints, and patch machine code instructions on the fly.

For the complete controls reference, step-by-step shortcuts, and debugging workflows, see:
👉 [**Developer Studio & Debugger Guide**](docs/debugger.md)

---

## 2. For Developers

For compilation instructions, bootstrapping options, test suites, and technical reference guides:
👉 [**Developer Guide & Technical Index**](docs/developers.md)

---

## 3. Documentation Cheat Sheet & Technical Index

Architectural deep dives, research papers, and AI pair-programming specifications:

- [**Core Architecture & Hardware Execution Model**](docs/architecture.md): Color Clock phases (CCK1/CCK2), Gary bus arbitration, Agnus DMA contention, circuit simulation, Big-Endian invariance, and decoupled ownership.
- [**How This Emulator Was Written: Pair-Programming with an AI Agent**](docs/how_this_emulator_was_written.md): Engineering methodology, zero-code human steering, architectural sparring, minimal frame prototyping, and the evolutionary harness.
- [**AI Agent Engineering & Pair-Programming Guide**](docs/ai_agents.md): Autonomous AI agent pairing guidelines, rules adherence, RAG knowledge base, Graphify AST, and specialized skills.
