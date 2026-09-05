# Amiga 500 Specialized Chips Architecture (Agnus, Denise, Paula, CIAs)

> [!NOTE]
> Physical bus addressing, byte-lane decoding, and open-bus `$FF` handling for custom chips and CIAs are defined in [MemoryBus.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/MemoryBus.md).
> Save state serialization patterns are detailed in [SaveState.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/SaveState.md).

---

## 1. Modular Subsystem Decomposition

To prevent monolithic files, each chip module serves as a coordinator that delegates to focused internal subcomponents:

```
chips/
├── agnus/
│   ├── mod.rs             // Agnus coordinator & register dispatch
│   ├── copper.rs          // Copper coprocessor (MOVE, WAIT, SKIP)
│   ├── blitter.rs         // 4-channel DMA Blitter, minterms, line drawing
│   ├── dma.rs             // DMA channel arbitration & DMACON scheduling
│   └── beam.rs            // Horizontal & vertical beam counters (VHPOSR, VPOSR)
├── denise/
│   ├── mod.rs             // Denise coordinator & register dispatch
│   ├── bitplanes.rs       // BPLCON0-3, bitplane serializers, dual playfield
│   ├── sprites.rs         // 8 hardware sprites, position comparators
│   ├── palette.rs         // COLOR00-COLOR31 (12-bit RGB444 color registers)
│   └── collision.rs       // CLXDAT sprite/playfield collision detection
├── paula/
│   ├── mod.rs             // Paula coordinator & register dispatch
│   ├── audio.rs           // 4 independent DMA sound channels (AUD0-AUD3)
│   ├── floppy.rs          // Floppy MFM encoding/decoding, track buffer, DSKSYN
│   ├── uart.rs            // Serial port UART (SERDAT, SERPER)
│   └── interrupts.rs      // Central INTENA and INTREQ priority encoder
└── cia/
    ├── mod.rs             // MOS 8520 coordinator (instantiated as cia_a and cia_b)
    ├── timers.rs          // 16-bit decrementing Timer A and Timer B
    ├── ports.rs           // Port A and Port B bidirectional I/O latches (PRA, PRB)
    ├── tod.rs             // 50Hz/60Hz Time-of-Day counter
    └── sdr.rs             // 8-bit bidirectional serial shift register
```

---

## 2. Multi-Chip Peripheral Coordination

Several Amiga peripherals bridge across multiple custom chips. Rather than coupling chips directly, the emulator organizes these cross-chip subsystems in a dedicated `peripherals/` layer coordinated by the main loop:

```
peripherals/
├── floppy/
│   ├── mod.rs             // Multi-chip Floppy coordinator (bridges CIA-A, CIA-B, Paula, Agnus)
│   ├── drive.rs           // Drive mechanics (motor on/off, track step, side select, status flags)
│   └── disk_image.rs      // ADF byte slice (&[u8]) loader & MFM track buffer
├── game_ports/
│   ├── mod.rs             // Game Ports coordinator (bridges Denise, CIA-A, Paula)
│   ├── mouse.rs           // Host delta accumulator -> Denise JOY0DAT quadrature counters
│   └── joystick.rs        // Host direction/buttons -> Denise JOY1DAT and CIA-A PRA
└── keyboard/
    ├── mod.rs             // Keyboard coordinator (bridges CIA-A SDR and reset pin)
    ├── scancodes.rs       // Host key to Amiga 8-bit scancode matrix translator
    └── reset_detector.rs  // Ctrl-Amiga-Amiga watcher triggering hardware _RESET
```

### 2.1 Game Ports (Mouse & Joysticks) & Host Input Timing
At the physical hardware level, **Mouse and Joystick use identical 9-pin D-sub connectors (Port 1 and Port 2)** and are decoded by the exact same registers:

