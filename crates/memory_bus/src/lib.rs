//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod arbitration;
pub mod big_array;
pub mod bus_trait;
pub mod map;
pub mod test_bus;
pub mod test_injection;

pub use arbitration::{function_code, BusAccessSize, BusResult};
pub use bus_trait::{AddressBus, RecordedTransaction};
pub use config::{
    A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard,
};
pub use map::{
    build_bank_map, build_preset_bank_map, get_preset_bank_map, handler_for_bank, BankHandler,
    BankReadByteFn, BankWriteByteFn, BANK_MAP_BARE, BANK_MAP_EXPANDED, BANK_MAP_STANDARD,
};
pub use rtc;
pub use rtc::RtcMsm6242b;
pub use test_bus::TestMemoryBus;

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

    /// Default byte value returned when reading unpopulated memory or unmapped open bus space.
    /// In real Amiga hardware execution, this is 0xFF (floating open bus with pull-up resistors).
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
            chip_ram_blocked: false,
            low_memory_overlay: true,
            cia_a_registers: [0xFF; 16],
            cia_b_registers: [0xFF; 16],
            custom_registers: [0xFFFF; 256],
            rtc,
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

    /// Queries whether the Chip RAM bus lock flag is asserted by Agnus/DMA.
    ///
    /// # Architectural Note
    /// This method is strictly an external inspection/diagnostic getter for test assertions and debuggers.
    /// It must NOT be used in CPU stepping or bus arbitration logic; clients must perform bus accesses
    /// via `read_byte`, `read_word`, `write_byte`, or `write_word` and check the returned `BusResult`.
    #[inline]
    pub fn is_chip_ram_locked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Reads an 8-bit byte from the 24-bit physical address space, checking for Chip RAM bus contention.
    /// Returns `BusResult::WaitState` if the target is Chip RAM (or Slow RAM) and Agnus/DMA is blocking the bus.
    #[inline(always)]
    pub fn read_byte(&self, addr: u32) -> BusResult<u8> {
        if self.chip_ram_blocked && self.is_chip_ram_target(addr) {
            return BusResult::WaitState;
        }
        BusResult::Ready(self.read_byte_internal(addr))
    }

    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space, checking for Chip RAM bus contention.
    /// Returns `BusResult::WaitState` if the target is Chip RAM (or Slow RAM) and Agnus/DMA is blocking the bus.
    #[inline(always)]
    pub fn read_word(&self, addr: u32) -> BusResult<u16> {
        if self.chip_ram_blocked && self.is_chip_ram_target(addr) {
            return BusResult::WaitState;
        }
        BusResult::Ready(self.read_word_internal(addr))
    }

    /// Writes an 8-bit byte to the 24-bit physical address space, checking for Chip RAM bus contention.
    /// Returns `BusResult::WaitState` if the target is Chip RAM (or Slow RAM) and Agnus/DMA is blocking the bus.
    #[inline(always)]
    pub fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        if self.chip_ram_blocked && self.is_chip_ram_target(addr) {
            return BusResult::WaitState;
        }
        self.write_byte_internal(addr, val);
        BusResult::Ready(())
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space, checking for Chip RAM bus contention.
    /// Returns `BusResult::WaitState` if the target is Chip RAM (or Slow RAM) and Agnus/DMA is blocking the bus.
    #[inline(always)]
    pub fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        if self.chip_ram_blocked && self.is_chip_ram_target(addr) {
            return BusResult::WaitState;
        }
        self.write_word_internal(addr, val);
        BusResult::Ready(())
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
        self.map_kickstart_to_low_memory();
    }

    /// Warm Reset: Preserves RAM contents and re-engages Kickstart overlay
    pub fn reset_warm(&mut self) {
        self.chip_ram_blocked = false;
        self.map_kickstart_to_low_memory();
    }
}

impl AddressBus for MemoryBus {
    #[inline(always)]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        MemoryBus::read_byte(self, addr)
    }

    #[inline(always)]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        MemoryBus::read_word(self, addr)
    }

    #[inline(always)]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        MemoryBus::write_byte(self, addr, val)
    }

    #[inline(always)]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        MemoryBus::write_word(self, addr, val)
    }

    #[inline(always)]
    fn read_word_debug(&self, addr: u32) -> u16 {
        MemoryBus::read_word_debug(self, addr)
    }
}
