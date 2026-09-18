---
title: "Amiga 500 Developer GUI & Debugger Layout Specification"
aliases: ["Developer GUI Layout", "Debugger Layout", "GUI Docking"]
tags: ["amiga", "design", "gui", "layout", "egui", "docking"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-10
updated: 2026-09-16
related: ["[GUI.md](GUI.md)", "[egui Guidelines.md](egui%20Guidelines.md)", "[Debugger.md](Debugger.md)", "[General Architecture.md](General%20Architecture.md)", "[Rust Guidelines.md](Rust%20Guidelines.md)"]
---

# Amiga 500 Developer GUI & Debugger Layout Specification

- **Parent Specification:** [GUI.md](GUI.md) | [General Architecture.md](General%20Architecture.md)
- **Debugger Engine Companion:** [Debugger.md](Debugger.md)
- **Frontend Guidelines:** [egui Guidelines.md](egui%20Guidelines.md) | [Rust Guidelines.md](Rust%20Guidelines.md)
- **Module Location:** `crates/gui/`
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md).

> [!NOTE]
> Living architectural specification for the Amiga 500 Developer GUI & Interactive Debugger Studio implemented under [`crates/gui`](../../../crates/gui).
> For high-level frontend architecture, CRT shaders, and audio sinks, see [GUI.md](GUI.md).

---

## 1. Visual Layout & 1:1 Source Code Directory Mapping

The physical arrangement on screen strictly maps 1:1 to the Rust source code hierarchy under `crates/gui/src/`:

```
+-------------------------------------------------------------------------------------------------------------------+
| Top Menu Bar (layout/top_menu_bar.rs)                                                                             |
+-------------------+-----------------------------------+---------------------------+-------------------------------+
| Left Dock: CPU    | Center: Main Viewport             | Column 3: Disassembly     | Right Dock: Memory & Tools    |
|                   |                                   |                           |                               |
| 1. Registers Panel| 1. Amiga Screen (320x256 4:3 CRT) | 1. Jump & Goto Bar        | 1. Memory Hex Editor          |
|    (registers.rs) |    (main_viewport/                | 2. Full-Height Disassembly|    (right_dock/memory_hex.rs) |
|    - D0-D7 & A0-A7|     amiga_screen.rs)              |    Stream (auto-height)   |                               |
|    - 2-Col Status |                                   | 3. Breakpoint Margins (●) | 2. Memory Search Bar          |
|      & CCR Flags  | 2. Temporal Rewind Scrubber       | 4. In-Place Assembler (✏) |    (right_dock/               |
|                   |    (main_viewport/                |    (left_dock/            |     memory_search.rs)         |
| 2. Microcode      |     temporal_bar.rs)              |     disassembly.rs)       |                               |
|    Inspector      |                                   |                           | 3. Breakpoints & Watches      |
|    (microcode.rs) |                                   |                           |    (breakpoints_panel.rs)     |
|    - 4-Col Staging|                                   |                           |                               |
|    - Bus State    |                                   |                           | 4. Trace History Log          |
|                   |                                   |                           |    (right_dock/trace_log.rs)  |
+-------------------+-----------------------------------+---------------------------+-------------------------------+
```

