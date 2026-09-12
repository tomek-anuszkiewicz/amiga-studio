# egui & Immediate-Mode Frontend Best Practices

All user interface code in the Amiga 500 emulator (`crates/desktop_gui`) must strictly adhere to these immediate-mode guidelines.

---

## 1. Architectural Model & Concurrency
- **Synchronous Direct-State Pull:** Panels directly query emulator state (`&CpuState`, `&mut MemoryBus`, `&mut Debugger`) on render. The emulator core never "notifies" or sends events/callbacks to the UI.
- **Zero Asynchrony / Zero Message Passing:** No channels (`mpsc`), no async runtimes (`tokio`), and no observer patterns. Execution and UI rendering proceed in a synchronous, deterministic loop.
- **Single-Threaded Time-Slicing:** When the emulator is running, execute a bounded slice of cycles (e.g. 1/60th second of PAL CCKs or up to 10,000 instructions) per GUI frame. This keeps the window 100% responsive and enables seamless compilation to WebAssembly (`wasm32-unknown-unknown`).

---

## 2. Visual-to-Source 1:1 Layout Mapping
Structure all files under `crates/desktop_gui/src/layout/` to strictly mirror what is visible on the screen:
- `top_menu_bar.rs`: Top toolbar (file loading, run/pause/step, theme, zoom).
- `main_viewport/amiga_screen.rs`: 4:3 centered Amiga CRT display container.
- `main_viewport/temporal_bar.rs`: Timeline scrubber and rewind slider below screen.
- `left_dock/registers.rs`: Live D0-D7, A0-A7, PC, SR, and CCR LED badges.
- `left_dock/microcode.rs`: Micro-step index, active ALU op, staging registers.
- `left_dock/disassembly.rs`: Disassembly stream and breakpoint margin.
- `right_dock/memory_hex.rs`: Hex + ASCII editor with chunk quick-jump buttons.
- `right_dock/memory_search.rs`: Hex/ASCII pattern search bar.
- `right_dock/trace_log.rs`: 1024-entry execution history log table.

---

## 3. High-DPI, Browser Zoom & Themes
- **Adaptive DPI & Zoom:** Rely on `eframe`'s automatic OS DPI scaling (`pixels_per_point`) and browser `window.devicePixelRatio`. Do not use hardcoded pixel coordinates; use relative layouts and points.
- **Aspect-Locked Viewport:** The Amiga display viewport ($320 \times 256$) must maintain strict 4:3 aspect ratio framing with integer prescaling (1x, 2x, 3x).
- **Theme Support:** Support dark theme by default, with auto-detection of host OS / browser preference (`prefers-color-scheme`) and runtime switching.

---

## 4. Render Loop Performance
- **Zero Allocations in Render Loops:** Avoid allocating `Vec`s or formatting strings in inner loops of tables and grids. Use stack buffers or pre-formatted labels.
- **Virtual Scrolling:** For large memory views (512 KB) and trace logs, always use `egui::ScrollArea::show_rows` to only compute and render rows currently visible in the viewport.

---

## 5. Mandatory Automated Integration Testing & Invariants (`crates/gui/tests/`)
- **Simulated Headless Input Passes:** Every user interaction, keyboard shortcut, drag-and-drop, inline editing lifecycle, modal dialog, and time-travel navigation must be accompanied by automated integration tests using `egui::Context::default()` and `ctx.run(RawInput, |ctx| { app.update_ui(ctx); })`.
- **Zero Graphical Regressions Mandate:** Whenever a graphical bug, misalignment, or interaction defect is reported or fixed:
  1. Analyze whether the defect or its invariant can be asserted headlessly.
  2. Implement an automated integration test in `crates/gui/tests/test_interactions.rs` or `test_gui.rs` verifying the expected behavior (e.g. text edit focus acquisition, click-outside dismissal, Escape cancellation, grid slot geometry invariance).
  3. No interactive UI bugfix is complete without passing automated integration tests.

