# GUI behavior and visual review

The [GUI specification](../../Obsidian/Amiga/Design/Application/GUI.md) owns view
behavior and scaling; [egui guidance](../../Obsidian/Amiga/Design/egui%20Guidelines.md)
owns visual responsibilities, complete hover coverage, structured tooltip layout
and allocation constraints. Apply the shared [egui best practices](egui-best-practices.md)
to the selected implementation or review; assess only the assigned UI scope.

- Preserve dark-default appearance, host preference detection, runtime theme
  switching, point/relative layout and host DPI scaling.
- Uniformly scale the complete framebuffer to fit while preserving its existing
  width-to-height ratio. Do not force 4:3 or stretch axes independently; 1:1 pixels
  are unnecessary. Do not silently choose a new raster/blanking region.
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

A capture provides an image; interpretation belongs to the requested assessment.
A failed visual assessment reports findings. The user selects any subsequent
repair, which retains the original reproduction. Findings do not start a capture
or repair loop.