### Living Source File Tree & Implementation Links:
- [`crates/gui/Cargo.toml`](../../../crates/gui/Cargo.toml): Package configuration and UI dependencies (`eframe`, `egui`, `rfd`, `m68000`, `physical_memory`, `debugger`, `config`, `rtc`).
- [`Trunk.toml`](../../../Trunk.toml): Root Trunk configuration enabling `trunk serve --open` and `trunk build` directly from workspace root.
- [`crates/gui/index.html`](../../../crates/gui/index.html): HTML5 canvas runner template for WebAssembly browser execution.
- [`crates/gui/src/gui.rs`](../../../crates/gui/src/gui.rs): WebAssembly entrypoint (`eframe::WebRunner`) and shared re-exports.
- [`crates/gui/src/main.rs`](../../../crates/gui/src/main.rs): Desktop native entrypoint (`eframe::run_native`) with 1600x840 default viewport (optimized for 16:9 displays at 100-125% DPI scaling).
- [`crates/gui/src/app.rs`](../../../crates/gui/src/app.rs): Central `EmulatorApp` orchestrator, 4-column dock orchestration, bounded time-slicing execution, and event loop.
- [`crates/gui/src/theme.rs`](../../../crates/gui/src/theme.rs): Dark, Light, and Classic Amiga Workbench color palettes.
- [`crates/debugger/src/loader.rs`](../../../crates/debugger/src/loader.rs): Raw binary injection, boot overlay disengagement, and prefetch priming.
- [`crates/debugger/src/temporal.rs`](../../../crates/debugger/src/temporal.rs): High-capacity (250,000-frame, >=1.0s PAL) circular time-travel execution history ring buffer.
- [`crates/gui/src/layout/top_menu_bar.rs`](../../../crates/gui/src/layout/top_menu_bar.rs): [TOP] File loading, cold/warm reset, run/pause, step controls, telemetry, theme, and zoom.
- [`crates/gui/src/layout/main_viewport/amiga_screen.rs`](../../../crates/gui/src/layout/main_viewport/amiga_screen.rs): [CENTER] 4:3 aspect-locked CRT monitor canvas ($320 \times 256$) with retro bezel.
- [`crates/gui/src/layout/main_viewport/temporal_bar.rs`](../../../crates/gui/src/layout/main_viewport/temporal_bar.rs): [CENTER-BOTTOM] History scrubber slider, timing/CCK deltas, multi-granularity navigation (`-10`, `+10`, `-1 Frame`, `+1 Frame`), record toggle, and direct cycle jump.
- [`crates/gui/src/layout/left_dock/registers.rs`](../../../crates/gui/src/layout/left_dock/registers.rs): [LEFT-TOP] Live D0-D7, A0-A7, USP/SSP, 2-column Execution Status & CCR clickable LED badges, and diff highlighting.
- [`crates/gui/src/layout/left_dock/microcode.rs`](../../../crates/gui/src/layout/left_dock/microcode.rs): [LEFT-BOT] Compact Micro-step counter ($K / N$), CCK1/CCK2 phase badge, dense 4-column staging registers (`addr1`, `addr2`, `ea_addr`), and bus lock state.
- [`crates/gui/src/layout/left_dock/disassembly.rs`](../../../crates/gui/src/layout/left_dock/disassembly.rs): [COL 3] Dedicated full-height Disassembly listing, luminous execution cursor highlight, breakpoint margin toggles (`●`), double-click in-place assembler editor, right-click options menu, and quick loop rewind (`⏪`).
- [`crates/gui/src/layout/right_dock/memory_hex.rs`](../../../crates/gui/src/layout/right_dock/memory_hex.rs): [RIGHT-TOP] 16-byte hex + ASCII editor with virtual scrolling and fast chunk quick-jump buttons.
- [`crates/gui/src/layout/right_dock/breakpoints_panel.rs`](../../../crates/gui/src/layout/right_dock/breakpoints_panel.rs): [RIGHT-MID2] Interactive PC execution breakpoints, conditional expressions, and memory watchpoints manager.
- [`crates/gui/src/layout/right_dock/trace_log.rs`](../../../crates/gui/src/layout/right_dock/trace_log.rs): [RIGHT-BOT] 1024-entry execution trace table with zero-allocation virtual scrolling.
- [`crates/gui/tests/test_gui.rs`](../../../crates/gui/tests/test_gui.rs): Headless integration and unit test suite.

---

## 2. Execution Model: Synchronous Direct-State Pull

The GUI and the emulator core interact through a strictly **synchronous, pull-based model**:
1. **Direct References:** The top-level `EmulatorApp` struct owns `Cpu`, `MemoryBus`, and `Debugger`. During every `egui::App::update` pass, panels read directly from `&self.cpu.state`, `&self.bus`, and `&self.debugger`.
2. **Zero Asynchrony & Zero Callbacks:** There are no background threads for the core in the baseline, no `mpsc` message queues, and no observer pattern notifications. The emulator core never "pushes" change events to the GUI.
3. **Single-Threaded Bounded Time-Slicing:**
   - When free-running (`F5`), `app.rs` executes a bounded slice of up to `MAX_INSTRUCTIONS_PER_FRAME = 5000` instructions per GUI frame via `debugger.run_until_breakpoint_with_temporal(&mut cpu, &mut bus, &mut temporal, 5000)`.
   - The host window never hangs, and repaints are requested synchronously (`ctx.request_repaint()`).
4. **Synchronous Mutation on Interaction:**
   - When stepping (`F10`), `debugger.step_instruction(&mut cpu, &mut bus)` executes immediately, appends to `temporal`, advances `bus.step_cck(clocks / 2)` to synchronize RTC/timers, and redraws immediately.
   - When editing memory, `bus.write_byte_debug(addr, val)` updates the physical RAM buffer instantly.
   - When loading binary code, [`loader::inject_binary`](../../../crates/debugger/src/loader.rs) disengages Kickstart low-memory boot overlay (`bus.map_chip_ram_to_low_memory()`), initializes SP to top of 512 KB Chip RAM ($080000) if unset, and primes prefetch with `cpu.set_pc_and_prime_prefetch(target_pc, bus)`.

