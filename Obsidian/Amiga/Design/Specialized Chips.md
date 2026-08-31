# Specialized Chips

- Implement each chip in a dedicated module/directory matching its name
- Custom chips and controllers to implement:
  - **Agnus** (DMA controller, Copper, Blitter)
  - **Denise** (Video & display generator, bitplanes, sprites, color registers)
  - **Paula** (Audio channels, floppy disk controller, UART, interrupt controller)
  - **CIA-A** (Complex Interface Adapter 8520: keyboard, timers, joystick/mouse buttons)
  - **CIA-B** (Complex Interface Adapter 8520: disk motor/select, timers, serial/parallel handshakes)
- For each chip, provide methods to get and set register/memory cell values
- Define all hardware registers and bitfields using canonical names from Amiga documentation
- **Register Width & Byte Access Handling**:
  - **Custom Chips (16-bit)**: Custom registers are 16-bit wide. Byte writes target the addressed byte lane while the unaddressed byte defaults to `$FF`. Reading disconnected/write-only registers returns `$FF`.
  - **CIAs (8-bit)**: CIAs are native 8-bit peripherals. Byte reads and byte writes are the standard operating mode (CIA-A on odd addresses, CIA-B on even addresses). Word reads return the 8-bit value with `$FF` on the unused byte lane.
- In early implementation phase: treat all registers as R/W
- Provide `get_state()` / `set_state()` methods for full serialization in save states