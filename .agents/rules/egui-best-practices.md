# egui advance requirements

Load before relevant GUI design or implementation. Read the affected behavior in
[GUI](../../Obsidian/Amiga/Design/Application/GUI.md),
[egui guidance](../../Obsidian/Amiga/Design/egui%20Guidelines.md) and
[Debugger](../../Obsidian/Amiga/Design/Application/Debugger.md).

- Prepare inspector data at a coherent machine-owner boundary outside continuous
  Run. During Run draw retained values, prepared labels and visible rows without
  new inspection or formatting. Game view neither prepares nor draws inspectors.
- Core chips have no UI callbacks, observers, channels, async runtimes or shared
  threading handles. Host synchronization obeys the approved
  [Clippy permissions](../../tools/verification/checks/clippy_permissions.json);
  shared-image access never waits for a lock or new image.
- Execute bounded emulation slices on the host owner independently of repaint
  cadence to keep native and Wasm controls responsive. Preserve complete-frame
  proportions, relative/DPI layout and required interaction/hover behavior.
- Avoid allocation in inner grids/tooltips and virtualize large memory/trace views.
  Use nearby panel patterns; detailed visual/tooltip standards and evidence
  boundaries belong to [GUI criteria](../references/gui-criteria.md).
- Cover interactions with headless integration tests. Reproduce an interaction
  defect before mutation; layout/resize fixes additionally need actual image
  inspection and focused visual confirmation. Report what each kind of evidence
  establishes.

[TA-capture-gui-screenshot](../skills/TA-capture-gui-screenshot/SKILL.md) returns a
capture without interpreting or repairing it. Implementation and selected fixes
use ordinary Implementer operations; visual findings do not launch a repair loop.
