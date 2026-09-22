---
trigger: model_decision
description: Immediate-mode frontend guidelines for egui, direct state pulling, and headless integration testing in crates/gui.
---

# egui & Immediate-Mode Frontend Best Practices

All user interface code in the Amiga 500 emulator (`crates/desktop_gui`) must strictly adhere to these immediate-mode guidelines.

---

## 1. Architectural Model & Concurrency
- **Synchronous Direct-State Pull:** Panels directly query emulator state (`&CpuState`, `&mut MemoryBus`, `&mut Debugger`) on render. The emulator core never "notifies" or sends events/callbacks to the UI.
- **Zero Asynchrony / Zero Message Passing:** No channels (`mpsc`), no async runtimes (`tokio`), and no observer patterns. Execution and UI rendering proceed in a synchronous, deterministic loop. Mechanically enforced via `clippy::disallowed_types` (`mpsc::Sender`, `mpsc::Receiver`, `Arc`, `Mutex`, `RwLock`) and `clippy::disallowed_methods` (`std::thread::spawn`).
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

## 5. Mandatory Automated Integration Testing (`crates/gui/tests/`)
- **Simulated Headless Input Passes:** Every user interaction, keyboard shortcut, drag-and-drop, inline editing lifecycle, modal dialog, and time-travel navigation must be accompanied by automated integration tests using `egui::Context::default()` and `ctx.run(RawInput, |ctx| { app.update_ui(ctx); })`.
- **Zero Graphical Regressions Mandate:** Whenever a graphical bug, misalignment, or interaction defect is reported or fixed:
  1. Analyze whether the defect or expected layout state can be asserted headlessly.
  2. Implement an automated integration test in `crates/gui/tests/test_interactions.rs` or `test_gui.rs` verifying the expected behavior (e.g. text edit focus acquisition, click-outside dismissal, Escape cancellation, grid slot geometry stability).
  3. No interactive UI bugfix is complete without passing automated integration tests.

---

## 6. Self-Documenting UI & In-App Contextual Documentation Standard

The emulator frontend serves not only as an execution viewer, but as an **interactive, self-documenting Amiga hardware and software studio**. Every developer or retro enthusiast must be able to explore and understand the emulated system directly within the application without needing to switch out to external hardware reference manuals (Amiga Hardware Reference Manual, Motorola 68000 PRM, Guru Book).

### A. Universal Hover Documentation Mandate
- **100% Hover Coverage:** Every inspectable register, status bit, custom chip control field, memory range, opcode mnemonic, toolbar action, and timeline control across **all screens, panels, and modals** must expose comprehensive, self-contained documentation via `.on_hover_ui` or `.on_hover_text`.
- **Zero External Lookup Requirement:** The in-app documentation must provide all necessary context:
  - **CPU Registers ($D_0-D_7, A_0-A_7, PC, SR, USP, SSP$):** Bit widths, byte/word/long access rules, signed vs unsigned interpretations, active privilege state, prefetch pipeline progression ($IR$/$IRC$), and supervisor restrictions.
  - **Condition Code Register ($CCR$ Flags):** Mathematical criteria for setting and clearing $X, N, Z, V, C$ flags and their effects on conditional branches ($Bcc$, $DBcc$, $Scc$).
  - **Status Register ($SR$ Bits):** Complete bitfield breakdown of the System Byte (Trace mode $T$, Supervisor state $S$, and Interrupt Priority Level mask $I_0-I_2$).
  - **Microcode & Color Clock Phases:** Concrete explanations of $CCK1$ (address/bus drive) vs $CCK2$ (data latch / ALU evaluation), dual staging registers (`addr1`, `addr2`), and Chip RAM DMA arbitration wait states.
  - **Memory Map & Addresses:** Region identity (Chip RAM, Extended Chip RAM, Fast RAM, Slow RAM, CIA-A/B spaces, Custom Chip register space `$DFF000..=$DFFFFF`, Kickstart ROM), mirror spaces, and alignment constraints.
  - **Toolbar & Timeline Controls:** Explicit keyboard shortcuts, cycle deltas, and stepping semantics (Step Instruction vs Step CCK vs Temporal Rewind).

### B. Structured Contextual Layouts (`.on_hover_ui`)
- For multi-field or complex hardware structures (such as the Status Register, condition code flags, or custom chip registers), use structured `.on_hover_ui` layouts featuring bold headings, monospace bit diagrams, and concise bullet points rather than truncated single-line strings.
- **Zero Heap Allocation in Tooltip Generators:** Tooltip text and layouts must use static string slices (`&'static str`) or pre-formatted stack buffers to maintain zero dynamic heap allocations during draw passes.

---

## 7. Execution Skill: `egui-vision-debugger`

For visual layout audits, responsive resizing tests (1024x600, 800x600), splitter hover verification, and offscreen screenshot diagnostics, follow the operational recipe in [`egui-vision-debugger`](../skills/egui-vision-debugger/SKILL.md).

---

## 8. Authoritative Frontend Design Specifications & Delegation

When designing GUI layouts, panel hierarchies, or debugger widgets, agents must adhere to:
- [`GUI.md`](../../Obsidian/Amiga/Design/GUI.md): High-level Developer Studio architecture and component model.
- [`GUI Specification.md`](../../Obsidian/Amiga/Design/GUI%20Specification.md): Panel layout dimensions, view modes (`Developer` vs `ScreenOnly`), and docking behavior.
- [`egui Guidelines.md`](../../Obsidian/Amiga/Design/egui%20Guidelines.md): Immediate-mode styling, zero-allocation rendering, and input event routing.
- [`Debugger.md`](../../Obsidian/Amiga/Design/Debugger.md): Disassembly inspection, breakpoint evaluation, and register editing interfaces.
