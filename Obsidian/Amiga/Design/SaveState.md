# Save State

- Define save state to load/save the complete machine state (`A500`).
- Serializes `CycleCounter` (64-bit CCK counter).
- Serializes CPU registers, internal states, and prefetch buffer.
- Serializes Custom Chips (Agnus, Denise, Paula) and CIAs (CIA-A, CIA-B).
- Memory state (Chip RAM, Slow RAM, Fast RAM) is encoded and stored as **Base64** strings.
- Implement methods to serialize / deserialize state to/from JSON format.
- Support selecting and restoring memory configurations (0.5MB Chip, 0.5MB Chip + 0.5MB Slow, 0.5MB Chip + 0.5MB Slow + 4MB Fast).