---
name: capture-gui-screenshot
description: 1-shot headless screenshot capture and multimodal visual inspection for the Amiga 500 egui Developer Studio.
---

# capture-gui-screenshot Skill

This skill provides instant, 1-shot visual perception of the Amiga 500 emulator Developer Studio (`crates/gui`) directly within the main pair-programming conversation.

Unlike the iterative repair loop in [`egui-vision-debugger`](../egui-vision-debugger/SKILL.md), this skill is designed for **fast, zero-overhead visual checks** whenever you or the user need to see the current UI state, evaluate layout balance, or audit themes.

---

## 1. Execution Mode: Main Agent Direct Execution

- **Execution Host:** **Main Agent** (Runs directly in the active conversation; no subagent delegation required).
- **Execution Time:** **~0.3 seconds** (Headless offscreen rendering via `egui_kittest` and `wgpu`).
- **Context Footprint:** **~1,000 image tokens** (Replaces 20,000+ tokens of reading code files to guess layout).
- **User Visibility:** Screenshots are automatically copied to the conversation artifacts directory and embedded in markdown for the user.

---

## 2. Headless Inspector Command

The capture engine is driven by the compiled `gui-inspector` binary from `crates/gui`:

```powershell
cargo run -p gui --bin gui-inspector -- --scenario <SCENARIO> --output target/gui_captures/<NAME>.png [OPTIONS]
```

### Supported Scenarios:
| Scenario | Description | Key Focus Area |
| :--- | :--- | :--- |
| `baseline` *(Default)* | Standard 1280x720 3-dock developer studio with loaded sample code | General layout, register panel, CRT viewport, hex editor |
| `clean_startup` | Fresh emulator startup with uninitialized state | Initial window balance, default placeholder text |
| `workbench_theme` | Classic Commodore Amiga Workbench blue/orange color palette | Theme contrast, palette fidelity, widget borders |
| `game_mode` | ScreenOnly retro CRT display view | CRT frame scaling, scanline presentation |
| `hover_splitter` | Mouse hovered over the left dock resize border | Splitter highlight, resize indicator visibility |
| `hover_scrollbar`| Mouse hovered over the memory editor scrollbar | Scrollbar thumb contrast, margin clipping |
| `hover_register` | Mouse hovered over register `D0` in the left dock | In-app documentation card, bit breakdown readability |
| `hover_ccr`      | Mouse hovered over CCR status flags (`XNZVC`) | CCR documentation card, flag tooltip positioning |
| `hover_memory`   | Mouse hovered over memory map bookmarks | Memory region tooltip, address boundary display |
| `edit_register`  | Inline hexadecimal editing active on register `D0` | Focus border, active cursor styling, value contrast |
| `edit_disasm`    | Inline assembly editing active at address `$001000` | Disassembly input box, commit/cancel indicators |
| `small_window`   | Constrained 1024x600 laptop viewport | Responsive docking, column collapse, overflow clipping |

### Common Flags:
- `--width <PIXELS>`: Custom viewport width (e.g. `1024`, `1600`, `1920`).
- `--height <PIXELS>`: Custom viewport height (e.g. `600`, `900`, `1080`).
- `--json-metadata <PATH>`: Dumps AccessKit widget tree and bounding boxes to a JSON file.

---

## 3. The 4-Step Visual Inspection Workflow

Whenever a visual check is needed, the agent executes this exact 4-step sequence:

### Step 1: Render Headless Frame
```powershell
cargo run -p gui --bin gui-inspector -- --scenario baseline --output target/gui_captures/baseline.png
```

### Step 2: Inspect via Native Multimodal Vision
Call `view_file` to load the rendered PNG into the agent's multimodal vision:
```rust
view_file(AbsolutePath: "D:/Programowanie/Amiga/target/gui_captures/baseline.png")
```
The agent inspects:
1. Are the 3 columns (CPU Registers, Amiga Viewport, Memory Hex Editor) balanced?
2. Is the retro CRT screen maintaining its proper 4:3 aspect ratio?
3. Is typography crisp and monospace addresses aligned?
4. Are text or scrollbars clipping against dock boundaries?

### Step 3: Copy to Artifact Directory for User Visibility
Copy the verified PNG to the conversation artifacts directory:
```powershell
Copy-Item target/gui_captures/baseline.png <ARTIFACTS_DIR>/gui_baseline.png
```

### Step 4: Embed in Response or Walkthrough
Embed the image directly into the response or artifact:
```markdown
![Amiga 500 Developer Studio (Baseline View)](file:///<ARTIFACTS_DIR>/gui_baseline.png)
```
This guarantees that **both the Agent and the User have 100% synchronized visual ground truth**.

---

## 4. When to Switch to `egui-vision-debugger`

- Use **`capture-gui-screenshot`** for: Viewing current state, verifying theme colors, checking responsive resolution, or showing the user the UI.
- Escalate to **[`egui-vision-debugger`](../egui-vision-debugger/SKILL.md)** (Subagent) when: You detect a visual defect that requires multi-iteration code modifications in `crates/gui/src/layout/` and recompilation loops.
