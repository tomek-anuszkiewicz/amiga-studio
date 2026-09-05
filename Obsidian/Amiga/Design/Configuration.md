# Amiga 500 Configuration Specification (`A500Config`)

> [!NOTE]
> All ROM binaries, disk images, and memory configurations are injected into the core externally as raw byte slices (`&[u8]`), preserving WASM portability and system independence (see [AGENTS.md](file:///d:/Programowanie/Amiga/AGENTS.md)).

---

## 1. Configuration Data Structure

The `A500Config` struct defines the machine parameters used when initializing or re-configuring the `A500` emulator:

```rust
use serde::{Deserialize, Serialize};

/// Master configuration struct for the Amiga 500 emulator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A500Config {
    /// Video standard and master clock timing
    pub video_standard: VideoStandard,

    /// Chip RAM capacity
    pub chip_ram: ChipRamSize,

    /// Optional Trapdoor Slow RAM at $C00000
    pub slow_ram: SlowRamSize,

    /// Optional Auto-Config Fast RAM at $200000
    pub fast_ram: FastRamSize,

    /// Custom chipset hardware revisions
    pub agnus_model: AgnusModel,
    pub denise_model: DeniseModel,

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
    /// 512 KB Chip RAM ($000000 - $07FFFF) — Standard stock A500
    Kb512,
    /// 1 MB Chip RAM ($000000 - $0FFFFF) — Enhanced Chip Set (ECS)
    Mb1,
    /// 2 MB Chip RAM ($000000 - $1FFFFF) — Megachip / Late ECS
    Mb2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlowRamSize {
    /// No Slow RAM installed
    None,
    /// 512 KB Trapdoor Slow RAM at $C00000 ($C00000 - $C7FFFF) — Most common A501 expansion
    Kb512,
    /// 1.8 MB Extended Pseudo-Fast RAM ($C00000 - $DCFFFF)
    Mb1_8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FastRamSize {
    None,
    Mb2,
    Mb4,
    Mb8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgnusModel {
    /// OCS 8370 (NTSC 512 KB) / 8371 (PAL 512 KB)
    Ocs512Kb,
    /// ECS 8372A (1 MB Agnus with PAL/NTSC software switching)
    Ecs1Mb,
    /// ECS 8372B (2 MB Agnus)
    Ecs2Mb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeniseModel {
    /// OCS 8362 (Standard OCS Denise)
    Ocs8362,
    /// ECS 8373 (Enhanced Denise with Productivity modes)
    Ecs8373,
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

## 2. ROM & Image Injection Interfaces

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

## 3. Standard Preset Configurations

1. **Stock Amiga 500 (Early 1987 OCS):**
   - PAL 50Hz, 512 KB Chip RAM, No Slow RAM, No Fast RAM.
   - Agnus OCS 8371, Denise OCS 8362, Kickstart 1.2 or 1.3 (256 KB).
2. **Classic Gaming Amiga 500 (1 MB OCS - Most Common):**
   - PAL 50Hz, 512 KB Chip RAM + 512 KB Slow RAM (A501 expansion).
   - Agnus OCS 8371, Denise OCS 8362, Kickstart 1.3.
3. **Amiga 500 Plus / ECS:**
   - PAL 50Hz, 1 MB Chip RAM, Agnus ECS 8372A, Denise ECS 8373, Kickstart 2.04 (512 KB).
