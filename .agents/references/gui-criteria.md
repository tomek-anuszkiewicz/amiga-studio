# GUI behavior and visual review

The [GUI specification](../../Obsidian/Amiga/Design/Application/GUI.md) defines
view behavior and scaling. The [egui guidance](../../Obsidian/Amiga/Design/egui%20Guidelines.md)
defines what each part of the UI displays, which fields and actions require
hover documentation, how to lay out structured tooltips and where to avoid
allocation. Apply the shared [egui best practices](egui-best-practices.md)
within the assigned UI implementation or review scope.

- Preserve the dark default theme, detection of host theme preferences, theme
  switching while the application runs, point-based and relative layout, and host
  DPI scaling.
- Scale the complete framebuffer to fit. Use the same scale factor on both axes
  to keep its existing width-to-height ratio. Do not force 4:3. Display pixels
  can differ from source pixels, so 1:1 is optional. Do not silently choose a new
  raster/blanking region.
- Inspect layout, contrast, resizing, splitters, visible panel responsibilities and
  relevant interaction behavior. Large memory/trace views use virtualized visible
  rows; inner grids and tooltip generators avoid heap allocation through static
  text, stack buffers or prepared labels.
- Every inspectable hardware field and action has self-contained hover
  documentation. Check this coverage in docks, panels and modal dialogs. Use
  structured `on_hover_ui` for complex fields, with a heading, a short
  explanation and aligned related fields or bits. Source tooltip semantics
  from the owning CPU, bus or chip specification and hardware evidence.
  A callback or modeled CCK boundary is not a measured silicon timing edge.
- Headless interaction assertions establish behavior, not complete visual quality.
  Reproduce interaction defects before mutation; for layout/resize fixes inspect
  actual captured images and confirm the repaired scenario. Report automated and
  visual evidence separately, including unavailable coverage.

Use the captured image as evidence for the requested assessment. If the
assessment finds a problem, report it. If the user selects a repair, carry the
original reproduction into that repair. The finding itself starts no further
capture or repair.