---

## 3. Universal In-App Documentation Standard (Zero External Lookup)

The Developer Studio is designed as a **self-contained Amiga hardware encyclopedia**. Every interactive and inspectable UI element must provide rich contextual documentation upon hover, removing any need to switch to external PDF reference manuals (Amiga Hardware Reference Manual, Motorola 68000 PRM):

1. **Hover Universality:** 100% of labels, registers, flags, memory cells, and controls must implement `.on_hover_ui` or `.on_hover_text`.
2. **Context Depth:**
   - **Registers ($D_0-D_7, A_0-A_7$):** Monospace bit layouts, signed/unsigned decimal conversions, word/long access semantics, and supervisor privilege restrictions.
   - **Execution Status & CCR:** Complete mathematical truth tables for $X, N, Z, V, C$ and full bitfield breakdown of the Status Register (Trace mode, Supervisor mode, and IPL interrupt mask).
   - **Microcode & Bus Phases:** Physical circuit definitions for $CCK1$ (address drive) and $CCK2$ (data latch / ALU step), staging register identities (`addr1`, `addr2`), and Chip RAM wait state causes.
   - **Disassembly & Addressing Modes:** Mnemonic descriptions, cycle costs, effective address formulas (e.g. `d16(An, Xi)`), and branch targets.
   - **Memory Map & Regions:** Physical boundary identifications (Chip RAM, Slow RAM, Fast RAM, CIA spaces, Custom Chip register space `$DFF000..=$DFFFFF`, Kickstart ROM).
   - **Temporal History & Timeline:** Step deltas, millisecond offsets, and exact keyboard shortcut references.
3. **Zero Heap Allocation:** Tooltips use compile-time static strings (`&'static str`) or stack formatting, lazily evaluated only when hovered.

---

## 4. Panel-by-Panel Specification

### 3.1 Top Menu Bar ([`layout/top_menu_bar.rs`](../../../crates/gui/src/layout/top_menu_bar.rs))
- **File Actions:**
  - `Load Binary...` (`Ctrl+O`): Opens file dialog (`rfd`). Prompts for Target Address (defaults to `$001000`) and a checkbox `Auto-set PC & prime prefetch`. Injects binary slice directly into RAM.
  - `Reset Cold` (`Ctrl+R`): Cold-resets CPU, clears RAM via `bus.reset_cold()`, and resets temporal trace buffers.
  - `Reset Warm`: Preserves RAM and re-engages Kickstart overlay via `bus.reset_warm()`.
- **Execution Controls:**
  - `▶ Run` / `⏸ Pause` (`F5` or `Space`): Toggles continuous emulation. While running, executes up to 5,000 instructions per frame without hanging the UI.
  - `⏭ Step Inst` (`F10`): Executes exactly one completed M68000 instruction, records temporal frame, and halts.
  - `⏯ Step CCK` (`F11`): Advances exactly 1 Color Clock phase (2 CPU clocks), advancing microcode state.
  - `⏮ Rewind` (`Shift+F10`): Restores the previous execution frame from the temporal history buffer.
  - `🎮 Game View (F12)`: Switches from Developer Studio into Clean Screen Mode.
- **View Menu:**
  - `Show Microcode Inspector (F8)`: Toggles microcode panel visibility in Left Dock.
- **Status Badges:**
  - Emulation state: `[RUNNING - Green]` / `[PAUSED - Yellow]` / `[HALTED - Red]` / `[STOPPED - Blue]`.
  - Machine Metrics: Current CCK count, simulated MHz, instructions executed.
- **Preferences:**
  - Theme Selector: `Dark (Default)`, `Light`, `Classic Workbench`.
  - Zoom Selector: `100%`, `125%`, `150%`, `200%`.

### 3.2 Main Viewport: Amiga Screen ([`layout/main_viewport/amiga_screen.rs`](../../../crates/gui/src/layout/main_viewport/amiga_screen.rs))
- **Display Container:**
  - Strict 4:3 aspect ratio framing centered in the panel.
  - Canvas resolution: $320 \times 256$ (PAL baseline) / $320 \times 200$ (NTSC).
- **Dual Display Modes:**
  - **Developer Viewport:** Nested within the center panel with temporal history scrubber below.
  - **Clean Standalone Screen (`ViewMode::ScreenOnly`):** Full-window distraction-free display without side docks or floating overlays. Pressing `F12` cleanly returns to Developer Studio.
