# Hardware Bus Topology

Applies to chip, DMA, bus, interrupt, and peripheral coordination. Read the affected specifications through [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md), including the [register ownership matrix](../../Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md), [cross-chip signals catalog](../../Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md), and [platform invariants](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).

- Chips hold no direct peer pointers and never invoke peer mutations, callbacks, or private backchannels. The machine loop and memory bus route physical signals.
- Coordination follows chip execution, motherboard polling of output pins, then driving target inputs at the modeled phase and propagation delay. Preserve explicit pin/state interfaces.
- Agnus exclusively arbitrates Chip RAM DMA, owns and updates bitplane, sprite, audio, disk, Copper, and Blitter pointers, and drives the memory address and RGA buses.
- Denise and Paula passively latch shared-bus data on matching register strobes. They neither read PhysicalMemory directly nor retain Chip RAM views or autonomous DMA address generators.
- Disk DMA is bidirectional; Agnus still generates addresses and bus cycles while Paula supplies or receives holding-register data.
- Route DMA requests, interrupts, reset, sync, TOD, and peripheral port signals through the coordinator according to their authoritative signal definitions. Do not replace physical dependencies with software shortcuts.

Use [audit-hardware-quality](../skills/audit-hardware-quality/SKILL.md) for the review procedure. Specifications own detailed signal names, directions, register lists, and timing tables.
