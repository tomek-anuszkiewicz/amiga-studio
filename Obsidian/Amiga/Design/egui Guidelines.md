---
title: "Amiga 500 egui & Frontend Best Practices"
aliases: ["egui Guidelines", "Frontend Guidelines", "egui Best Practices"]
tags: ["amiga", "design", "gui", "egui", "frontend"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-10
updated: 2026-09-12
related: ["[GUI Specification.md](GUI%20Specification.md)", "[GUI.md](GUI.md)", "[Rust Guidelines.md](Rust%20Guidelines.md)"]
---

# Amiga 500 egui & Frontend Best Practices

- **Parent Specification:** [GUI.md](GUI.md)
- **Detailed Layout Specification:** [GUI Specification.md](GUI%20Specification.md)
- **Rules & Policies:** [egui-best-practices.md](../../../.agents/rules/egui-best-practices.md) | [Rust Guidelines.md](Rust%20Guidelines.md)
- **Module Location:** `crates/gui/`
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md).

> [!NOTE]
> This document details the architectural principles, design patterns, and engineering best practices for building native desktop and WebAssembly user interfaces with `egui` and `eframe` in the Amiga 500 emulator project.

---

## 1. The Immediate-Mode Paradigm & Lifecycle

`egui` is an immediate-mode graphical user interface library:
- **No Retained Widget Tree:** Unlike Qt or React, widgets are not persisted as stateful DOM objects. On every frame, `update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)` runs from top to bottom, reconstructing the UI layout and issuing drawing commands in real time.
- **State Belongs in the Model:** The UI retains minimal internal state. All operational data (registers, memory, execution state, breakpoints) lives in the emulator core (`Cpu`, `MemoryBus`, `Debugger`), not inside UI widgets.

---

## 2. Synchronous Direct-State Pull (Zero Async / Zero Callbacks)

To prevent cognitive complexity, concurrency bugs, and UI stalls:
1. **Direct State Access:** Panels directly query references to emulator state (`&app.cpu.state`, `&app.bus`, `&app.debugger`) during their draw calls.
2. **No Change Notifications:** The emulator core never fires "events" or sends "messages" when registers or memory change. The GUI simply reads whatever the current state is on every render pass.
3. **Synchronous Mutations:** User interactions (typing a new hex value into memory, toggling a CCR flag, clicking a breakpoint) execute synchronously against the underlying state in the same frame tick.

---

## 3. Single-Threaded Time-Slice Execution Model

To ensure seamless compilation to **WebAssembly (`wasm32-unknown-unknown`)** where native OS threads are not readily available:
- **Bounded Cycle Slices:** When the emulator is running continuously (F5), the frame loop executes a bounded budget of machine cycles (e.g. 1/60th second of PAL Color Clocks = ~141,875 cycles, or up to 10,000 instructions) per GUI frame.
- **Responsive UI Under Load:** Because execution is bounded per frame, the host window never freezes. If guest code enters an infinite loop (`BRA *`), the user can still hit Pause (Space / F5) or click buttons without lag.
- **Frame Rate Synchronization:** Request repaints (`ctx.request_repaint()`) only when the emulator is running or animating. When paused, `egui` renders strictly on user interaction, saving host CPU cycles.

---

## 4. Visual-to-Source 1:1 Layout Mapping

To maintain code clarity and eliminate mystery files, the source code directory hierarchy strictly reflects the physical layout on screen:
- [`top_menu_bar.rs`](../../../crates/gui/src/layout/top_menu_bar.rs): Top toolbar (file loader, execution controls, theme, zoom).
- [`main_viewport/`](../../../crates/gui/src/layout/main_viewport/): Centered primary workspace (Amiga CRT canvas, temporal rewind scrubber).
- [`left_dock/`](../../../crates/gui/src/layout/left_dock/): CPU and execution inspection (registers, microcode state, disassembly).
- [`right_dock/`](../../../crates/gui/src/layout/right_dock/): Memory inspection, breakpoints, and history (memory hex editor, search, breakpoints panel, trace log).

Each `.rs` file corresponds to a single, visually distinct dock or panel on the screen.

---

## 5. High-DPI Scaling & Browser Zoom Adaptation