- **Initial Standby State:**
  - Renders an authentic CRT phosphor standby display with retro Commodore Amiga copy:
    - `"Commodore Amiga 500"`
    - `"PAL — 50 Hz (320 × 256)"`
    - `"Press F12 for Developer Studio / Debugger"`
- **Scaling:** Crisp integer prescaling (1x, 2x, 3x) based on available panel geometry.

### 3.3 Main Viewport: Temporal Rewind Scrubber ([`layout/main_viewport/temporal_bar.rs`](../../../crates/gui/src/layout/main_viewport/temporal_bar.rs))
- Located immediately below the Amiga display viewport.
- **Lean Circular Buffer (Default 25,000 frames / ~7.5 MB):**
  - Starts **inactive / paused by default** (`is_recording == false`) to eliminate startup CPU/memory overhead.
  - Dynamically resizable via capacity presets (`5k`, `10k`, `25k`, `50k`, `100k`, `250k`).
- **Recording Active Toggle:**
  - Button `[ ⏺ Rec: ON ]` (green) vs `[ ⏸ Rec: OFF ]` (amber) / shortcut `Alt+T` to pause/resume recording on the fly without losing current buffer history.
- **Multi-Granularity Navigation:**
  - `[ ⏮ First ]`: Jumps to the earliest captured snapshot.
  - `[ ◀◀ Frame ]`: Jumps backward by 1 PAL video frame (~70,824 CCKs / 20ms).
  - `[ -10 ]`: Rewinds 10 instructions.
  - `[ ◀ -1 ]` (`Shift+F10`): Restores previous instruction.
  - `[ +1 ▶ ]`: Steps forward 1 instruction toward head.
  - `[ +10 ]`: Advances 10 instructions.
  - `[ Frame ▶▶ ]`: Advances forward by 1 video frame.
  - `[ Live Head ⏭ ]`: Returns immediately to live execution.
- **Direct Cycle Jump & Timing Metrics:**
  - Input field `[ Jump to CCK: #_______ ] [ Go ]` for jumping directly to any exact processor clock cycle.
  - Slider displaying `[Index / Total]`, relative millisecond offset (e.g. `-245.3 ms`), and CCK deltas.

### 3.4 Left Dock: CPU Registers ([`layout/left_dock/registers.rs`](../../../crates/gui/src/layout/left_dock/registers.rs))
- **Data Registers ($D_0 - D_7$):**
  - Displays each register in hexadecimal (`$00000000`) and signed decimal via public accessor `state.d_regs()`.
  - **Interactive Inline Editing:** Clicking any register value switches to an inline text box with immediate focus. Pressing Enter or clicking outside commits; Escape cancels.
  - **Educational Tooltips:** Rich `.on_hover_ui` detailing bitwidths, byte/word/long access rules, and signed interpretation.
  - **Diff Highlighting:** If a register value changed in the last execution step, it renders with a glowing pill background in electric cyan (`#67E8F9` text with `#083344` pill background per `ColorTokens`).
- **Address Registers ($A_0 - A_7$):**
  - Displays $A_0 - A_6$ and the active stack pointer $A_7$ (`USP` vs `SSP`). Word alignment rules explained in contextual tooltips.
- **Program Counter ($PC$) Normalization & Prefetch:**
  - Normalized to active instruction PC (`state.pc - 4`), matching the Disassembly pointer 1:1.
  - Prefetch reload on manual PC change via `cpu.set_pc_and_prime_prefetch(new_pc, bus)`.
- **Status Register ($SR$) & CCR LED Badges:**
  - Supervisor mode badge (`[SUPERVISOR]` vs `[USER]`), IPL mask (0-7).
  - Interactive high-contrast LED badges for $X, N, Z, V, C$ (emerald `#059669` fill with white text for active; dark slate `#2A3241` fill with `#E2E8F0` text for inactive per `ColorTokens`). Clicking toggles bits in `cpu.state.sr`.

### 3.5 Left Dock: Microcode State Inspector ([`layout/left_dock/microcode.rs`](../../../crates/gui/src/layout/left_dock/microcode.rs))
- **Collapsible Card:** Header `▼ Microcode Inspector` toggleable locally or globally via `F8`.
- **Live Cycle & Timing Metrics:**
  - Active micro-step remaining clocks (`state.micro.clocks_remaining` / `base_clocks`).
  - Total remaining clock cycles and Color Clocks (CCK) until instruction retirement.
  - Format: `Step rem: X clk | Total rem: Y clk (Z CCK)`.
