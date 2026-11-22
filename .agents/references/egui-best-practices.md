# egui implementation and review practices

Apply to the selected GUI implementation or review. Read the affected behavior in
[GUI](../../Obsidian/Amiga/Design/Application/GUI.md),
[egui guidance](../../Obsidian/Amiga/Design/egui%20Guidelines.md) and
[Debugger](../../Obsidian/Amiga/Design/Application/Debugger.md).

- Outside continuous Run, prepare inspector data at a point when the machine
  owner can provide a consistent view of state. During Run, render the inspector
  from retained values, prepared labels and visible rows without new inspection
  or formatting. Game view neither prepares nor draws inspectors.
  Send mutations through the ordered machine-owner commands described in
  [Coherent Owner Data and Retained Inspection](../../Obsidian/Amiga/Design/egui%20Guidelines.md#2-coherent-owner-data-and-retained-inspection).
- Core chips have no UI callbacks, observers, channels, async runtimes or shared
  threading handles. Host synchronization obeys the approved
  [Clippy permissions](../../tools/verification/checks/clippy_permissions.json);
  shared-image access never waits for a lock or new image.
- Execute bounded emulation slices on the host owner on a schedule independent
  of repaints. This keeps native and Wasm controls responsive. Preserve the full
  frame's proportions, relative layout, DPI scaling and required interactions
  and hover behavior.
- Avoid allocation in inner grids and tooltips. Virtualize large memory and
  trace views by rendering the visible rows. Follow nearby panel patterns and
  use the detailed visual, tooltip and evidence requirements in the
  [GUI criteria](gui-criteria.md).
- Cover interactions with headless integration tests. Reproduce an interaction
  defect before changing the implementation. For layout and resize fixes, also
  inspect the captured images and confirm the affected layout or resize scenario
  visually. Report what each kind of evidence establishes.