Interfaces must look sharp and properly proportioned across monitors and browser viewports:
- **Desktop High-DPI:** `eframe` automatically detects OS scale factors (100%–200%) and sets `egui::Context::set_pixels_per_point`. Do not hardcode absolute pixel sizes for text or UI elements; use relative units (`em`, point sizes, and layout proportions).
- **WebAssembly Zoom:** `eframe::WebRunner` automatically binds to `window.devicePixelRatio`. Browser zoom actions (`Ctrl +` / `Ctrl -`) scale the vector UI automatically.
- **Retro Pixel Viewport Isolation:** The 4:3 Amiga display viewport ($320 \times 256$) in [`amiga_screen.rs`](../../../crates/gui/src/layout/main_viewport/amiga_screen.rs) uses explicit integer prescaling (1x, 2x, 3x) within its container, ensuring vintage pixel graphics remain crisp and undistorted while tool panels scale with the host display.

---

## 6. Theme & Visual Styling

- **Unified Palette:** Follow a curated, low-eyestrain developer dark theme by default, with automatic fallback to host OS preference ([`theme.rs`](../../../crates/gui/src/theme.rs)).
- **Visual Feedback & State Diffing:**
  - When stepping through code, highlight mutated registers and memory cells with a distinct accent color (cyan) that fades back to baseline text ([`registers.rs`](../../../crates/gui/src/layout/left_dock/registers.rs)).
  - Active condition code flags (CCR) render as glowing LED badges (green = 1, dark gray = 0) with clickable toggle states directly mutating `state.sr`.
  - Breakpoints render as distinct colored markers (red dot `●`) in the disassembly margin ([`disassembly.rs`](../../../crates/gui/src/layout/left_dock/disassembly.rs)).

---

## 7. Zero-Allocation UI Drawing

Because `egui` draws every widget on every frame:
- **Avoid Heap Allocations in Render Loops:** Do not allocate new `Vec`s or perform heavy `format!` concatenations inside inner loops.
- **Reusable Format Buffers & Slices:** Use stack-local variables or pre-formatted static labels where possible.
- **Lazy/Virtual Scrolling:** In large views (e.g. 512 KB memory hex grid in [`memory_hex.rs`](../../../crates/gui/src/layout/right_dock/memory_hex.rs) or 1024-entry trace history in [`trace_log.rs`](../../../crates/gui/src/layout/right_dock/trace_log.rs)), use `egui::ScrollArea::show_rows` to render only the rows currently visible on screen alongside zero-allocation entry indexing (`trace.get(idx)`).

---

## 8. Self-Documenting UI & Comprehensive In-App Documentation

The emulator frontend embodies the **Zero-External-Lookup Principle**: the application serves as a self-contained, interactive hardware encyclopedia and execution studio. A developer, reverse engineer, or retro enthusiast exploring the system should never need to leave the emulator to consult external Commodore Hardware Reference Manuals (HRM), Motorola 68000 PRMs, or Guru Book specs to understand what a bit, signal, register, or addressing mode means.

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
- **Structured Layouts in `.on_hover_ui`:** Use bold headings (`ui.heading`), monospace tables (`egui::Grid`), and bullet points for complex structures:
  ```rust
  response.on_hover_ui(|ui| {
      ui.heading("Status Register (SR)");
      ui.label("16-bit register combining System Byte and CCR:");
      ui.monospace(" • Bit 13 [S]: Supervisor Mode (1=Supervisor, 0=User)");
      ui.monospace(" • Bits 8-10 [IPL]: Interrupt Priority Level Mask (0-7)");
  });
  ```

---

## 9. Reference Documentation & Upstream Ground Truth

- [egui Best Practices Rule](../../../.agents/rules/egui-best-practices.md): Mandatory immediate-mode UI rules and headless testing policies.
- [GUI Frontend Architecture & Overview](GUI.md): Display scaling, CRT shaders, and audio ring buffers.
- [GUI Detailed Layout & Panel Specification](GUI%20Specification.md): Panel layouts, geometries, and interactive editor specs.
- [GUI Crate Implementation](../../../crates/gui/src/lib.rs): Living Rust implementation of eframe app, views, docks, and modals.
- [GUI Interaction Test Suite](../../../crates/gui/tests/test_interactions.rs): Headless integration tests validating UI layout, keyboard events, and theme toggling.


