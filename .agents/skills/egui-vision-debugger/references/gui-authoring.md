# GUI layout and contextual documentation

When creating panels or hardware tooltips, read the
[frontend rules](../../../rules/egui-best-practices.md) and
[egui design guidance](../../../../Obsidian/Amiga/Design/egui%20Guidelines.md).
The design guidance owns visual responsibility, complete hover coverage,
structured tooltip layout and allocation constraints. The
[GUI specification](../../../../Obsidian/Amiga/Design/Application/GUI.md)
owns view behavior and image scaling. Use the
[vision debugger](../SKILL.md) for reproduction and visual confirmation.

For tooltip semantics, use the owning CPU, bus or chip specification and its
hardware sources. Do not infer physical ALU timing from an emulator callback
or describe a modeled CCK boundary as a measured silicon edge.
