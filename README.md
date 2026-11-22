# Amiga 500 Cycle-Exact Emulator in Rust

A cycle-exact, high-performance emulator for the **Commodore Amiga 500** (OCS).
It is written in Rust and targets native desktops and WebAssembly.

The project focuses on playing games from floppy disk (`DF0:`, `.adf`). Hard
drives and larger expansions are outside its scope. The project grew through
human–AI pair programming and discussion of architectural choices. The
engineering story follows how the human guided development without writing
code and how the test harness evolved.
[**How This Emulator Was Written**](docs/how_this_emulator_was_written.md)

---

## Hardware Configurations Supported

- **Basic A500** has 512 KB Chip RAM, like the early stock model without memory
  expansion.
- **Classic A500 (Recommended)** has 512 KB Chip RAM + 512 KB Slow RAM
  (1 MB total). This is the default configuration and suits the vast
  majority of Amiga games.
- **Expanded A500** has 1 MB base RAM + 4 MB Fast RAM. It represents a
  configuration for power users and professionals running productivity
  software, multitasking in Workbench or running demanding demos.
- **Target Platforms** are native desktop executables for Windows, Linux and
  macOS, and WebAssembly for play directly in the browser.

---

## Current Runnable Demo

The emulator is under active development. A playable Amiga system is still
pending. The desktop application opens the game library, and the browser
build opens manual setup. Both screens let you open the
**Developer Studio & M68000 Debugger** to run standalone processor programs.
[Game library and manual setup](docs/game-library.md)

The roadmap tracks the remaining implementation work. [Development roadmap](ROADMAP.md)

To run an example program, launch the emulator, choose **Debug**, then select
**File → Load Binary** and open one of the example `.bin` files. These
programs are designed to load at address `$001000`.
[`examples/m68k/`](examples/m68k)

### Native Desktop Application

Run the Developer Studio in a standalone desktop window.

```powershell
cargo run --release -p gui
```

### Web Browser

Use Trunk to run the WebAssembly build in a browser. [Trunk](https://trunkrs.dev)

```powershell
cargo install --locked trunk
rustup toolchain install nightly --component rust-src --target wasm32-unknown-unknown
./tools/web.ps1 serve
```

Open the address printed by Trunk. The browser build uses shared Wasm memory
and a dedicated worker that runs the machine. The web launcher rebuilds the
Rust standard library with atomic support and serves the required
cross-origin isolation headers. `./tools/web.ps1 build` writes an
independent build under `.agent/tmp/web-dist`. [Web launcher](tools/web.ps1)

The debugger guide covers all controls and gives step-by-step instructions
for shortcuts and debugging workflows.
[**Developer Studio & Debugger Guide**](docs/debugger.md)

The developer guide provides an overview of the emulator, verification and
ways to work with the project.
[**Developer Guide & Technical Index**](docs/developers.md)