- **Phase & Staging Visualization:**
  - Current Color Clock phase badge (`[CCK1]` vs `[CCK2]`) with educational bus drive/latch tooltips.
  - Compact 4-column staging registers: `addr1` ($X_1$), `addr2` ($X_2$), `source`, `destination`, `ea_addr`.
  - Chip RAM DMA arbitration: indicates whether bus is free or blocked by Agnus/Denise DMA with wait states.

### 3.6 Column 3: Disassembly View ([`layout/left_dock/disassembly.rs`](../../../crates/gui/src/layout/left_dock/disassembly.rs))
- **Live Disassembly Table & Infinite Scroll:**
  - Full-height layout without nested `ScrollArea` wrappers: calculates visible rows directly from available height (`(ui.available_height() / 19.0).floor() as usize`).
  - Continuous memory browsing: Mouse-wheel scrolling up or down freely streams disassembly across 24-bit memory space without snapping back to PC, maintaining valid 16-bit instruction alignment. Supports Ctrl (10x) and Shift (5x) acceleration and consumes scroll deltas to prevent parent container fighting.
  - Interactive 24-bit vertical scrollbar (8px width) on right edge for instantaneous navigation across full 16 MB 68000 address space.
  - Active PC Synchronization: Executing any step (`F10`, `Shift+F10`, `F11`), running (`F5`), or clicking top bar execution buttons automatically resets the view override (`disassembly_view_addr = None`), re-centering on current $PC$.
  - Single-row scoped execution pointer highlight rect in electric cyan (`accent_pc`).
  - Breakpoint margin: Vector-painted resolution-independent circular breakpoint indicators (`circle_filled` / `circle_stroke`), eliminating OS missing-glyph boxes. Clicking toggles PC breakpoint.
- **Row Selection & Interaction:**
  - Single-clicking any row selects the instruction, applying an accent border outline and tinted fill (`selected_addr`).
  - Double-clicking opens the in-place inline assembler editor with clean styled text buttons (`Save`, `Cancel`).
- **Time-Travel Navigation & Loop Rewind:**
  - **Right-Click Context Menu:**
    - `📍 Set PC here`
    - `● Toggle Breakpoint`
    - `✏ Edit Instruction`
    - `⏪ Rewind to Pass #N (CCK: X)` for each historical execution pass detected by `find_matches_by_pc`.
  - **Quick Rewind Button (`⏪`):** Appears next to instructions with execution history; clicking immediately scrubs temporal history to the most recent pass.
- **In-Place Instruction Editing with Byte Size Invariance:**
  - Inline editor matches row height with zero column shift. Rejects writes if replacement size differs from original, preserving memory alignment.

### 3.7 Right Dock: Memory Hex Editor ([`layout/right_dock/memory_hex.rs`](../../../crates/gui/src/layout/right_dock/memory_hex.rs))
- **16-Byte Row Hex + ASCII Grid:**
  - 510px default dock width with uniform 19px fixed-width cell slot allocations (`allocate_ui_with_layout`).
  - **Dynamic Fill & Automatic Bottom-Docking:** Memory Hex Editor dynamically docks to the top and automatically expands to fill 100% of all available vertical height above bottom-docked tool panels via natural bottom-up layout (`Layout::bottom_up`), displaying 35–48 rows on Full HD displays ($000000..=$000220+) with in-place memory scrolling.
  - Zero margin on byte input editor ensures typing never causes horizontal jitter or shifts adjacent columns.
  - 14px comfortable right margin on row end prevents right-edge border collision on ASCII column.
- **Cell Selection vs Inline Editing:**
  - Single-clicking any hex cell or ASCII glyph selects that exact byte address (`selected_addr = Some(addr)`) with an accent border and tinted fill without opening the text input box.
  - Double-clicking or pressing `Enter` on a selected cell activates inline hex editing with direct auto-focus.
  - Pressing `Escape` or clicking outside dismisses editing or clears selection cleanly.
- **Row-Wrapping Keyboard Navigation:**
  - `ArrowLeft` wraps from column 0 to column 15 of the preceding row.
  - `ArrowRight` wraps from column 15 to column 0 of the succeeding row.
  - `ArrowUp` / `ArrowDown` step backward/forward by 16 bytes.
  - `Home` / `End` jump to column 0 / 15 of the current row.
  - `PageUp` / `PageDown` navigate by visible page capacity (or 1 KB with Ctrl held).
  - Navigating past the top or bottom visible row automatically scrolls `base_addr`.
