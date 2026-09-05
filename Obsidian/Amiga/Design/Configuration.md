# Amiga 500 Configuration Specification (`A500Config`)

> [!NOTE]
> All ROM binaries, disk images, and memory configurations are injected into the core externally as raw byte slices (`&[u8]`), preserving WASM portability and system independence (see [AGENTS.md](file:///d:/Programowanie/Amiga/AGENTS.md)).

---

## 1. Focused Initial Scope: Stock Amiga 500 (OCS)

To achieve cycle-exact emulation quickly without configuration explosion, the initial core focuses strictly on the standard **Amiga 500 OCS (Rev 5 / Rev 6a)** hardware profile:
- 512 KB Chip RAM
- Optional 512 KB Trapdoor Slow RAM (A501 expansion)
- Optional 4 MB Fast RAM
- OCS Chipset (Agnus 8370/8371, Denise 8362)
- Game Ports (Port 1 and Port 2 configured for Mouse, Joystick, or Unplugged)

---

## 2. Configuration Data Structure

```rust
use serde::{Deserialize, Serialize};

/// Master configuration struct for the Amiga 500 emulator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A500Config {
    /// Video standard and master clock timing
    pub video_standard: VideoStandard,

    /// Chip RAM capacity (Fixed at 512 KB for baseline A500 OCS)
    pub chip_ram: ChipRamSize,

    /// Optional Trapdoor Slow RAM at $C00000
    pub slow_ram: SlowRamSize,

    /// Optional Auto-Config Fast RAM at $200000
    pub fast_ram: FastRamSize,

    /// Custom chipset hardware revisions (OCS)
    pub agnus_model: AgnusModel,
    pub denise_model: DeniseModel,

    /// Game Ports input configuration
    pub port1: GamePortDevice,
    pub port2: GamePortDevice,

    /// Floppy drive configuration
    pub floppy_drives: FloppyConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoStandard {
    /// PAL (Europe / Australia): 50 Hz vertical refresh, ~3.546895 MHz Color Clock (CCK)
    Pal,
    /// NTSC (North America / Japan): 60 Hz vertical refresh, ~3.579545 MHz Color Clock (CCK)
    Ntsc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipRamSize {
    /// 512 KB Chip RAM ($000000 - $07FFFF) — Standard baseline A500
    Kb512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlowRamSize {
    /// No Slow RAM installed
    None,
    /// 512 KB Trapdoor Slow RAM at $C00000 ($C00000 - $C7FFFF) — Standard A501 expansion
    Kb512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FastRamSize {
    /// No Fast RAM
    None,
    /// 4 MB Auto-Config Fast RAM ($200000 - $5FFFFF)
    Mb4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgnusModel {
    /// OCS 8370 (NTSC 512 KB) / 8371 (PAL 512 KB)
    Ocs512Kb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeniseModel {
    /// OCS 8362 (Standard OCS Denise)
    Ocs8362,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePortDevice {
    /// Two-button quadrature mouse (default on Port 1)
    Mouse,
    /// Atari-standard digital 2-button joystick (default on Port 2)
    Joystick,
    /// Nothing plugged in
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyConfig {
    pub df0_enabled: bool,
    pub df1_enabled: bool,
    pub df2_enabled: bool,
    pub df3_enabled: bool,
}
```

---

## 3. ROM & Image Injection Interfaces

Because the emulator core is decoupled from the host filesystem, ROMs and disk images are injected as byte slices:

```rust
impl A500 {
    /// Instantiate a new machine with the specified configuration and Kickstart ROM image
    pub fn new(config: A500Config, kickstart_rom: &[u8]) -> Result<Self, ConfigError> {
        // Validate Kickstart ROM length (256 KB or 512 KB)
        // Initialize MemoryBus, CPU, Agnus, Denise, Paula, CIAs
        // Perform initial Cold Reset
        todo!()
    }

    /// Insert or swap an ADF floppy disk image into a specific drive (e.g. DF0)
    pub fn insert_floppy(&mut self, drive: usize, adf_bytes: &[u8]) -> Result<(), FloppyError> {
        todo!()
    }

    /// Eject disk from specified drive
    pub fn eject_floppy(&mut self, drive: usize) {
        todo!()
    }
}
```

---

## 4. Standard Preset Profiles

1. **Stock Amiga 500 (1987 baseline):**
   - PAL 50Hz, 512 KB Chip RAM, No Slow RAM, No Fast RAM.
   - Port 1: Mouse, Port 2: Joystick.
   - Agnus OCS 8371, Denise OCS 8362, Kickstart 1.2 or 1.3 (256 KB).
2. **Classic 1 MB Gaming Setup (Most common A500):**
   - PAL 50Hz, 512 KB Chip RAM + 512 KB Slow RAM (A501 trapdoor).
   - Port 1: Mouse, Port 2: Joystick.
   - Agnus OCS 8371, Denise OCS 8362, Kickstart 1.3.
3. **Productivity / Expanded Setup:**
   - PAL 50Hz, 512 KB Chip RAM + 512 KB Slow RAM + 4 MB Fast RAM.
   - Kickstart 1.3.

---

## 5. Future Roadmap Extensions (Post-A500)

*The following configurations are deferred to subsequent project milestones:*
- **A500 Rev 6A / Late OCS:** 1 MB Chip RAM jumperable option (Fat Agnus 8372A in OCS mode).
- **A500 Plus (ECS):** 1 MB Chip RAM (`ChipRamSize::Mb1`), ECS Agnus 8372A (`AgnusModel::Ecs1Mb`), ECS Denise 8373 (`DeniseModel::Ecs8373`), Kickstart 2.04 (512 KB).
- **Megachip / ECS 2MB:** 2 MB Chip RAM (`ChipRamSize::Mb2`), Agnus 8372B.
- **A1200 (AGA):** 68EC020 CPU (32-bit), 2 MB Chip RAM, Alice (AGA Agnus), Lisa (AGA Denise), 24-bit color palette.
- **Extended Peripherals:** 4-Player Parallel Port Joystick adapter, analog proportional joysticks.
