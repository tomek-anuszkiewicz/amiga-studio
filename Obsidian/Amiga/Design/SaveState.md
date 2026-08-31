# Save State

- Define save state to load/save the complete machine state (`A500`).
- Serializes `CycleCounter` (64-bit CCK counter).
- Serializes CPU registers, internal states, and prefetch buffer.
- Serializes Custom Chips (Agnus, Denise, Paula) and CIAs (CIA-A, CIA-B).
- Serializes `MemoryBus` state, including:
  - Low-memory routing status (`map_kickstart_to_low_memory` active or inactive)
  - Memory configuration type (0.5MB Chip, 0.5MB Chip + 0.5MB Slow, 0.5MB Chip + 0.5MB Slow + 4MB Fast)
  - Memory contents (retrieved from `MemoryBus` as arrays of bytes `[u8]` and encoded to **Base64** strings for JSON storage)
- Implement methods to serialize / deserialize state to/from JSON format.