```mermaid
flowchart TD
    PORT1["Game Port 1 (Mouse / Joy 1)"] -->|Quadrature / Direction| DENISE_JOY0["Denise: JOY0DAT ($DFF00A)"]
    PORT1 -->|Button 1 (Left Click / Fire 1)| CIAA_PRA6["CIA-A: PRA bit 6 ($BFE001)"]
    PORT1 -->|Button 2/3 (Right / Middle Click)| PAULA_POT0["Paula / Denise: POT0DAT / POTGO ($DFF012/$DFF034)"]

    PORT2["Game Port 2 (Joy 2 / Mouse 2)"] -->|Quadrature / Direction| DENISE_JOY1["Denise: JOY1DAT ($DFF00C)"]
    PORT2 -->|Button 1 (Fire 1)| CIAA_PRA7["CIA-A: PRA bit 7 ($BFE001)"]
    PORT2 -->|Button 2 (Fire 2)| PAULA_POT1["Paula / Denise: POT1DAT / POTGO ($DFF014/$DFF034)"]
```

- **Hardware Decoding:**
  - **Direction / Movement:** Mouse quadrature optical pulses or joystick switch closures directly increment/decrement counters in `JOY0DAT` / `JOY1DAT` in Denise.
  - **Buttons:** Fire 1 (Left Mouse Button) is read via **CIA-A Port A** (`_FIR0` bit 6, `_FIR1` bit 7). Fire 2 & 3 (Right / Middle Mouse Button) are read via **Paula / Denise** through the proportional pot pins in `POTGO` (`$DFF034`).
  - *Detailed Specifications:* For full register decoding, pinouts, quadrature math, 4-player adapters, and host mapping, see [Joystick.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/Joystick.md) and [Mouse.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/Mouse.md).
- **Host Input Update Rate (Once Per Frame):**
  - The host window loop provides mouse and joystick updates to the emulator via public `A500` methods from the outside:
    ```rust
    pub fn set_mouse_delta(&mut self, dx: i32, dy: i32);
    pub fn set_joystick_state(&mut self, port: usize, state: JoystickState);
    ```
  - **Is once-per-frame updating sufficient?**
    - **Yes, it is optimal.** Amiga games, demos, and Intuition OS tasks poll mouse coordinates and joystick switches synchronously during the Vertical Blanking interrupt (50 Hz PAL / 60 Hz NTSC, once every 20 ms).
    - Updating host inputs once per frame right before calling `step_frame()` provides sub-millisecond responsiveness with zero input lag.
    - If running sub-frame CCK stepping, host events can also be pushed continuously into the delta accumulators.

### 2.2 Floppy Disk Drive & ADF Image Injection
The 3.5" floppy drive interface spans three different custom chips and connects directly to raw ADF disk images injected as byte slices:
- **ADF Image Connection:**
  - Disk images are passed into the emulator as raw byte slices: `a500.insert_floppy(drive, adf_bytes: &[u8])`.
  - The emulator buffers tracks and serializes sector data into MFM pulses in memory without disk I/O.
- **Hardware Coordination:**
  - **CIA-B (Drive Control Output):** Drives motor on/off (`_MTR`), step pulse (`_STEP`), step direction (`_DIR`), head side select (`_SIDE`), and drive select (`_SEL0`–`_SEL3`).
  - **CIA-A (Drive Status Input):** Reads drive status signals: Disk Ready (`_RDY`), Track 0 sensor (`_TK0`), Write Protect (`_WPROT`), and Disk Change (`_CHNG`).
  - **Paula (High-Speed Data & DMA):** Reads and writes serial MFM data streams (`DSKDAT`), detects track sync words (`DSKSYN`), and generates Disk Block Done (`DSKBLK`) and Disk Sync Match (`DSKSYN`) interrupts.
  - **Agnus (Bus Master):** Allocates Chip RAM DMA cycles for the disk data buffer.

