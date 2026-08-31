# Main Loop (A500)

- Name the machine struct `A500`
- Create whole machine wiring CPU (`M68000`), `MemoryBus`, `CycleCounter` (64-bit CCK), and custom chips
- Stepping capabilities:
  - Process by single CCK cycle
  - Process predefined number of cycles
  - Process a full video frame
- Reset method (initializes hardware state and routes low memory addresses to Kickstart)
- Load / save state methods for the whole machine (JSON with Base64-encoded memory)
- Output methods to retrieve video frame and sound buffers