- **Permanent Fixed Margins & Persistent Layout Configuration:**
  - **Uniform 8px Divider & Window Boundaries:** Every panel frame enforces exact symmetric spacing (`Margin { left: 8, right: 4 }` for Left Dock, `Margin { left: 4, right: 4 }` for Central Panel, and `Margin { left: 4, right: 8 }` for Right Dock, creating an exact 8px spacing across all panel dividers and outer window edges).
  - **Flush Disassembly Stream:** Disassembly rows and 24-bit scrollbar stretch flush right up to the vertical splitter line with zero dead padding.
  - **Symmetric Right Dock Alignment:** Bottom tools (`Memory Search` and `Breakpoints & Watchpoints`) align flush with Memory Hex Editor with zero lopsided indentations (`ui.spacing_mut().indent = 0.0`).
  - **Column 2 / Column 3 Vertical Splitter (Full HD Mode):** 100% full-height vertical divider (`ui.max_rect().y_range()`) with `ResizeHorizontal` cursor between Disassembly and CRT Screen / Trace column, stored in `disasm_pane_width` (default 460px, clamped 280px..=total_w - 380px).
  - **CRT Screen / Trace Log Horizontal Splitter (Full HD Mode):** Draggable horizontal divider with `ResizeVertical` cursor separating top CRT screen/temporal bar from lower Trace Log panel, stored in `crt_pane_height` (default 380px, clamped 200px..=total_h - 100px).
  - **Zero Horizontal Splitter in Right Dock:** The manual draggable splitter has been removed in favor of automatic bottom-docking and reactive top hex editor expansion.
  - Active dimensions are persisted across application restarts via `UserPreferences` in `eframe::Storage`.

### 3.8 Right Dock: Memory Search ([`layout/right_dock/memory_search.rs`](../../../crates/gui/src/layout/right_dock/memory_search.rs))
- Pattern searching across memory with full-width stretching (`ui.set_width(ui.available_width())`):
  - Hex mode: e.g. `4E 71 32 00`.
  - ASCII mode: e.g. `DOS\0`, `Kickstart`.
  - Stretched singleline text input with right-aligned `[🔍 Find Next]` button eliminating any right-side gap.
  - Jump-to-match buttons (`Prev Match`, `Next Match`) that synchronize the Hex Editor view.

### 3.9 Right Dock: Breakpoints & Watchpoints Manager ([`layout/right_dock/breakpoints_panel.rs`](../../../crates/gui/src/layout/right_dock/breakpoints_panel.rs))
- Full-width card containers (`ui.set_width(ui.available_width())`) docked at the bottom of Right Dock:
  - **PC Breakpoints:** Active breakpoint list with enable checkboxes, address formatting, and right-aligned delete `[✕]` buttons.
  - Attached register conditions: e.g. `[IF D0 == $2A]`, `[IF PC == $1004]`.
  - Inline creator to specify PC address, register, operator (`==, !=, <, >, <=, >=`), and value.
  - **Memory Watchpoints:** Address range monitor with access badges (`READ`, `WRITE`, `ANY`) and right-aligned delete `[✕]` buttons.
  - Enable checkboxes and remove buttons.

### 3.10 Right Dock: Trace History Log ([`layout/right_dock/trace_log.rs`](../../../crates/gui/src/layout/right_dock/trace_log.rs))
- Fixed 1024-entry execution trace table backed by `debugger::TraceRingBuffer`.
- Chronological table showing Cycle (CCK), PC, Opcode, Disassembly, and Register state.
- Clicking any historical entry synchronizes the temporal debugger to that moment.

---

## 5. Semantic Design Tokens & Theme Palettes (`ColorTokens`)

To guarantee that the visual presentation can be reconstructed from specifications alone with 100% fidelity, all UI surfaces, typography, accents, and condition badges are governed by centralized immutable design tokens implemented in [`crates/gui/src/theme/tokens.rs`](../../../crates/gui/src/theme/tokens.rs).

### 5.1 Palette Matrix & Exact Color Definitions

