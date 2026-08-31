# Main Loop (A500)

- Name the machine struct `A500`
- Create whole machine wiring CPU (`M68000`), `MemoryBus`, `CycleCounter` (64-bit CCK), and custom chips
- Stepping capabilities:
  - Process by single CCK cycle
  - Process predefined number of cycles
  - Process a full video frame
- **Interrupt Arbitration in Main Loop**:
  - On each cycle / step, the main loop queries the internal interrupt states of Paula and the CIAs
  - Resolves the active interrupt priority level (IPL 1-6) and drives the CPU interrupt lines (`cpu.set_ipl(level)`)
- Reset method (initializes hardware state and calls `map_kickstart_to_low_memory()`)
- Load / save state methods for the whole machine (JSON with Base64-encoded memory)
- Output methods to retrieve video frame and sound buffers