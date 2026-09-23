# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance **Commodore Amiga 500** (OCS) emulator written in Rust, engineered for native desktop platforms and WebAssembly.

Focused on the authentic floppy disk gaming experience (`DF0:`, `.adf`) without hard drives or bulky expansion clutter. Built collaboratively through human-AI pair-programming and architectural sparring — explore [**How This Emulator Was Written**](docs/how_this_emulator_was_written.md) for the full engineering story, zero-code human steering methodology, and evolutionary test harness design.

---

## Hardware Configurations Supported

- **Basic A500:** 512 KB Chip RAM (early stock model without memory expansion).
- **Classic A500 (Recommended):** 512 KB Chip RAM + 512 KB Slow RAM (1 MB total) — the default standard configuration for the vast majority of Amiga games.
- **Expanded A500:** 1 MB base RAM + 4 MB Fast RAM — power-user and professional configuration for productivity, Workbench multitasking, and demanding demos.
- **Target Platforms:** Native desktop executable (Windows, Linux, macOS) and WebAssembly for direct browser play.

---

## Current Runnable Demo

The emulator is still under active development and does not yet provide a playable Amiga system. The current interactive entry point is the **Developer Studio & M68000 Debugger**, where you can load and inspect small standalone processor programs.

Example programs are available under [`tests/bin/`](tests/bin). After launching the Developer Studio, select **File → Load Binary** and open one of the `.bin` files from that directory. The examples are designed to load at address `$001000`.

### Native Desktop Application

Run the Developer Studio as a standalone desktop window:

```powershell
cargo run --release -p gui
```

### Web Browser

Run the WebAssembly build in a browser using [Trunk](https://trunkrs.dev):

```powershell
cargo install --locked trunk
trunk serve crates/gui/index.html --open
```

For the complete controls reference, step-by-step shortcuts, and debugging workflows, see:
👉 [**Developer Studio & Debugger Guide**](docs/debugger.md)

For compilation instructions, bootstrapping options, test suites, and technical reference guides:
👉 [**Developer Guide & Technical Index**](docs/developers.md)
