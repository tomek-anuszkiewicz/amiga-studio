# Developer Studio & Time-Travel Debugger

Use Developer Studio to pause the emulator, inspect the machine or follow what
a program does as it runs. The guide covers everyday controls and views and ends
with links to implementation details.

---

## 1. Launching the Developer Studio

- **Default Launch.** Run the GUI, choose **Return to session** from the library, and press **F12** to open Developer Studio.
  ```powershell
  cargo run -p gui
  ```
- **From Game Mode.** In a session, press **F12** to pause emulation and open Developer Studio.
- **Direct Binary Loading.** Run this command to load compiled machine code into RAM.
  ```powershell
  cargo run -p gui -- --load path/to/game.bin --addr 001000
  ```

---

## 2. Developer Keybindings & Controls

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`F12`** | **Toggle Screen Mode** | Switches between Clean Game Screen and Developer Studio. Pauses emulation on entry. |
| **`F5`** | **Run / Pause** | Runs at the selected speed; pause finishes the current video field. |
| **`F10`** | **Step Instruction** | Executes exactly 1 M68000 instruction, recording trace and history. |
| **`Shift + F10`** | **Step Backward (Rewind)** | Restores a recorded CPU state; full-machine rewind remains pending. |
| **`F11`** | **Step CCK** | Steps 1 Color Clock phase (2 CPU clocks, CCK1 / CCK2). |
| **`Shift + F11`** | **Step Frame** | Advances to the next completed PAL/NTSC video field and pauses. Breakpoints and CPU halts take precedence. |
| **`Alt + T`** | **Toggle Recording** | Activates or pauses temporal execution history recording. |
| **`Ctrl + O`** | **Load Binary** | Opens file dialog to load compiled machine code at an arbitrary RAM address. |
| **Drag & Drop** | **Binary Loading** | Opens the binary injection dialog for the dropped bytes. To boot firmware, open Manual setup in Game view. |

Use the minus and plus buttons beside **Speed** to select 0.25x, 0.5x, 1x,
2x or 4x. **1x** is
normal PAL/NTSC speed. At normal or slower speed, Run presents each completed
video field before waiting for the next field deadline. Above 1x, Run executes
every field and presents only the latest completed image from each batch. The
target rate is 50 Hz for PAL or 60 Hz for NTSC. **FPS** counts all emulated
fields completed per host second, including fields the display skips. For an
interlaced image, frame controls count each field separately. Host capacity can
limit the achieved speed and presentation rate. When paused, instruction and CCK stepping can show a
partly completed field.

CPU **STOPPED** means the processor is waiting for an interrupt. In both views,
Run and Step Frame still advance the rest of the machine, including video and
timers. During continuous Run, the inspectors retain the last values prepared
for display, including CPU status and CCK labels. The Pause control, FPS and
Amiga screen stay live. A CPU halt ends Run and refreshes the inspectors.

To boot a 256 KiB Kickstart image, choose **Game View**, **Manual setup**,
then **Select Kickstart...**. Choose **Start** to boot and F12 to return to
inspection. Loading a snapshot restores the ROM stored in it.

Game view shows the State menu, speed, measured FPS, theme, zoom and image
scale. **Pause** and **Resume** are in the player controls. Press F12 or the
configured shortcut to return to inspection.
The keyboard works while the running session is active. Click the display to
capture the mouse. The middle mouse button frees the mouse while play
continues. You can assign a mouse release key in Configurations, where it
starts unassigned. Reset
has no keyboard shortcut. Game View disconnects debugger panels and shortcuts,
breakpoints, disassembly, trace collection, CPU history and memory diff capture.
The CPU's STOP instruction still allows the chips to run and deliver interrupts.
The emulator saves state only when you request a save from the State menu or one
of its shortcuts.

---

## 3. Core Debugger Subsystems

### A. Execution History and Rewind
Continuous Run checks enabled PC breakpoints and their register conditions against
the current CPU state. Trace and execution history recording stay off during Run,
even with Rec enabled. Existing entries remain available. When you stop, history
still has a gap for the unrecorded interval. Instruction, CCK and frame steps
retain diagnostic recording.

The inspector's registers, memory, decoded code and counters keep their displayed
values during Run and while Stop is pending. Editing and navigation resume when
execution stops. A breakpoint stops before the matching instruction, so you may
see a partial field. An ordinary Stop lets the current field finish. Reset and
state loading refresh the inspection view. On return from Game View, diagnostics
resume after its pending pause completes.

The recorded history lets you browse CPU states. Full-machine rewind is tracked
on the roadmap. [Debugger work](../ROADMAP.md#debugger-execution-control)

Within the retained history, you can use these controls.

- **Instruction Scrubbing.** Step backward and forward through past instructions to locate bugs or crashes.
- **Frame Scrubbing.** Jump backward by entire 50 Hz PAL video frames (~70,824 CCK cycles).
- **Cycle-Exact Timestamps.** Inspect each step's exact Color Clock timestamp and register state.

### B. Live Register Inspector
The inspector shows all 68000 CPU registers and lets you edit them.
- **Data & Address Registers:** $D_0-D_7$ and $A_0-A_7$ displayed in hex and decimal.
- **System Registers:** Program Counter ($PC$), Status Register ($SR$ with individual flag indicators $X, N, Z, V, C, I_0-I_2, S$), User Stack Pointer ($USP$), and Supervisor Stack Pointer ($SSP$).
- **Diff Highlighting.** Cyan highlights mark registers whose values changed between steps.
- **Interactive Editing:** Click any register value to edit it in hexadecimal.

### C. Memory Hex Grid
The live memory grid shows banks across Chip RAM, Slow RAM and Fast RAM.
- **Inline Editing:** Click any byte to edit in hex. Press `Tab` or `Enter` to advance to the next byte, or `Esc` to cancel.
- **Mutation Glow:** Memory locations modified during recent cycles glow in amber/cyan.
- **Address Navigation:** Jump directly to any 24-bit Amiga address ($000000..$FFFFFF).

### D. Disassembly & In-Place Instruction Editing
- **Live Disassembler.** The zero-dependency `crates/disassembler` crate decodes instructions for an aligned, multiline display.
- **In-Place Assembly.** Click the pencil icon (`✏`) next to an instruction and type standard M68000 assembly, for example `NOP`, `MOVE.W D0, D1` or `BRA.S label`. Developer Studio checks the size in bytes before patching memory.

### E. Execution Breakpoints & Memory Watchpoints
- **PC Breakpoints:** Pause emulation when the Program Counter reaches a designated address.
- **Memory Watchpoints:** Configure `Read`, `Write`, or `Any` ranges; execution stops remain pending. See the [debugger work](../ROADMAP.md#debugger-execution-control).

---

## 4. Architecture & Upstream References

These references cover the full data structure definitions, temporal trace buffer
schemas and integration details.
- **Obsidian Design Spec:** [`Obsidian/Amiga/Design/Application/Debugger.md`](../Obsidian/Amiga/Design/Application/Debugger.md)
- **Frontend Guidelines:** [`Obsidian/Amiga/Design/egui Guidelines.md`](../Obsidian/Amiga/Design/egui%20Guidelines.md)
- **Source Crates:** [`crates/debugger/`](../crates/debugger/) and [`crates/disassembler/`](../crates/disassembler/)
