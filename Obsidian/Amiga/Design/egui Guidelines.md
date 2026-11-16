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
- **Rules & Policies:** [egui-best-practices.md](../../../.agents/rules/egui-best-practices.md) | [Rust Guidelines.md](Rust%20Guidelines.md)
- **Module Location:** `crates/gui/`
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md).

> [!NOTE]
> This document details the architectural principles, design patterns, and engineering best practices for building native desktop and WebAssembly user interfaces with `egui` and `eframe` in the Amiga 500 emulator project.

---

## 1. Immediate-Mode Architecture & Frame Lifecycle

`egui` is an immediate-mode graphical user interface library:
- **No Retained Widget Tree:** Unlike Qt or React, widgets are not persisted as stateful DOM objects. On every frame, `update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)` runs from top to bottom, reconstructing the UI layout and issuing drawing commands in real time.
- **State Belongs in the Model:** The UI retains minimal internal state. All operational data (registers, memory, execution state, breakpoints) lives in the emulator core (`Cpu`, `MemoryBus`, `Debugger`), not inside UI widgets.

---

## 2. Coherent Owner Data and Retained Inspection

The complete session belongs to the [host worker](Application/GUI.md#input-and-output).
GUI widgets never execute or mutate a live machine directly.

1. **Stopped Preparation:** Panels format coherent CPU, memory, breakpoint, trace
   and timeline data obtained at a stopped owner boundary. Historical CPU snapshots
   remain on the worker; the GUI receives the positions and PCs needed for navigation.
2. **Retained Presentation During Run:** Inspectors draw the last prepared contents,
   including their CCK and instruction counters. Before Run, the owner supplies a
   coherent snapshot for preparation. Continuous Run and a pending Pause perform
   no new inspection or diagnostic formatting. Game neither requests nor draws
   inspectors. Core chips have no GUI callbacks, shared runtime handles or messages.
3. **Ordered Mutations:** Register edits, flag toggles, memory patches, breakpoints,
   history and execution controls become typed owner commands. The GUI applies
   owned replies asynchronously and rejects obsolete generations. File histories
   and success labels update only after the corresponding successful reply.

---

## 3. Independent Execution and Presentation

- **One Machine Owner:** A native thread or browser Web Worker runs bounded CCK
  slices. All chips remain synchronous within that owner. Guest execution does
  not depend on GUI repaint cadence.
- **Responsive UI:** GUI reads only try shared locks. A full video slot stops new
  image publication without preventing the owner from servicing controls. The
  [playback contract](Application/GUI.md#input-and-output) defines field selection,
  speed, host deadlines and image ownership.
- **Event-Driven Repaint:** Images, replies and shutdown wake the GUI. The GUI
  converts a claimed image into owned presentation data and releases it before
  texture upload. Repeated layout consumes no second image in the same UI frame.
  Native execution parks until work is due; browser execution yields to worker
  messages between slices. Idle GUI updates do not generate a repeated clock
  command/reply repaint cycle.

---

## 4. Visual-to-Source 1:1 Layout Mapping

Organize panels by their visible responsibility so a layout change has an obvious
source owner. The [layout tree](../../../crates/gui/src/layout/) is the living map;
each layout file corresponds to a visually distinct dock or panel. Follow the
[cohesion policy](../../../.agents/rules/file-size-and-cohesion.md) within that responsibility.

---

## 5. High-DPI Scaling & Browser Zoom Adaptation

Interfaces must look sharp and properly proportioned across monitors and browser viewports:
- **Desktop High-DPI:** `eframe` automatically detects OS scale factors (100%–200%) and sets `egui::Context::set_pixels_per_point`. Do not hardcode absolute pixel sizes for text or UI elements; use relative units (`em`, point sizes, and layout proportions).
- **WebAssembly Zoom:** `eframe::WebRunner` automatically binds to `window.devicePixelRatio`. Browser zoom actions (`Ctrl +` / `Ctrl -`) scale the vector UI automatically.
- **Retro Pixel Viewport Isolation:** The Amiga display viewport in [`amiga_screen.rs`](../../../crates/gui/src/layout/main_viewport/amiga_screen.rs) uses one scale for both axes, following the [image scaling contract](Application/GUI.md#scaling-and-appearance). Fit uses bilinear sampling; fixed scales use nearest-neighbor sampling in physical host pixels, with a pixel-aligned origin and clipping to the display area. Tool panels continue to scale with the host display.

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

Because `egui` draws every widget on every frame:
- **Avoid Heap Allocations in Render Loops:** Do not allocate new `Vec`s or perform heavy `format!` concatenations inside inner loops.
- **Reusable Format Buffers & Slices:** Use stack-local variables or pre-formatted static labels where possible.
- **Lazy/Virtual Scrolling:** In large views (e.g. 512 KB memory hex grid in [`memory_hex.rs`](../../../crates/gui/src/layout/right_dock/memory_hex.rs) or 1024-entry trace history in [`trace_log.rs`](../../../crates/gui/src/layout/right_dock/trace_log.rs)), use `egui::ScrollArea::show_rows` to render only the rows currently visible on screen alongside zero-allocation entry indexing (`trace.get(idx)`).

---

## 8. Self-Documenting UI & Comprehensive In-App Documentation

The emulator frontend embodies the **Zero-External-Lookup Principle**: the application serves as a self-contained, interactive hardware encyclopedia and execution studio. A developer, reverse engineer, or retro enthusiast exploring the system should never need to leave the emulator to consult external Commodore Hardware Reference Manuals (HRM), Motorola 68000 PRMs, or hardware register specs to understand what a bit, signal, register, or addressing mode means.

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
`egui` evaluates UI commands on every frame. To ensure high performance while delivering rich documentation:
- **Lazy Evaluation Invariance:** `egui` executes `.on_hover_ui` closures **only when the pointer actively hovers over the widget's bounding rect**. Rich layout code, tables, and typography inside hover closures execute with zero overhead during ordinary running frames.
- **Static String Slices (`&'static str`):** Hardware specifications, bitfield definitions, and reference notes should use compile-time static strings to guarantee zero heap allocations.
- **Structured Layouts in `.on_hover_ui`:** Give each tooltip a clear heading, a short explanation of the value, and aligned labels for related fields or bits. For example, a status-register tooltip can explain its system and condition-code fields, then list the supervisor bit and interrupt priority mask with their meanings. Use a grid when values need to line up in columns.

---

## 9. Reference Documentation & Upstream Ground Truth

- [egui Best Practices Rule](../../../.agents/rules/egui-best-practices.md): Mandatory immediate-mode UI rules and headless testing policies.
- [GUI](Application/GUI.md): Frontend role, game and debugger views, intended input and output paths, and display scaling.
- [GUI Crate Implementation](../../../crates/gui/src/gui.rs): Living Rust implementation of eframe app, views, docks, and modals.
- [GUI Interaction Test Suite](../../../crates/gui/tests/test_interactions.rs): Headless integration tests validating UI layout, keyboard events, and theme toggling.


