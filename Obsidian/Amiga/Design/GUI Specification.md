# Amiga 500 Developer GUI & Debugger Layout Specification

> [!NOTE]
> This document is the concrete operational and visual execution specification for the Amiga 500 Developer GUI (`crates/desktop_gui`).
> For high-level frontend architecture, decoupled core interfaces, CRT shaders, and audio sinks, see [GUI.md](GUI.md).

---

## 1. Visual Layout & 1:1 Source Code Directory Mapping

The physical arrangement on screen strictly maps 1:1 to the Rust source code hierarchy under `crates/desktop_gui/src/`:

```
+---------------------------------------------------------------------------------------------------+
| Top Menu Bar (layout/top_menu_bar.rs)                                                             |
+-----------------------------------+-----------------------------------+---------------------------+
| Left Dock: Execution & CPU        | Center: Main Viewport             | Right Dock: Memory        |
|                                   |                                   |                           |
| 1. Registers Panel                | 1. Amiga Screen (320x256 4:3 CRT) | 1. Memory Hex Editor      |
|    (layout/left_dock/registers.rs)|    (layout/main_viewport/         |    (layout/right_dock/    |
|                                   |     amiga_screen.rs)              |     memory_hex.rs)        |
| 2. Microcode Inspector            |                                   |                           |
|    (layout/left_dock/microcode.rs)|                                   | 2. Memory Search Bar      |
|                                   | 2. Temporal Rewind Scrubber       |    (layout/right_dock/    |
| 3. Disassembly View               |    (layout/main_viewport/         |     memory_search.rs)     |
|    (layout/left_dock/             |     temporal_bar.rs)              |                           |
|     disassembly.rs)               |                                   | 3. Trace History Log      |
|                                   |                                   |    (layout/right_dock/    |
|                                   |                                   |     trace_log.rs)         |
+-----------------------------------+-----------------------------------+---------------------------+
```

### Source File Tree:
```
crates/desktop_gui/
├── Cargo.toml                  <- Dependencies (eframe, egui, rfd, m68000, memory_bus, debugger, config)
├── index.html                  <- Web runner template for WebAssembly browser canvas
├── Trunk.toml                  <- Trunk web bundler configuration
└── src/
    ├── lib.rs                  <- WASM web entrypoint (eframe::WebRunner)
    ├── main.rs                 <- Desktop native entrypoint (eframe::run_native)
    ├── app.rs                  <- Central EmulatorApp layout orchestrator
    ├── theme.rs                <- Dark/Light mode detection & classic Amiga palettes
    ├── loader.rs               <- Raw binary file injection into RAM
    ├── temporal.rs             <- Time-travel rewind history buffer
    └── layout/
        ├── top_menu_bar.rs     <- [TOP] Controls, file dialogs, execution state, theme
        ├── main_viewport/
        │   ├── amiga_screen.rs <- [CENTER] 4:3 Game / Workbench CRT display viewport
        │   └── temporal_bar.rs <- [CENTER-BOTTOM] Time-travel scrubber & rewind slider
        ├── left_dock/
        │   ├── registers.rs    <- [LEFT-TOP] Live D0-D7, A0-A7, PC, SR, CCR flags
        │   ├── microcode.rs    <- [LEFT-MID] Microcode state machine, ALU, staging registers
        │   └── disassembly.rs  <- [LEFT-BOT] M68000 instruction stream & breakpoints
        └── right_dock/
            ├── memory_hex.rs   <- [RIGHT-TOP] Hex + ASCII editor & chunk jump buttons
            ├── memory_search.rs<- [RIGHT-MID] Hex/ASCII pattern search bar
            └── trace_log.rs    <- [RIGHT-BOT] 1024-entry execution trace log table
```

---

## 2. Interaction & Execution Paradigm: Synchronous Direct-State Pull

The GUI and the emulator core interact through a strictly **synchronous, pull-based model**:
1. **Direct References:** The top-level `EmulatorApp` struct owns `Cpu`, `MemoryBus`, and `Debugger`. During every `egui::App::update` pass, panels read directly from `&self.cpu.state`, `&self.bus`, and `&self.debugger`.
2. **Zero Asynchrony & Zero Callbacks:** There are no background threads for the core in the baseline, no `mpsc` message queues, and no observer pattern notifications. The emulator core never "pushes" change events to the GUI.
3. **Synchronous Mutation on Interaction:**
   - When the user steps (F10), `cpu.step_instruction(&mut bus)` executes immediately in the frame loop, appends to `temporal`, and the UI redraws immediately with the updated state.
   - When the user edits memory, `bus.write_byte_debug(addr, val)` updates the physical RAM buffer instantly.
   - When the user edits a register or double-clicks an address to set $PC$, `cpu.set_pc_and_prime_prefetch(addr, &mut bus)` updates CPU state immediately.

