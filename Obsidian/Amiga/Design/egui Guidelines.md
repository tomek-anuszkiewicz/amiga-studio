# Amiga 500 egui & Frontend Best Practices

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
- `top_menu_bar.rs`: Top toolbar (file loader, execution controls, theme, zoom).
- `main_viewport/`: Centered primary workspace (Amiga CRT canvas, temporal rewind scrubber).
- `left_dock/`: CPU and execution inspection (registers, microcode state, disassembly).
- `right_dock/`: Memory inspection and history (memory hex editor, search, trace log).

Each `.rs` file corresponds to a single, visually distinct dock or panel on the screen.

---

## 5. High-DPI Scaling & Browser Zoom Adaptation

Interfaces must look sharp and properly proportioned across monitors and browser viewports:
- **Desktop High-DPI:** `eframe` automatically detects OS scale factors (100%–200%) and sets `egui::Context::set_pixels_per_point`. Do not hardcode absolute pixel sizes for text or UI elements; use relative units (`em`, point sizes, and layout proportions).
- **WebAssembly Zoom:** `eframe::WebRunner` automatically binds to `window.devicePixelRatio`. Browser zoom actions (`Ctrl +` / `Ctrl -`) scale the vector UI automatically.
- **Retro Pixel Viewport Isolation:** The 4:3 Amiga display viewport ($320 \times 256$) uses explicit integer prescaling (1x, 2x, 3x) within its container, ensuring vintage pixel graphics remain crisp and undistorted while tool panels scale with the host display.

---

## 6. Theme & Visual Styling

- **Unified Palette:** Follow a curated, low-eyestrain developer dark theme by default, with automatic fallback to host OS preference.
- **Visual Feedback & State Diffing:**
  - When stepping through code, highlight mutated registers and memory cells with a distinct accent color (e.g. cyan/yellow) that fades back to baseline text.
  - Active condition code flags (CCR) should be rendered as glowing LED badges (green = 1, dark gray = 0) with clickable toggle states.
  - Breakpoints should render as distinct colored markers (e.g. red dot `●`) in the disassembly margin.

---

## 7. Zero-Allocation UI Drawing

Because `egui` draws every widget on every frame:
- **Avoid Heap Allocations in Render Loops:** Do not allocate new `Vec`s or perform heavy `format!` concatenations inside inner loops (such as memory hex tables with thousands of bytes).
- **Reusable Format Buffers & Slices:** Use fixed-size stack buffers (`[u8; N]`) or pre-formatted static labels where possible.
- **Lazy/Virtual Scrolling:** In large views (e.g. 512 KB memory hex grid or 1024-entry trace history), use `egui::ScrollArea::show_rows` to render only the rows currently visible on screen.
