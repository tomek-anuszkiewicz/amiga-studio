# CPU Motorola M68000

- Keep code in directory `M68000`
- Code should be cycle-exact
- CPU state should be read-only (queryable)
- Implement all registers, including internal ones (prefetch queue, instruction stage)
- Provide methods to get and set whole CPU state at once for save/load state
- Build instruction execution state machine (e.g. big switch / table for instructions)
- Memory bus operations and instruction execution stages are modeled using Color Clock phases: **CCK1** and **CCK2** (replacing traditional S-states, where 1 bus access = 4 CPU clocks = 2 CCK cycles: CCK1 and CCK2):
	- When reading/writing during a CCK phase, check memory bus availability via `MemoryBusResult`:
		- If bus is ready (`MemoryBusResult::Ready`), progress to the next state/phase.
		- If bus is blocked (`MemoryBusResult::Blocked` / `Wait`), remain at the same state/phase (insert wait state).
- Execute prefetch in defined CCK cycles as per the MC68000 manual
- For each instruction, calculate exact cycle timings based on CCK phases
- Only progress when CPU cycle requirements are satisfied and the bus is unblocked
	- Note that because CPU may be blocked by Chip RAM / DMA, it can wait additional CCKs
- Use `MemoryBus` to access memory
- Method to progress one cycle / CCK forward
- Initial milestone: Implement and verify `NOP` instruction