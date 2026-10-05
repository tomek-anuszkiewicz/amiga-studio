# GUI layout and contextual documentation

Read when creating panels or hardware tooltips. These details supplement the [GUI rule](../../../rules/egui-best-practices.md) and the authoritative GUI specifications; visual verification uses the owning skill.

## 2. Visual-to-Source 1:1 Layout Mapping
Structure all files under `crates/gui/src/layout/` to strictly mirror what is visible on the screen:
- `top_menu_bar.rs`: Top toolbar (file loading, run/pause/step, theme, zoom).
- `main_viewport/amiga_screen.rs`: 4:3 centered Amiga CRT display container.
- `main_viewport/temporal_bar.rs`: Timeline scrubber and rewind slider below screen.
- `left_dock/registers.rs`: Live D0-D7, A0-A7, PC, SR, and CCR LED badges.
- `left_dock/microcode.rs`: Micro-step index, active ALU op, staging registers.
- `left_dock/disassembly.rs`: Disassembly stream and breakpoint margin.
- `right_dock/memory_hex.rs`: Hex + ASCII editor with chunk quick-jump buttons.
- `right_dock/memory_search.rs`: Hex/ASCII pattern search bar.
- `right_dock/trace_log.rs`: 1024-entry execution history log table.

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