### 2.3 Keyboard & Reset Line
- **CIA-A:** Receives keyboard serial clock and data in its Serial Data Register (`SDR`), firing a Level 2 interrupt (`PORTS`).
- **Keyboard Microcontroller (6500/1):** Monitors for the **Ctrl-Amiga-Amiga** key combination. When detected, the microcontroller physically pulls the system `_RESET` pin low, triggering a hardware reset across CPU, Agnus, Denise, Paula, and CIAs.
- *Detailed Specification:* For complete serial protocol, scancode matrix, handshake timing, and warm reset flows, see [Keyboard.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/Keyboard.md).

### 2.4 Video Beam Synchronization
- **Agnus:** Drives master beam position counters (`VHPOSR`, `VPOSR`) and executes the Copper display list synchronized to beam coordinates.
- **Denise:** Receives the beam clock and sync signals from Agnus to serialize bitplanes and sprites into RGB output pixels.

### 2.5 Low-Memory Boot Overlay (`_OVL`)
- **CIA-A:** Port A bit 0 drives the physical `_OVL` control line.
- **MemoryBus (Gary):** Intercepts `$000000-$07FFFF` accesses and routes them to Kickstart ROM while `_OVL` is asserted.

---

## 3. Paula Interrupt Multiplexing & Signal Routing

Paula houses the central interrupt multiplexer, aggregating internal sources and external CIA lines into priority levels:

| Level | Priority Sources | Triggering Chip / Subcomponent |
| :---: | :--- | :--- |
| **1** | `TBE` (Serial TX Empty), `DSKBLK` (Disk Block Done), `SOFT` | Paula (UART, Floppy) |
| **2** | `PORTS` (CIA-A interrupt line) | CIA-A |
| **3** | `VERTB` (Vertical Blank), `BLIT` (Blitter Finished), `COPER` (Copper) | Agnus (Beam, Blitter, Copper) |
| **4** | `AUD0`, `AUD1`, `AUD2`, `AUD3` (Audio Channels 0–3) | Paula (Audio) |
| **5** | `RBF` (Serial RX Buffer Full), `DSKSYN` (Disk Sync Match) | Paula (UART, Floppy) |
| **6** | `EXTER` (CIA-B interrupt line) | CIA-B |

- **State Inspection:** Each chip maintains its internal interrupt request and mask lines and exposes read-only query methods.
- **Arbitration:** The top-level [Main loop A500.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/Main%20loop%20A500.md) polls Paula and the CIAs to resolve the highest unmasked priority ($IPL \in 1..6$) and drives `cpu.set_ipl(level)`.

---

## 4. Hardware Reset Register Defaults

### 4.1 Custom Chips (Agnus, Denise, Paula)
- **`DMACON` (`$DFF096`):** Forced to **`$0000`** (all DMA channels disabled). Agnus stops generating memory access cycles, ensuring Chip RAM is unblocked.
- **`INTENA` (`$DFF09A`):** Forced to **`$0000`** (master and all 14 individual interrupt sources disabled).
- **`INTREQ` (`$DFF09C`):** Cleared to **`$0000`** (pending latches discarded).
- **`AUDxVOL` (`$DFF0A8..$DFF0D8`):** Volumes forced to `0`, muting audio immediately.
- **`COPCON` (`$DFF02E`):** Danger mode bit `CDANG` set to `0` (Copper locked out of dangerous registers). Copper halted.
- **`BPLCON0` (`$DFF100`):** Bitplane count set to 0, disabling video rendering.

### 4.2 Complex Interface Adapters (CIA-A & CIA-B)
- **`DDRA`, `DDRB`:** Initialized to **`$00`** (all port pins set as high-impedance inputs).
  - Ensures CIA-A Port A bit 0 (`_OVL`) floats high, keeping `map_kickstart_to_low_memory()` active.
- **`PRA`, `PRB`:** Latches cleared to **`$00`**.
- **`CRA`, `CRB`:** Set to **`$00`** (Timers stopped, continuous mode reset, PBON disabled). Counters reset to `$FFFF`.
- **`ICR`:** Reset to **`$00`** (all CIA interrupt sources masked, latches cleared).
- **`SDR`:** Cleared to **`$00`**.
- **`TOD`:** Time-of-Day clock halted/unlatched until programmed.