| Semantic Token | Rust Identifier | Dark (Default) | Light | Classic Workbench 1.3 | Description / UI Usage |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Canvas Background** | `canvas_bg` | `#0E1117` `(14, 17, 23)` | `#F5F7FA` `(245, 247, 250)` | `#003C7D` `(0, 60, 125)` | Root background behind docked panels |
| **Panel Background** | `panel_bg` | `#161B22` `(22, 27, 34)` | `#EEF2F6` `(238, 242, 246)` | `#0055AA` `(0, 85, 170)` | Side docks, menu bars, and inner container fill |
| **Card Background** | `card_bg` | `#1E2430` `(30, 36, 48)` | `#E4E9F0` `(228, 233, 240)` | `#00468C` `(0, 70, 140)` | Sub-cards, grouped headers, and edit backgrounds |
| **Subtle Border** | `border_subtle` | `#2E3646` `(46, 54, 70)` | `#CBD5E1` `(203, 213, 225)` | `#006ED7` `(0, 110, 215)` | Inactive cell borders, separators, and card strokes |
| **Active Border** | `border_active` | `#38BDF8` `(56, 189, 248)` | `#0284C7` `(2, 132, 199)` | `#FF8800` `(255, 136, 0)` | Focused inputs and active component outlines |
| **Primary Text** | `text_primary` | `#E2E8F0` `(226, 232, 240)` | `#0F172A` `(15, 23, 42)` | `#FFFFFF` `(255, 255, 255)` | High-emphasis headings, values, and titles |
| **Secondary Text** | `text_secondary` | `#94A3B8` `(148, 163, 184)` | `#475569` `(71, 85, 105)` | `#DCEBFF` `(220, 235, 255)` | Medium-emphasis labels and column captions |
| **Muted Text** | `text_muted` | `#64748B` `(100, 116, 139)` | `#94A3B8` `(148, 163, 184)` | `#78AAE6` `(120, 170, 230)` | De-emphasized offsets, comments, and empty slots |
| **Monospace Text** | `text_monospace` | `#F1F5F9` `(241, 245, 249)` | `#0F172A` `(15, 23, 42)` | `#FFFFFF` `(255, 255, 255)` | Code disassembly, hex cells, and register dumps |
| **PC Accent** | `accent_pc` | `#38BDF8` `(56, 189, 248)` | `#0284C7` `(2, 132, 199)` | `#FFAA00` `(255, 170, 0)` | Execution pointer cursor and active instruction border |
| **PC Highlight Fill** | `accent_pc_bg` | `rgba(12, 74, 110, 0.55)` | `rgba(186, 230, 253, 0.63)` | `rgba(255, 136, 0, 0.47)` | Subtle luminous background behind active PC row |
| **Diff Accent** | `accent_diff` | `#67E8F9` `(103, 232, 249)` | `#0891B2` `(8, 145, 178)` | `#FFC832` `(255, 200, 50)` | Mutated register and memory diff foreground |
| **Diff Fill** | `accent_diff_bg` | `rgba(8, 51, 68, 0.47)` | `rgba(207, 250, 254, 0.63)` | `rgba(255, 170, 0, 0.35)` | Diff highlight background pill |
| **Success Accent** | `accent_success` | `#34D399` `(52, 211, 153)` | `#10B981` `(16, 185, 129)` | `#50DC64` `(80, 220, 100)` | Running status, active breakpoints, healthy metrics |
| **Warning Accent** | `accent_warning` | `#FBBF24` `(251, 191, 36)` | `#D97706` `(217, 119, 6)` | `#FF8800` `(255, 136, 0)` | Paused status, IPL 7 interrupts, high memory usage |
| **Error Accent** | `accent_error` | `#F87171` `(248, 113, 113)` | `#E11D48` `(225, 29, 72)` | `#FF5050` `(255, 80, 80)` | Halted status, bus error vector, unaligned address |
| **Address Blue** | `address_blue` | `#818CF8` `(129, 140, 248)` | `#4F46E5` `(79, 70, 229)` | `#8CC8FF` `(140, 200, 255)` | 24-bit memory addresses and hex column indices |
| **CCR Active Fill** | `ccr_active_bg` | `#059669` `(5, 150, 105)` | `#10B981` `(16, 185, 129)` | `#FF8800` `(255, 136, 0)` | Active condition flag badge background |
| **CCR Active Text** | `ccr_active_text` | `#FFFFFF` `(255, 255, 255)` | `#FFFFFF` `(255, 255, 255)` | `#000000` `(0, 0, 0)` | Active condition flag letter text |
| **CCR Inactive Fill** | `ccr_inactive_bg` | `#2A3241` `(42, 50, 65)` | `#CBD5E1` `(203, 213, 225)` | `#003264` `(0, 50, 100)` | Inactive condition flag badge background |
| **CCR Inactive Text** | `ccr_inactive_text` | `#E2E8F0` `(226, 232, 240)` | `#334155` `(51, 65, 85)` | `#FFFFFF` `(255, 255, 255)` | Inactive condition flag letter text |

---

## 6. Keyboard Shortcuts & Interactive Bindings Matrix