---

## 3. Panel-by-Panel Specification

### 3.1 Top Menu Bar (`layout/top_menu_bar.rs`)
- **File Actions:**
  - `Load Binary...` (`Ctrl+O`): Opens file dialog (`rfd`). Prompts for Target Address (defaults to `$001000`) and a checkbox `Auto-set PC & prime prefetch`. Injects binary slice directly into RAM.
  - `Reset A500` (`Ctrl+R`): Cold-resets CPU, clears RAM (or resets to initial vector values), and clears the temporal trace buffer.
- **Execution Controls:**
  - `▶ Run` / `⏸ Pause` (`F5` or `Space`): Toggles continuous emulation. While running, executes up to a bounded batch of cycles per frame without hanging the UI.
  - `⏭ Step Instruction` (`F10`): Executes exactly one completed M68000 instruction, records temporal frame, and halts.
  - `⏯ Step CCK` (`F11`): Advances exactly 1 Color Clock phase (2 CPU clocks), advancing microcode state.
  - `⏮ Step Backward (Rewind)` (`Shift+F10`): Pops the previous execution frame from the temporal history buffer, restoring exact CPU registers and memory deltas.
- **Status Badges:**
  - Emulation state: `[RUNNING - Green]` / `[PAUSED - Yellow]` / `[HALTED - Red]` / `[STOPPED - Blue]`.
  - Machine Metrics: Current CCK count, simulated MHz, instructions executed.
- **Preferences:**
  - Theme Selector: `Auto (System)`, `Dark (Default)`, `Light`, `Classic Workbench`.
  - Zoom Selector: `100%`, `125%`, `150%`, `200%`.

---

### 3.2 Main Viewport: Amiga Screen (`layout/main_viewport/amiga_screen.rs`)
- **Display Container:**
  - Strict 4:3 aspect ratio framing centered in the panel.
  - Canvas resolution: $320 \times 256$ (PAL baseline) / $320 \times 200$ (NTSC).
- **Initial Standby State:**
  - Renders a retro dark bezel with an authentic CRT phosphor test pattern / standby message: `"Amiga 500 Video Engine Standby — No Video Signal"`.
  - Ready to receive native Denise/Agnus ARGB8888 frame buffer blits in Phase 1.
- **Scaling:** Crisp integer prescaling (1x, 2x, 3x) based on available panel geometry.

---

### 3.3 Main Viewport: Temporal Rewind Scrubber (`layout/main_viewport/temporal_bar.rs`)
- Located immediately below the Amiga display viewport.
- **Timeline Slider:**
  - Displays a continuous scrubber representing the 1024–4096 step circular history buffer.
  - Left end = oldest recorded state; Right end = live head.
  - Dragging the slider dynamically scrubs through past execution states, updating registers, memory, and PC in real time.
- **Scrubber Buttons:**
  - `|◀ Rewind to Start`: Jumps to the earliest captured snapshot.
  - `◀ Step Back` (`Shift+F10`): Restores the immediately preceding step.
  - `Step Forward ▶` (`F10`): Advances one step toward live head (or executes a new step if at head).
  - `▶| Jump to Live Head`: Returns immediately to real-time execution.

---

### 3.4 Left Dock: CPU Registers (`layout/left_dock/registers.rs`)
- **Data Registers ($D_0 - D_7$):**
  - Displays each register in hexadecimal (`$00000000`) and signed/unsigned decimal (`0`).
- **Address Registers ($A_0 - A_7$):**
  - Displays $A_0 - A_6$ and active Stack Pointer $A_7$.
  - Explicitly displays inactive stack pointer: $USP$ (User Stack Pointer) vs $SSP$ (Supervisor Stack Pointer).
- **Program Counter & Status:**
  - $PC$: Current instruction address.
  - $SR$: Full 16-bit status register in hex.
  - Supervisor mode badge: `[S]` (Supervisor) vs `[U]` (User).
  - Interrupt Mask: `IPL: 0..7`.
- **Condition Code Flags (CCR):**
  - Glowing LED badges: `[ X ]`, `[ N ]`, `[ Z ]`, `[ V ]`, `[ C ]`.
  - Color: Active (1) = Bright Green; Inactive (0) = Dark Gray.
  - Clicking any flag toggles its state directly in CCR.
