---
title: "Amiga 500 GUI"
aliases: ["Frontend Architecture", "Developer Studio GUI", "Amiga 500 Developer GUI Specification"]
tags: ["amiga", "design", "gui", "egui", "eframe"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-05
updated: 2026-11-13
related: ["[egui Guidelines.md](../egui%20Guidelines.md)", "[Debugger.md](Debugger.md)", "[General Architecture.md](../General%20Architecture.md)", "[Keyboard.md](../Keyboard.md)", "[Mouse.md](../Mouse.md)", "[Audio.md](../Audio.md)", "[Denise.md](../Denise.md)"]
tracked_paths:
  - "crates/gui"
---
# Amiga 500 GUI

The GUI is the host-facing layer of the emulator. It presents the Amiga display and debugger state, receives host input, and is intended to play the machine's audio. The emulation core owns the machine's behavior; the GUI presents its outputs and translates user actions into machine input without making the core depend on a window or browser.

The same frontend targets a native desktop application and a browser build. It presents the same emulated machine in two views:

- **Game view** gives the framed Amiga screen the available space below a compact toolbar. File and State menus, Run/Stop, optional FPS display, speed ratio and Slower/Faster controls, theme, zoom and a Debug View switch remain available. Inspection panels and debugger shortcuts are disconnected.
- **Developer Studio** places the screen in a debugging workspace with execution controls and state inspection. The [debugger specification](Debugger.md) defines the behavior behind those controls.

The views share one machine. F2 or F12 switches between them; Escape returns from the game view to the Developer Studio. Returning requests a pause at the field boundary and keeps instrumentation disconnected until that pause completes. Game execution bypasses breakpoints, disassembly, trace collection, CPU history and memory diff snapshots. Previously collected history remains available when returning, but no automatic snapshots or progress recording run in Game view. Manual State operations remain available. A stopped CPU still allows chip clocks, video and interrupts to advance in both views; a halted CPU ends Run.

Continuous Developer Run retains the last prepared inspector contents, including register and CPU status values, disassembly, memory, history and toolbar CCK/instruction labels. The Amiga image, FPS and execution controls remain live. Editing, browsing new inspection data and history navigation are inactive until paused. Repaints and resizing lay out retained data without rereading the machine or rebuilding diagnostic text. Stop pending retains the same freeze; a real stop refreshes current data. Explicit instruction, CCK and frame steps retain their diagnostic collection and show their resulting state. Reset, state loading and paused edits refresh through the synchronous UI path. Enabled breakpoints use current CPU state and stop before their matching instruction, even before a field completes.

## Application and debugger layout

The Developer Studio has a top bar and three main areas: a left dock for CPU state, a central workspace for the screen and code, and a right dock for memory and tools. The width of each side dock can be changed. On a wide window, the central workspace separates disassembly from the screen: the screen and history controls sit above the execution trace. On narrower windows, the screen, history controls, and disassembly stack vertically in the central area; the trace moves into the right dock's tools section. In either layout, the monitor image supplied by the frame builder is centered in the available display area. Both axes use the selected image scale, preserving the monitor image's width-to-height ratio instead of imposing 4:3.

| Area | Available information and controls |
| --- | --- |
| Top bar | Load a binary into memory or reset the machine; save and load state through files or quick slots; run, pause at a video field boundary, step one field, instruction or Color Clock, and step backward through recorded history. Measured video FPS, speed controls and CCK/instruction counters sit beside theme, zoom, and game-view controls. |
| Left dock | CPU data and address registers, program counter, status register and condition flags, followed by engine status such as bus availability and counters. An optional microcode inspector shows the current execution phase and intermediate CPU state; F8 toggles it. |
| Central workspace | The framed Amiga screen, a history bar for recording and browsing CPU snapshots by step, frame, slider, or CCK position, and a disassembly listing. The listing can follow the current PC or jump to an address, set breakpoints, change the PC, and edit an instruction in place. On wide windows, the trace below the screen lists recent execution; the split between code and screen and the split above the trace are draggable. |
| Right dock | A hex and ASCII memory viewer with address navigation and byte editing. Below it are memory search and controls for PC breakpoints, register conditions, and read/write watchpoint ranges. On narrower windows, the execution trace is also here. |

The execution trace and the optional snapshot history serve different purposes: the trace lists recent instructions, while the history bar browses recorded CPU states. History recording starts disabled, and browsing a snapshot does not rewind memory or the rest of the machine. Watchpoint stops remain incomplete; see the [debugger work](../../../../ROADMAP.md#debugger-execution-control). See [Debugger](Debugger.md) for these behavior limits and the [Debugger Guide](../../../../docs/debugger.md) for user-facing shortcuts.

Binary loading into the Kickstart address window accepts only a 256 KiB image. A rejected load leaves the machine unchanged and keeps the load dialog available for correction. File and snapshot loading errors remain visible until dismissed, including the expected and actual ROM byte counts for an invalid image.

File also offers **Load ROM from File** on native desktop and WASM. A valid 256 KiB image is installed as Kickstart, followed by a cold reset with the ROM overlay and reset vectors active; execution remains paused until Run. The toolbar displays the loaded filename. Invalid images preserve both machine state and the previous filename. Loading a state replaces this label with a snapshot source because the snapshot includes its own ROM. Browser files are selected explicitly by the user; no local directory scanning is required. Browser file selection and download are host operations, separate from synchronous machine execution.

## Input and output

The intended input path maps host keyboard events to the [Amiga keyboard](../Keyboard.md) and host mouse movement and buttons to the emulated [mouse and game port](../Mouse.md). Host gamepad or keyboard controls can likewise represent an emulated [joystick](../Joystick.md). These mappings belong at the frontend boundary; the emulated devices and chips decide their hardware effects.

For video, [Denise](../Denise.md) builds a rendered frame in the [frame buffer](../Frame%20Buffer.md). Run completes a video field, uploads its monitor image into an egui texture, presents it with the selected image scaling, and waits until the next field is due. If a bounded slice ends before field completion, the viewport retains the previous image. Run at 1x follows the configured PAL or NTSC color clock using host elapsed time; emulation and rendering time count toward each deadline. Slower and Faster controls select 0.25x, 0.5x, 1x, 2x or 4x; a 1x control restores normal speed. FPS measures completed video fields per host second, independently of GUI repaints. Here, Step Frame and FPS count fields, including each field of an interlaced image. Each update advances at most one field, without skipping completed images; host execution and presentation capacity can limit the achieved speed.

Ordinary pause finishes the current field, and Step Frame advances to the next completed field, including while the CPU is stopped. Instruction/CCK stepping, breakpoints and CPU halts can still expose a partial field. Snapshot restoration replaces the displayed pixels on the next UI frame. Host timing, texture allocation and upload belong to the frontend. For sound, the GUI is intended to deliver samples produced by the [audio subsystem](../Audio.md) to the host playback device. Emulated audio generation remains in the core.

The shared `live_stripes` guest fixture supplies the live-frame smoke check without firmware. Full guest display acceptance remains [guest verification](../../../../ROADMAP.md#cpu-memory-gary-verification) work. Host audio/input remain incomplete; see [native test prerequisites](../../../../ROADMAP.md#native-disk-verification).

## Scaling and appearance

The interface uses logical sizes so controls and text remain readable at different desktop DPI settings and browser zoom levels. Interface zoom is separate from the Amiga image scale.

Both views offer one **Image scale** selector: **Fit (smooth)**, **0.5x**, **1x**, **2x**, and **4x**. Fit is the default; it uniformly fits the complete image into the display area with bilinear filtering. Fixed scales use nearest-neighbor filtering and count physical host pixels per framebuffer sample, independently of interface zoom and host DPI. Their origin aligns to the host pixel grid. An image larger than its display area stays at the selected scale, centered and clipped to that area; it does not shrink automatically or add scrollbars. The selected scale persists with the other GUI preferences. There is no separate filter control.

Changing image scale takes effect while paused or running. It changes presentation only: the machine state and framebuffer samples remain unchanged, and a filter change during an unfinished field retains the previously presented image.

The GUI offers dark and light themes, plus a Workbench-inspired theme. A theme changes the host interface's appearance without changing emulation state. The same view and scaling principles apply in the native application and the browser.

## References

- [General Architecture](../General%20Architecture.md): Machine ownership and the boundary between emulation and host presentation.
- [Debugger](Debugger.md): Execution and inspection behavior behind the debugger view.
- [egui Guidelines](../egui%20Guidelines.md): Frontend implementation and interaction guidance.
- [GUI source](../../../../crates/gui/src/gui.rs): Native and browser entry points, view state, and layout modules.
