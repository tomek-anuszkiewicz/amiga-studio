# Hardware bus topology

Load before chip, DMA, bus, interrupt or peripheral coordination design and changes.
Read relevant specifications through
[General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md), including
[register ownership](../../Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md),
[cross-chip signals](../../Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
and [platform invariants](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).
Unresolved hardware questions follow [source provenance](amiga-rag.md).

- The machine owns CPU, memory bus, Agnus, Denise, Paula and both CIAs, with master
  `cck: u64`. The synchronization unit is the Color Clock (CCK), approximately
  3.54 MHz PAL / 3.58 MHz NTSC. A 68000 bus cycle spans four CPU clocks, or CCK1
  and CCK2. Blocked Chip RAM returns `BusResult::WaitState`; the CPU does not advance
  its active micro-step while waiting.
- Chips hold no direct peer pointers or private mutation/callback backchannels.
  The machine loop and memory bus route physical signals: execute chip state,
  poll motherboard outputs, then drive target inputs at the modeled phase/delay.
- Agnus exclusively arbitrates Chip RAM DMA, owns/updates bitplane, sprite, audio,
  disk, Copper and Blitter pointers, and drives memory addresses and RGA. Active OCS
  pointers are 18-bit word addresses; use shared `MASK_OCS_DMA_ADDRESS` and derived
  register-half masks for writes, arithmetic and DMA accesses as specified in
  [Agnus](../../Obsidian/Amiga/Design/Agnus.md).
- Denise and Paula passively latch shared-bus data on matching strobes. They do not
  read PhysicalMemory directly, retain Chip RAM views or own DMA address generators.
  Disk DMA is bidirectional: Agnus supplies addresses/cycles while Paula supplies
  or receives holding-register data.
- Route DMA requests, interrupts, reset, sync, TOD and peripheral port signals
  through the coordinator and authoritative pin/state interfaces. Do not replace
  physical dependencies with software shortcuts.

Specifications own detailed signal names, directions, registers and timing tables.
