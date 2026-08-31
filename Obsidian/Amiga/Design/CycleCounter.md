# Cycle Counter

- Shared 64-bit unsigned integer counter (`u64`).
- Counts Color Clocks (CCK, ~3.54 MHz PAL / ~3.57 MHz NTSC).
- Shared among all custom chips (Agnus, Denise, Paula), CIAs, and CPU as read-only.
- Prevents counter overflow across long emulation sessions and save states.
- 1 CCK = 2 CPU clock ticks (MC68000 @ 7.09 MHz PAL / 7.16 MHz NTSC).