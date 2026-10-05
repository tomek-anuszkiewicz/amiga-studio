# egui Frontend Rules

Applies to `crates/gui`. Read [GUI.md](../../Obsidian/Amiga/Design/GUI.md), [GUI Specification.md](../../Obsidian/Amiga/Design/GUI%20Specification.md), [egui Guidelines.md](../../Obsidian/Amiga/Design/egui%20Guidelines.md), and [Debugger.md](../../Obsidian/Amiga/Design/Debugger.md) for the affected UI behavior.

- Render by directly querying machine and debugger state in a synchronous loop. No core-to-UI callbacks, observers, channels, async runtimes, or shared threading handles.
- Execute bounded emulation slices per frame to keep native and WASM frontends responsive. Organize layout modules by visible panel responsibilities.
- Use points and relative layout with host DPI scaling. Preserve 4:3 display framing, integer prescaling, dark-default theme, host preference detection, and runtime theme switching.
- Avoid heap allocation in inner table/grid loops and tooltip generators; use static text, stack buffers, or preformatted labels. Virtualize large memory and trace views with `ScrollArea::show_rows`.
- Every inspectable hardware field and action needs self-contained hover documentation. Use structured `on_hover_ui` for complex fields. Panel mappings and tooltip content are in [GUI authoring detail](../skills/egui-vision-debugger/references/gui-authoring.md).
- Cover interactions with headless integration tests. Before fixing an interaction defect, confirm its reproduction and verify the repaired behavior per [testing policy](unit-testing-policy.md).
- For visual layout or resize defects, use [egui-vision-debugger](../skills/egui-vision-debugger/SKILL.md) and [capture-gui-screenshot](../skills/capture-gui-screenshot/SKILL.md) as applicable. Report what automated assertions and visual inspection establish.
