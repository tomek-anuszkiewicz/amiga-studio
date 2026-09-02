
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
- **Interrupt Management & Internal Interrupt State**:
  - **Paula**: Houses central Amiga interrupt controller (`INTENA` `$DFF09A`, `INTREQ` `$DFF09C`). Multiplexes interrupt sources into priority levels:
    - Level 1: Serial TX empty (TBE), Disk Block (DSKBLK), Software interrupt (SOFT)
    - Level 2: CIA-A interrupt (`PORTS`)
    - Level 3: Vertical Blank (VERTB), Blitter finished (BLIT), Copper (COPER)
    - Level 4: Audio channels 0-3 (`AUD0`-`AUD3`)
    - Level 5: Serial RX buffer full (RBF), Disk Sync match (DSKSYN)
    - Level 6: CIA-B interrupt (`EXTER`)
  - **CIA-A**: Keeps internal Interrupt Control Register (ICR) state; drives the Level 2 interrupt line
  - **CIA-B**: Keeps internal Interrupt Control Register (ICR) state; drives the Level 6 interrupt line
  - **Agnus**: Signals VBlank, Blitter, and Copper interrupt conditions to Paula
  - **Interrupt State Inspection**: Each chip maintains its internal interrupt line state and exposes public getter methods so that external systems (the main loop) can read active interrupt lines
- In early implementation phase: treat all registers as R/W
- Provide `get_state()` / `set_state()` methods for full serialization in save states

TODO:
### RESET

### 1. Custom Chip Registers (Agnus, Denise, Paula)

The custom chips do not wipe all internal data registers (like color palettes or coordinate pointers), but their critical control registers are forced to safe defaults by the hardware reset line:

- **DMA Control (`DMACON` / `DMACONR` at `$DFF096` / `$DFF002`):**
    
    - **Set to `$0000` (All DMA channels disabled).**
        
    - Bitplane, Copper, Blitter, Audio, Sprite, and Disk DMA are completely shut off.
        
    - Agnus stops generating memory access cycles, ensuring the Chip RAM bus is completely unblocked for the 68000 CPU.
        
- **Interrupt Enable (`INTENA` / `INTENAR` at `$DFF09A` / `$DFF001C`):**
    
    - **Set to `$0000` (All custom interrupts disabled).**
        
    - Master interrupt enable and all 14 individual interrupt sources (Vertical Blank, Copper, Audio, Blitter, CIAs, etc.) are masked.
        
- **Interrupt Requests (`INTREQ` / `INTREQR` at `$DFF09C` / `$DFF01E`):**
    
    - Cleared to `$0000` (Any pending latch states or requests are discarded).
        
- **Audio Channels (`AUDxDAT`, `AUDxVOL`, `AUDxPER`, `AUDxLEN` at `$DFF0A0–$DFF0DF`):**
    
    - Volume registers are forced or defaulted to zero (`0`), muting the audio DACs immediately.
        
- **Copper (`COPCON` at `$DFF02E`):**
    
    - Copper danger mode bit (CDANG) is set to `0` (Copper is locked out of writing to dangerous registers like Blitter/Copper jump pointers).
        
    - Copper program counters (`COP1LCH`/`COP2LCH`) are halted since DMA is off.
        
- **Display & Beam Positioning (`BPLCON0`, `VPOSR`, etc.):**
    
    - Bitplane enable bits are cleared (`BPLCON0` planes set to 0), disabling video rendering.
        
- **Other Custom Registers (Palettes `COLORxx`, Sprite positions, etc.):**
    
    - Retain undefined/random values or previous states. Kickstart explicitly initializes the palette and coordinate registers during early boot.
        

### 2. Complex Interface Adapters (8520 CIA-A & CIA-B)

The reset pin on the 6526/8520 CIA chips drives standard, well-defined hardware resets:

- **Data Direction Registers (`DDRA`, `DDRB`):**
    
    - Initialized to **`$00`** (All I/O port pins set as high-impedance **inputs**).
        
    - This is critical for CIA-A Port A, where bit 0 is connected to the low-memory overlay (`_OVL`):
        
        - As an input with an external pull-up, it guarantees that the overlay line stays active, keeping `map_kickstart_to_low_memory()` engaged.
            
- **Port Data Registers (`PRA`, `PRB`):**
    
    - Initialized to **`$00`** (Output latches cleared, though pins float as inputs).
        
- **Timers (`Timer A`, `Timer B`):**
    
    - Timer Control Registers (`CRA`, `CRB`) are set to **`$00`** (Timers are stopped, continuous mode reset, PBON output disabled).
        
    - Latches/Counters typically reset to `$FFFF`.
        
- **Interrupt Control Register (`ICR`):**
    
    - **Set to `$00` (All CIA interrupts masked/disabled).**
        
    - Pending interrupt flag latches are cleared.
        
- **Serial Shift Register (`SDR`):**
    
    - Cleared to `$00`, with shift mode disabled in `CRA`.
        
- **Time-of-Day Clock (TOD):**
    
    - TOD event counter/clock is halted or remains unlatched until programmed.
        

### Summary Checklist for an Emulator Reset

To bring the chips to the correct initial state before running the CPU:

1. Clear `DMACON` to `0` (guaranteeing `chip_ram_blocked = false`).
    
2. Clear `INTENA` and `INTREQ` to `0`.
    
3. Mute Paula audio channels (volumes to `0`).
    
4. Set CIA-A and CIA-B control registers (`CRA`, `CRB`, `ICR`, `DDRA`, `DDRB`) to `0`.
    
5. Ensure CIA-A Port A bit 0 leaves the Kickstart overlay enabled (`$000000` routed to ROM).