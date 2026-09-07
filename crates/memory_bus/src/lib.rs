//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod arbitration;
pub mod big_array;
pub mod map;
pub mod test_injection;

pub use arbitration::MemoryBusResult;
pub use config::{A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard};
pub use map::{
    build_bank_map, build_preset_bank_map, get_preset_bank_map, handler_for_bank, BankHandler,
    BankReadByteFn, BankWriteByteFn, BANK_MAP_BARE, BANK_MAP_EXPANDED, BANK_MAP_STANDARD,
};
pub use rtc;
pub use rtc::RtcMsm6242b;

use serde::{Deserialize, Serialize};

/// Maximum size of physical memory regions
pub const CHIP_RAM_SIZE_512K: usize = 512 * 1024;
pub const CHIP_RAM_SIZE_1MB: usize = 1024 * 1024;
pub const SLOW_RAM_SIZE: usize = 512 * 1024;
pub const FAST_RAM_SIZE: usize = 8 * 1024 * 1024; // Max 8MB Zorro II Fast RAM
pub const KICKSTART_SIZE_256K: usize = 256 * 1024;
pub const KICKSTART_SIZE_512K: usize = 512 * 1024;

/// Classification of a 64 KB physical memory bank
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryBank {
    /// Chip RAM (Base $000000-$07FFFF, optionally extended to $000000-$0FFFFF)
    ChipRam,
    /// Auto-Config Fast RAM expansion ($200000-$9FFFFF)
    FastRam,
    /// CIA-A and CIA-B peripheral registers ($BF0000-$BFFFFF)
    Cia,
    /// Slow / Trapdoor RAM ($C00000-$C7FFFF)
    SlowRam,
    /// Real-Time Clock ($DC0000-$DC003F)
    Rtc,
    /// Custom Chip Registers ($DF0000-$DFFFFF, active at $DFF000-$DFFFFE)
    CustomChips,
    /// Kickstart ROM ($F80000-$FFFFFF)
    KickstartRom,
    /// Unmapped floating open bus (returns $FF / $FFFF)
    OpenBus,
}

/// Cycle-exact Amiga 500 MemoryBus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBus {
    /// Active hardware configuration
    pub config: A500Config,

    /// 256-entry direct bank dispatch table (function pointers to read/write handlers)
    #[serde(with = "big_array")]
    pub bank_map: [BankHandler; 256],

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
    #[serde(with = "big_array")]
    pub custom_registers: [u16; 256],

    /// Real-Time Clock (OKI MSM6242B) at $DC0000..$DC003F
    pub rtc: rtc::RtcMsm6242b,

    /// Sparse test memory for CPU SingleStepTests and synthetic test runner execution
    #[serde(skip)]
    pub test_memory: Option<std::collections::HashMap<u32, u8>>,

    /// Default byte value returned when reading unpopulated test memory or unmapped open bus space.
    /// In real Amiga hardware execution, this is 0xFF (floating open bus with pull-up resistors).
    /// In SingleStepTests flat RAM harness, this can be configured to 0x00.
    #[serde(default = "default_unmapped_byte")]
    pub unmapped_byte: u8,
}

#[inline(always)]
fn default_unmapped_byte() -> u8 {
    0xFF
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

    /// Creates a lightweight test memory bus with sparse 24-bit test RAM for CPU SingleStepTests
    pub fn new_test() -> Self {
        let config = A500Config::default();
        let bank_map = [map::OPEN_BUS_HANDLER; 256];
        let rtc = rtc::RtcMsm6242b::new(config.rtc());
        Self {
            config,
            bank_map,
            chip_ram: Vec::new(),
            slow_ram: None,
            fast_ram: None,
            kickstart_rom: Vec::new(),
            read_latch: 0xFFFF,
            pending_write_data: 0,
            chip_ram_blocked: false,
            low_memory_overlay: false,
            cia_a_registers: [0xFF; 16],
            cia_b_registers: [0xFF; 16],
            custom_registers: [0xFFFF; 256],
            rtc,
            test_memory: Some(std::collections::HashMap::with_capacity(32)),
            unmapped_byte: 0xFF,
        }
    }

    /// Enables sparse flat test memory on an existing bus
    pub fn enable_flat_test_memory(&mut self) {
        if self.test_memory.is_none() {
            self.test_memory = Some(std::collections::HashMap::with_capacity(32));
        }
    }

    /// Returns the byte value returned when reading unpopulated test memory or unmapped open bus
    #[inline]
    pub fn unmapped_byte(&self) -> u8 {
        self.unmapped_byte
    }

    /// Sets the byte value returned when reading unpopulated test memory or unmapped open bus
    #[inline]
    pub fn set_unmapped_byte(&mut self, val: u8) {
        self.unmapped_byte = val;
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
        let bank_map = map::build_bank_map(&config);
        let rtc = rtc::RtcMsm6242b::new(config.rtc());

        let mut bus = Self {
            config,
            bank_map,
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
            rtc,
            test_memory: None,
            unmapped_byte: 0xFF,
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

        self.rtc.model = config.rtc();
        self.bank_map = map::build_bank_map(&config);
        self.config = config;
    }

    /// Advances internal clock timers (including Real-Time Clock) by the given CCK cycles
    #[inline]
    pub fn step_cck(&mut self, cck_cycles: u64) {
        self.rtc.step_cck(cck_cycles);
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
