---
title: "Amiga 500 egui & Frontend Best Practices"
aliases: ["egui Guidelines", "Frontend Guidelines", "egui Best Practices"]
tags: ["amiga", "design", "gui", "egui", "frontend"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-10
updated: 2026-11-16
related: ["[GUI.md](Application/GUI.md)", "[Rust Guidelines.md](Rust%20Guidelines.md)"]
---

# Amiga 500 egui & Frontend Best Practices

- **Parent Specification:** [GUI.md](Application/GUI.md)
- **Shared Practices:** [egui-best-practices.md](../../../.agents/references/egui-best-practices.md) | [Rust Guidelines.md](Rust%20Guidelines.md)
- **Module Location:** `crates/gui/`
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md).

> [!NOTE]
> These guidelines cover the emulator's native desktop and WebAssembly
> interfaces. They explain how `egui` draws the interface and how `eframe`
> provides the desktop and browser application framework.

---

## 1. Immediate-Mode Architecture & Frame Lifecycle

`egui` is an immediate-mode graphical user interface library. Each frame,
`update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)` runs from
top to bottom. It rebuilds the layout and issues drawing commands for that
frame. Qt and React use a retained model in which the interface maintains
stateful widgets or elements across frames.

The interface keeps minimal internal state. The emulator core keeps the
registers, memory, execution state and breakpoints in `Cpu`, `MemoryBus` and
`Debugger`.

---

## 2. Coherent Owner Data and Retained Inspection

