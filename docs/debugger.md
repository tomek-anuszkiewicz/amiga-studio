# Developer Studio & Time-Travel Debugger

The Amiga 500 emulator provides a built-in immediate-mode **Developer Studio and Time-Travel Debugger** implemented in `crates/gui` and `crates/debugger`.

---

## 1. Launching the Developer Studio

- **Default Launch:** Running the GUI without flags opens directly with the Developer Studio docked:
  ```powershell
  cargo run -p gui
  ```
- **From Game Mode:** When running in clean game mode (`cargo run -p gui -- --game`), press **`F12`** at any time to pause emulation and switch into Developer Studio.
- **Direct Binary Loading:** To boot directly into a compiled machine code binary or kickstart snapshot:
  ```powershell
  cargo run -p gui -- --load path/to/game.bin --addr 001000
  ```

---

## 2. Developer Keybindings & Controls

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`F12`** | **Toggle Screen Mode** | Switches between Clean Game Screen and Developer Studio. Pauses emulation on entry. |
| **`F5` / `Space`** | **Run / Pause** | Toggles continuous free-running emulation. |
| **`F10`** | **Step Instruction** | Executes exactly 1 M68000 instruction, recording trace and history. |
| **`Shift + F10`** | **Step Backward (Rewind)** | Restores previous execution snapshot from the high-capacity temporal ring buffer. |
| **`F11`** | **Step CCK** | Steps 1 Color Clock phase (2 CPU clocks, CCK1 / CCK2). |
| **`Alt + T`** | **Toggle Recording** | Activates or pauses temporal execution history recording. |
| **`Ctrl + O`** | **Load Binary** | Opens file dialog to load compiled machine code at an arbitrary RAM address. |
| **`Ctrl + R`** | **Reset Cold** | Restores initial hardware state and reloads CPU vectors. |
| **Drag & Drop** | **Quick File Loading** | Drag any `.adf`, `.bin`, or `.rom` directly onto the window. |

---

## 3. Core Debugger Subsystems

### A. Time-Travel Rewind ($\ge 1.0\text{s}$ PAL History)
A high-capacity ring buffer records cycle-exact CPU and bus snapshots with zero dynamic heap allocations in hot paths:
- **Instruction Scrubbing:** Step backward and forward through past instructions to locate bugs or crashes.
- **Frame Scrubbing:** Jump backward by entire 50 Hz PAL video frames (~70,824 CCK cycles).
- **Cycle-Exact Timestamps:** Inspect exact Color Clock cycle timestamps and register states at each step.

### B. Live Register Inspector
Interactive view of all 68000 CPU registers:
- **Data & Address Registers:** $D_0-D_7$ and $A_0-A_7$ displayed in hex and decimal.
- **System Registers:** Program Counter ($PC$), Status Register ($SR$ with individual flag indicators $X, N, Z, V, C, I_0-I_2, S$), User Stack Pointer ($USP$), and Supervisor Stack Pointer ($SSP$).
- **Diff Highlighting:** Mutated registers glow with a cyan highlight when their values change between steps.
- **Interactive Editing:** Click any register value to edit it in hexadecimal.

### C. Memory Hex Grid
Live memory viewer displaying memory banks across Chip RAM, Slow RAM, and Fast RAM:
- **Inline Editing:** Click any byte to edit in hex. Press `Tab` or `Enter` to advance to the next byte, or `Esc` to cancel.
- **Mutation Glow:** Memory locations modified during recent cycles glow in amber/cyan.
- **Address Navigation:** Jump directly to any 24-bit Amiga address ($000000..$FFFFFF).

### D. Disassembly & In-Place Instruction Editing
- **Live Disassembler:** Powered by the zero-dependency `crates/disassembler` crate, providing instruction decoding with aligned multi-line view.
- **In-Place Assembly:** Click the pencil icon (`✏`) next to any instruction to type standard M68000 assembly (e.g. `NOP`, `MOVE.W D0, D1`, `BRA.S label`) with automated byte-size safety verification before patching memory.

### E. Execution Breakpoints & Memory Watchpoints
- **PC Breakpoints:** Pause emulation when the Program Counter reaches a designated address.
- **Memory Watchpoints:** Trigger breaks on `Read`, `Write`, or `Any` access across arbitrary memory ranges.

---

## 4. Architecture & Upstream References

For complete data structure definitions, temporal trace buffer schemas, and integration details, consult:
- **Obsidian Design Spec:** [`Obsidian/Amiga/Design/Debugger.md`](../Obsidian/Amiga/Design/Debugger.md)
- **Frontend Guidelines:** [`Obsidian/Amiga/Design/egui Guidelines.md`](../Obsidian/Amiga/Design/egui%20Guidelines.md)
- **Source Crates:** [`crates/debugger/`](../crates/debugger/) and [`crates/disassembler/`](../crates/disassembler/)