- **Prefetch Queue:**
  - Displays $IR$ (Instruction Register) and $IRC$ (Instruction Register Capture).
- **Diff Highlighting:** Any register whose value changed during the last step is highlighted in bright cyan, fading back to normal text color.
- **Inline Editing:** Double-clicking any register value opens an inline text box to modify its value directly.

---

### 3.5 Left Dock: Microcode State Inspector (`layout/left_dock/microcode.rs`)
- **Active Archetype:** Displays the active M68000 micro-operation handler name.
- **Micro-Step Counter:** `Step K / N` (e.g. `Step 2 / 4`).
- **Color Clock Phase:** Active phase badge: `[CCK1 (Bus Drive)]` vs `[CCK2 (Bus Latch / ALU)]`.
- **Dual Staging Registers:**
  - `addr1` ($X_1$, Source staged pointer).
  - `addr2` ($X_2$, Destination staged pointer).
  - `scratch[0..3]` internal calculation staging words.
- **Bus Cycle Contention:**
  - Transfer state: `Read`, `Write`, or `Idle`.
  - Wait states: `Wait States: 0` (or count of Agnus DMA contention stalls).

---

### 3.6 Left Dock: Disassembly View (`layout/left_dock/disassembly.rs`)
- **Listing Format:**
  - Columns: `[● Breakpoint] [Address] [Hex Opcode Words] [Mnemonic] [Operands]`
  - Example: `●  001004: 3200           MOVE.W   D0, D1`
- **Execution Cursor:** The instruction matching the current $PC$ is highlighted with a prominent background banner and a bold right-arrow indicator (`->`).
- **Breakpoint Interaction:** Clicking the margin next to any instruction toggles an execution breakpoint in `debugger.breakpoints`. A red dot `●` indicates an active breakpoint.
- **Direct PC Jump:** Double-clicking any instruction line sets the CPU $PC$ to that address and primes the prefetch queue.
- **Navigation:** Address jump text input allowing instant scrolling to arbitrary memory locations.

---

### 3.7 Right Dock: Memory Hex Editor (`layout/right_dock/memory_hex.rs`)
- **Fast Chunk Navigator Buttons:**
  - `[Vectors $000000]`
  - `[Low RAM $001000]`
  - `[Screen RAM $070000]`
  - `[Slow RAM $C00000]`
  - `[Kickstart $FC0000]`
  - `[Custom Chips $DFF000]`
  - `[CIA-A $BFE001]`
- **Hex Grid:**
  - 16 bytes per row: `[Address 6-hex]: [XX XX XX XX XX XX XX XX  XX XX XX XX XX XX XX XX] | [ASCII 16-chars]`
- **Inline Memory Editing:**
  - Clicking any hex byte allows direct keyboard input of new hex values.
  - Automatically commits changes to physical memory via `bus.write_byte_debug(addr, new_val)`.
- **ASCII Representation:** Printable ASCII characters rendered directly; non-printable bytes rendered as `.`.

---

### 3.8 Right Dock: Memory Search (`layout/right_dock/memory_search.rs`)
- **Search Modes:**
  - `Hex Sequence`: e.g. `4E 71 32 00 60 FA`
  - `ASCII String`: e.g. `DOS` or `AMIGA`
- **Action Buttons:** `Find Next` / `Find Previous`.
- **Behavior:** Searches through active memory banks; on match, scrolls the hex editor to the target offset and highlights the matched range.

---

### 3.9 Right Dock: Trace History Log (`layout/right_dock/trace_log.rs`)
- Displays recent instruction history from the 1024-entry `TraceRingBuffer`.
- Columns: `[CCK Timestamp] [PC] [Opcode] [Disassembled Instruction]`
- Clicking any history row synchronizes the temporal debugger to that exact past state.

---

## 4. Keyboard Shortcuts Matrix

| Key Combo | Action | Description |
|---|---|---|
| `F5` / `Space` | Toggle Run / Pause | Start or pause continuous execution |
| `F10` | Step Instruction | Execute 1 completed M68000 instruction |
| `F11` | Step CCK | Execute 1 Color Clock phase (2 CPU clocks) |
| `Shift + F10` | Step Backward | Rewind 1 step back in history |
| `Ctrl + R` | Reset A500 | Cold-reset machine and CPU state |
| `Ctrl + O` | Load Binary | Open file chooser to load machine code into RAM |
| `Ctrl + +` | Zoom In | Increase UI scaling by 25% |
| `Ctrl + -` | Zoom Out | Decrease UI scaling by 25% |
| `Ctrl + 0` | Reset Zoom | Reset UI scaling to 100% |