All keyboard shortcuts are centralized and dispatched in [`crates/gui/src/app.rs`](../../../crates/gui/src/app.rs) (`handle_keyboard_shortcuts`) and interactive panels:

| Domain | Key Binding | Action / Behavior | UI Origin & Target |
| :--- | :--- | :--- | :--- |
| **View Modes** | `F12` or `F2` | Toggle Developer Studio vs Screen-Only Game View | Central Viewport / Canvas |
| **View Modes** | `Escape` | Exit Game View and return to Developer Studio (pauses emulation) | Central Viewport |
| **Execution** | `F5` or `Space` | Toggle Run / Pause (when no text input field is focused) | Top Menu Bar / Engine |
| **Stepping** | `F10` | Step single M68000 instruction forward | Top Menu Bar / Disassembly |
| **Stepping** | `Shift + F10` | Step backward (rewind 1 instruction via temporal buffer) | Top Menu Bar / Temporal Bar |
| **Stepping** | `F11` | Step single Color Clock phase (CCK ~280 ns) | Top Menu Bar / Microcode |
| **Tooling** | `F8` | Toggle Microcode Inspector dock visibility | Left Dock |
| **Tooling** | `Alt + T` | Toggle Temporal Recording on / off | Temporal Scrubber Bar |
| **Tooling** | `Ctrl + R` | Machine Cold / Full Reset (`session.reset()`) | Top Menu Bar / File |
| **Tooling** | `Ctrl + O` | Open Load Binary modal file dialog | Top Menu Bar / File |
| **Persistence** | `F6` | Quick-save snapshot to in-memory Slot 1 | Top Menu Bar / State |
| **Persistence** | `F9` | Quick-load snapshot from in-memory Slot 1 | Top Menu Bar / State |
| **Persistence** | `Ctrl + S` | Open Save State to disk dialog | Top Menu Bar / State |
| **Persistence** | `Ctrl + L` | Open Load State from disk dialog | Top Menu Bar / State |
| **GUI Zoom** | `Ctrl + +` / `Ctrl + =` | Increase GUI scale (+25%, max 250%) | Top Menu Bar / Context |
| **GUI Zoom** | `Ctrl + -` | Decrease GUI scale (-25%, min 75%) | Top Menu Bar / Context |
| **GUI Zoom** | `Ctrl + 0` | Reset GUI scale to default (100%) | Top Menu Bar / Context |
| **Disassembly** | `Arrow Up` / `Down` | Scroll stream lines without moving PC | Disassembly Viewport |
| **Disassembly** | `Enter` | Open in-place instruction assembler patcher | Disassembly Selected Row |
| **Disassembly** | `Escape` | Cancel in-place instruction editing | Disassembly Patcher |
| **Memory Hex** | `Arrow Keys` | Navigate byte cursor horizontally/vertically | Memory Hex Grid |
| **Memory Hex** | `PageUp` / `PageDown` | Jump cursor 128 bytes backward/forward | Memory Hex Grid |
| **Memory Hex** | `Home` / `End` | Jump cursor to start/end of current row | Memory Hex Grid |
| **Memory Hex** | `Tab` or `Enter` | Commit edited hex nibble/byte and advance | Memory Hex Cell Editor |
| **Memory Hex** | `Escape` | Cancel active byte edit | Memory Hex Cell Editor |
| **Memory Hex** | `Ctrl + Wheel` | Jump 1,024 bytes (1 KB page) up/down | Memory Hex Scroller |

---

## 7. Reference Documentation & Upstream Ground Truth

- [GUI Frontend Architecture & Overview](GUI.md): High-level frontend architecture, CRT shaders, aspect ratios, and input mapping.
- [egui Guidelines & Frontend Best Practices](egui%20Guidelines.md): Immediate-mode UI patterns, layout invariants, and headless testing rules.
- [Debugger Architecture & Inspection Engine](Debugger.md): Breakpoint traps, stepping controls, and disassembler facade.
- [Binary Injection & Loader Implementation](../../../crates/debugger/src/loader.rs): Kickstart overlay disengagement, RAM injection, and prefetch priming.
- [Temporal Debugger Ring Buffer](../../../crates/debugger/src/temporal.rs): High-capacity zero-allocation execution history ring buffer.
- [GUI Crate Implementation](../../../crates/gui/src/gui.rs): Living Rust implementation of eframe app, views, docks, and modals.
- [GUI Interaction Test Suite](../../../crates/gui/tests/test_interactions.rs): Headless integration tests validating UI layout, keyboard events, and theme toggling.



