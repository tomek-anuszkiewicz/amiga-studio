//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod arbitration;
pub mod config;
pub mod map;
pub mod test_injection;

pub use arbitration::MemoryBusResult;
pub use config::{A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard};

use serde::{Deserialize, Serialize};

/// Maximum size of physical memory regions
pub const CHIP_RAM_SIZE_512K: usize = 512 * 1024;
pub const CHIP_RAM_SIZE_1MB: usize = 1024 * 1024;
pub const SLOW_RAM_SIZE: usize = 512 * 1024;
pub const FAST_RAM_SIZE: usize = 8 * 1024 * 1024; // Max 8MB Zorro II Fast RAM
pub const KICKSTART_SIZE_256K: usize = 256 * 1024;
pub const KICKSTART_SIZE_512K: usize = 512 * 1024;

/// Cycle-exact Amiga 500 MemoryBus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBus {
    /// Active hardware configuration
    pub config: A500Config,

    /// Physical Chip RAM buffer (512 KB default, expandable to 1 MB)
    pub chip_ram: Vec<u8>,

    /// Slow / Pseudo-fast RAM at $C00000 (512 KB, trapdoor expansion)
    pub slow_ram: Option<Vec<u8>>,

    /// Fast RAM at $200000 (up to 8 MB)
    pub fast_ram: Option<Vec<u8>>,

    /// Kickstart ROM buffer (256 KB or 512 KB)
    pub kickstart_rom: Vec<u8>,

    /// Internal transparent read latch holding data between CCK1 and CCK2
    pub read_latch: u16,

    /// Pending write data registered during CCK1 write phase
    pub pending_write_data: u16,

    /// Flag indicating whether Agnus/DMA currently blocks the Chip RAM bus
    pub chip_ram_blocked: bool,

    /// Low-memory boot overlay (_OVL) active flag
    pub low_memory_overlay: bool,

    /// CIA-A 8-bit register state placeholder (odd byte addresses $BFE001..$BFEF01)
    pub cia_a_registers: [u8; 16],

    /// CIA-B 8-bit register state placeholder (even byte addresses $BFD000..$BFDF00)
    pub cia_b_registers: [u8; 16],

    /// Custom chip register space $DFF000-$DFFFFE (256 16-bit words)
    pub custom_registers: [u16; 256],

    /// Real-Time Clock register bank at $DC0000..$DC003F (16 4-bit registers)
    pub rtc_registers: [u8; 16],
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBus {
    /// Creates a standard A500 MemoryBus using the default configuration (Standard 1 MB + RTC, PAL)
    pub fn new() -> Self {
        Self::from_config(A500Config::default())
    }

    /// Creates a MemoryBus configured per the provided A500Config
    pub fn from_config(config: A500Config) -> Self {
        let chip_ram_size = match config.chip_ram() {
            ChipRamSize::Kb512 => CHIP_RAM_SIZE_512K,
        };
        let slow_ram = match config.slow_ram() {
            SlowRamSize::None => None,
            SlowRamSize::Kb512 => Some(vec![0x00; SLOW_RAM_SIZE]),
        };
        let fast_ram = match config.fast_ram() {
            FastRamSize::None => None,
            FastRamSize::Mb4 => Some(vec![0x00; 4 * 1024 * 1024]),
        };

        let mut bus = Self {
            config,
            chip_ram: vec![0x00; chip_ram_size],
            slow_ram,
            fast_ram,
            kickstart_rom: vec![0xFF; KICKSTART_SIZE_256K],
            read_latch: 0xFFFF,
            pending_write_data: 0,
            chip_ram_blocked: false,
            low_memory_overlay: true,
            cia_a_registers: [0xFF; 16],
            cia_b_registers: [0xFF; 16],
            custom_registers: [0xFFFF; 256],
            rtc_registers: [0x00; 16],
        };
        bus.map_kickstart_to_low_memory();
        bus
    }

    /// Reconfigures RAM buffers and RTC mapping by applying a new A500Config
    pub fn apply_config(&mut self, config: A500Config) {
        let chip_ram_size = match config.chip_ram() {
            ChipRamSize::Kb512 => CHIP_RAM_SIZE_512K,
        };
        self.chip_ram.resize(chip_ram_size, 0);

        self.slow_ram = match config.slow_ram() {
            SlowRamSize::None => None,
            SlowRamSize::Kb512 => Some(vec![0x00; SLOW_RAM_SIZE]),
        };

        self.fast_ram = match config.fast_ram() {
            FastRamSize::None => None,
            FastRamSize::Mb4 => Some(vec![0x00; 4 * 1024 * 1024]),
        };

        self.config = config;
    }

    /// Engages low-memory boot overlay (_OVL), routing $000000-$07FFFF accesses to Kickstart ROM
    pub fn map_kickstart_to_low_memory(&mut self) {
        self.low_memory_overlay = true;
    }

    /// Disengages low-memory boot overlay (_OVL), restoring physical Chip RAM at $000000-$07FFFF
    pub fn map_chip_ram_to_low_memory(&mut self) {
        self.low_memory_overlay = false;
    }

    /// Queries whether the low-memory overlay is currently engaged
    #[inline]
    pub fn is_low_memory_overlay_active(&self) -> bool {
        self.low_memory_overlay
    }

    /// Locks Chip RAM bus (Agnus/DMA cycle stealing active)
    #[inline]
    pub fn lock_chip_ram(&mut self) {
        self.chip_ram_blocked = true;
    }

    /// Unlocks Chip RAM bus (Agnus/DMA cycle stealing inactive)
    #[inline]
    pub fn unlock_chip_ram(&mut self) {
        self.chip_ram_blocked = false;
    }

    /// Queries whether the Chip RAM bus is currently blocked by custom chips
    #[inline]
    pub fn is_chip_ram_blocked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Cold / Hard Reset: Wipes all RAM to zero and re-engages Kickstart overlay
    pub fn reset_cold(&mut self) {
        self.chip_ram.fill(0x00);
        if let Some(slow_ram) = &mut self.slow_ram {
            slow_ram.fill(0x00);
        }
        if let Some(fast_ram) = &mut self.fast_ram {
            fast_ram.fill(0x00);
        }
        self.chip_ram_blocked = false;
        self.read_latch = 0xFFFF;
        self.map_kickstart_to_low_memory();
    }

    /// Warm Reset: Preserves RAM contents and re-engages Kickstart overlay
    pub fn reset_warm(&mut self) {
        self.chip_ram_blocked = false;
        self.read_latch = 0xFFFF;
        self.map_kickstart_to_low_memory();
    }
}
