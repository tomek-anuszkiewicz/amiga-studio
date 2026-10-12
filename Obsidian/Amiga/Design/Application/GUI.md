---
title: "Amiga 500 GUI"
aliases: ["Frontend Architecture", "Developer Studio GUI", "Amiga 500 Developer GUI Specification"]
tags: ["amiga", "design", "gui", "egui", "eframe"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-05
updated: 2026-10-12
related: ["[egui Guidelines.md](../egui%20Guidelines.md)", "[Debugger.md](Debugger.md)", "[General Architecture.md](../General%20Architecture.md)", "[Keyboard.md](../Keyboard.md)", "[Mouse.md](../Mouse.md)", "[Audio.md](../Audio.md)", "[Denise.md](../Denise.md)"]
tracked_paths:
  - "crates/gui"
---
# Amiga 500 GUI

The GUI is the host-facing layer of the emulator. It presents the Amiga display and debugger state, receives host input, and is intended to play the machine's audio. The emulation core owns the machine's behavior; the GUI presents its outputs and translates user actions into machine input without making the core depend on a window or browser.

The same frontend targets a native desktop application and a browser build. It presents the same emulated machine in two views:

- **Game view** gives the framed Amiga screen the available window space and hides the menus and inspection panels.
- **Developer Studio** places the screen in a debugging workspace with execution controls and state inspection. The [debugger specification](Debugger.md) defines the behavior behind those controls.

The views change how the machine is presented, not which machine is running. F2 or F12 switches between them; Escape returns from the game view to the Developer Studio. Returning to the Developer Studio pauses execution so the current state can be inspected.

## Application and debugger layout

The Developer Studio has a top bar and three main areas: a left dock for CPU state, a central workspace for the screen and code, and a right dock for memory and tools. The width of each side dock can be changed. On a wide window, the central workspace separates disassembly from the screen: the screen and history controls sit above the execution trace. On narrower windows, the screen, history controls, and disassembly stack vertically in the central area; the trace moves into the right dock's tools section. The screen remains framed at a 4:3 presentation ratio in either layout.

| Area | Available information and controls |
| --- | --- |
| Top bar | Load a binary into memory or reset the machine; save and load state through files or quick slots; run, pause, step one instruction or Color Clock, and step backward through recorded history. Status and CCK/instruction counters sit beside theme, zoom, and game-view controls. |
| Left dock | CPU data and address registers, program counter, status register and condition flags, followed by engine status such as bus availability and counters. An optional microcode inspector shows the current execution phase and intermediate CPU state; F8 toggles it. |
| Central workspace | The framed Amiga screen, a history bar for recording and browsing CPU snapshots by step, frame, slider, or CCK position, and a disassembly listing. The listing can follow the current PC or jump to an address, set breakpoints, change the PC, and edit an instruction in place. On wide windows, the trace below the screen lists recent execution; the split between code and screen and the split above the trace are draggable. |
| Right dock | A hex and ASCII memory viewer with address navigation and byte editing. Below it are memory search and controls for PC breakpoints, register conditions, and read/write watchpoint ranges. On narrower windows, the execution trace is also here. |

The execution trace and the optional snapshot history serve different purposes: the trace lists recent instructions, while the history bar browses recorded CPU states. History recording starts disabled, and browsing a snapshot does not rewind memory or the rest of the machine. Watchpoints can be configured and highlighted, but guest memory accesses do not yet pause execution on a match. See [Debugger](Debugger.md) for these behavior limits and the [Debugger Guide](../../../../docs/debugger.md) for user-facing shortcuts.

## Input and output

The intended input path maps host keyboard events to the [Amiga keyboard](../Keyboard.md) and host mouse movement and buttons to the emulated [mouse and game port](../Mouse.md). Host gamepad or keyboard controls can likewise represent an emulated [joystick](../Joystick.md). These mappings belong at the frontend boundary; the emulated devices and chips decide their hardware effects.

For video, [Denise](../Denise.md) builds a rendered frame in the [frame buffer](../Frame%20Buffer.md). The GUI is intended to display that frame, preserve the intended screen proportions as the window changes, and optionally add a CRT-style presentation. For sound, the GUI is intended to deliver samples produced by the [audio subsystem](../Audio.md) to the host playback device. Emulated audio generation remains in the core.

The current GUI has the two views and a framed display area, but the display area still shows a standby image. Host audio playback and host input routing to the emulated keyboard and game ports are design intentions here, not completed GUI paths.

## Scaling and appearance

The interface uses logical sizes so controls and text remain readable at different desktop DPI settings and browser zoom levels. This scaling is separate from the Amiga image: enlarging the window should enlarge its presentation without changing the emulated display data or distorting its proportions.

The GUI offers dark and light themes, plus a Workbench-inspired theme. A theme changes the host interface's appearance without changing emulation state. The same view and scaling principles apply in the native application and the browser.

## References

- [General Architecture](../General%20Architecture.md): Machine ownership and the boundary between emulation and host presentation.
- [Debugger](Debugger.md): Execution and inspection behavior behind the debugger view.
- [egui Guidelines](../egui%20Guidelines.md): Frontend implementation and interaction guidance.
- [GUI source](../../../../crates/gui/src/gui.rs): Native and browser entry points, view state, and layout modules.