The host worker owns the entire session. Widgets draw from the data and
replies it sends. To run the live machine or change its state, widgets send
commands to that worker. [Host worker](Application/GUI.md#input-and-output)

1. **Preparing data while stopped.** With the machine stopped, the worker
   provides CPU, memory, breakpoint, trace and timeline data from the same
   stopped state. Panels format that data. The worker keeps historical CPU
   snapshots and sends the GUI the positions and PCs it needs for navigation.
2. **Drawing the last inspection during Run.** The worker supplies a
   coherent snapshot for inspection before Run. Inspectors keep drawing
   the last prepared contents and their CCK and instruction counters.
   Continuous Run and a pending Pause reuse that display without new
   inspection or diagnostic formatting. Game leaves inspectors out of both
   requests and drawing. Core chips operate without GUI callbacks, shared
   runtime handles or messages.
3. **Applying changes in order.** The GUI sends register edits, flag toggles,
   memory patches, breakpoints, history and execution controls as typed
   commands to the worker. The GUI applies owned replies asynchronously and
   discards replies from obsolete generations. It updates file histories and
   success labels only after the corresponding reply reports success.

---

## 3. Independent Execution and Presentation

- **One machine owner.** A native thread or browser Web Worker runs bounded
  CCK slices and keeps all the chips synchronous. The guest runs
  independently of the GUI's repaint rate.
- **Responsive reads.** The GUI uses try-locks for shared reads. While the
  video slot is full, the worker continues to service controls and cannot
  publish a new image. The playback contract defines field selection, speed,
  host deadlines and image ownership.
  [Playback contract](Application/GUI.md#input-and-output)
- **Event-driven repaint.** Images, replies and shutdown events wake the GUI.
  It converts a claimed image into presentation data it owns and releases the
  image before uploading the texture. Repeated layout in the same UI frame
  uses the same presentation data without consuming a second image. On the
  desktop, the thread parks until work is due. In the browser, the worker
  yields to messages between slices. An idle GUI update does not keep
  triggering repaints by sending clock commands and receiving replies.

---

## 4. Visual-to-Source 1:1 Layout Mapping

Organize panels by what they show or let the user do so a developer can locate
the source for a visible layout change. The layout tree shows which visually
distinct dock or panel each layout file owns. Follow the cohesion criteria
within each file's responsibility.
[Layout tree](../../../crates/gui/src/layout/),
[cohesion criteria](../../../.agents/references/architecture-criteria.md)

---

## 5. High-DPI Scaling & Browser Zoom Adaptation

The interface must stay sharp and keep its proportions across monitors and
browser viewports.

- **Desktop High-DPI.** `eframe` detects OS scale factors (100%–200%) and
  sets `egui::Context::set_pixels_per_point` automatically. Size text and
  interface elements in relative units (`em`, point sizes and layout
  proportions) instead of hardcoding absolute pixel sizes.
- **WebAssembly zoom.** `eframe::WebRunner` automatically uses
  `window.devicePixelRatio`. Browser zoom (`Ctrl +` / `Ctrl -`) scales the
  vector UI automatically.
- **Amiga display viewport.** The viewport uses one scale for both axes. Fit
  uses bilinear sampling. Fixed scales use nearest-neighbor sampling in
  physical host pixels, align the origin to a pixel and clip to the display
  area. Tool panels scale with the host display.
  [`amiga_screen.rs`](../../../crates/gui/src/layout/main_viewport/amiga_screen.rs),
  [image scaling contract](Application/GUI.md#scaling-and-appearance)

---

## 6. Theme, Semantic Design Tokens & Visual Styling

- **Centralized Design Tokens (`ColorTokens`):**
  - All panels, surfaces, text, diffs, and condition code badges use the immutable `ColorTokens` struct ([`tokens.rs`](../../../crates/gui/src/theme/tokens.rs)).
  - Supported palettes: **Retro Studio Pro Dark** (default), **Clean Light**, and **Classic Amiga Workbench 1.3** ([`theme.rs`](../../../crates/gui/src/theme.rs)).
- **Visual Feedback & State Diffing:** Changes, the active PC, CCR state and
  breakpoints need distinct visual cues. Use the named semantic tokens in
  [`tokens.rs`](../../../crates/gui/src/theme/tokens.rs), preserving their roles
  across palettes instead of copying color literals into panel code.
  Register and memory changes retain cyan highlighting; the active PC retains
  its blue accent and background. High-contrast CCR badges remain clickable,
  sending typed flag commands to the machine owner. Breakpoints retain red dot
  markers in the disassembly margin.

---

## 7. Zero-Allocation UI Drawing

`egui` draws widgets each frame, so inner loops must avoid allocating new
`Vec`s and performing heavy `format!` concatenations. Use stack-local
variables or preformatted static labels where possible.

For large views, use `egui::ScrollArea::show_rows` to render only the rows
visible on screen and index entries without allocating (`trace.get(idx)`).
Examples include the 512 KB memory hex grid and 1024-entry trace history.
[`memory_hex.rs`](../../../crates/gui/src/layout/right_dock/memory_hex.rs),
[`trace_log.rs`](../../../crates/gui/src/layout/right_dock/trace_log.rs)

---

## 8. Self-Documenting UI & Comprehensive In-App Documentation

Under the **Zero-External-Lookup Principle**, the emulator serves as an
interactive hardware reference as well as a place to run and examine code.
A developer, reverse engineer or retro enthusiast should be able to
understand a bit, signal, register or addressing mode within the application.
The interface should supply the context that would otherwise require a
lookup in the Commodore Hardware Reference Manuals (HRM), Motorola 68000
PRMs or hardware register specifications.

### 8.1 Universal Hover Documentation Coverage
Every inspectable or interactive element across all docks, panels, and modal dialogs must provide contextual documentation:
1. **CPU Registers ($D_0-D_7, A_0-A_7, PC, SR, USP, SSP$):**
   - Bit widths, valid addressing modes, byte/word/long access rules, and signed decimal representations.
   - Supervisor state restrictions and active stack pointer binding ($A_7 = \text{USP}$ in User mode vs $A_7 = \text{SSP}$ in Supervisor mode).
   - Program Counter normalization ($PC - 4$ instruction display vs hardware prefetch bus register).
2. **Condition Code Register ($CCR$) & System Byte ($SR$):**
   - Individual bit breakdowns for $X, N, Z, V, C$ with explicit mathematical setting/clearing criteria (e.g. $V = (Sm \land Dm \land \overline{Rm}) \lor (\overline{Sm} \land \overline{Dm} \land Rm)$) and their impact on conditional branching.
   - Complete System Byte breakdown: Trace mode $T$, Supervisor state $S$, and Interrupt Priority Level mask $I_0-I_2$.
3. **Microcode Engine & Bus Timing:**
   - Color Clock phase roles: $CCK1$ (address driving and bus arbitration) vs $CCK2$ (data latching, ALU step execution, and condition evaluation).
   - Staging registers: concrete roles of `addr1` ($X_1$) and `addr2` ($X_2$) in dual-memory operations.
   - DMA contention: visual explanation of bus lock states and Chip RAM wait states when blocked by Agnus or Denise.
4. **Memory Map & Addresses:**
   - Memory region identities (Chip RAM, Slow/Pseudo-Fast RAM, Auto-Config Fast RAM, CIA spaces, Custom Chip register space `$DFF000..=$DFFFFF`, Kickstart ROM).
   - Watchpoint triggers and access rules (Read, Write, Any).
5. **Timeline & Execution Controls:**
   - Explicit stepping granularity (Instruction vs CCK vs 1 PAL video frame).
   - Exact keyboard shortcuts and temporal rewind deltas.

### 8.2 Immediate-Mode Tooltip Implementation Patterns
`egui` evaluates UI commands on every frame. Keep the work of laying out
detailed documentation inside the tooltip.

- **Lazy evaluation.** `egui` executes `.on_hover_ui` closures **only while
  the pointer actively hovers over the widget's bounding rectangle**. The
  layout code, tables and typography inside the closure add zero execution
  overhead during ordinary frames without a hover.
- **Static string slices (`&'static str`).** Prefer compile-time static strings
  for hardware specifications, bitfield definitions and reference notes to
  guarantee zero heap allocations for that text.
- **Structured layouts in `.on_hover_ui`.** Give each tooltip a clear
  heading, a short explanation of the value and aligned labels for related
  fields or bits. A status-register tooltip can explain its system and
  condition-code fields, then describe the supervisor bit and interrupt
  priority mask. Use a grid when values need to line up in columns.

---

## 9. Reference Documentation & Upstream Ground Truth

- [egui Best Practices](../../../.agents/references/egui-best-practices.md): Shared immediate-mode UI requirements and headless testing criteria.
- [GUI](Application/GUI.md): Frontend role, game and debugger views, intended input and output paths, and display scaling.
- [GUI Crate Implementation](../../../crates/gui/src/gui.rs): Living Rust implementation of eframe app, views, docks, and modals.
- [GUI Interaction Test Suite](../../../crates/gui/tests/test_interactions.rs): Headless integration tests validating UI layout, keyboard events, and theme toggling.


