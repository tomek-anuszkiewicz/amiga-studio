//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod address_bus;
mod chip_access;
pub mod map;
pub mod presets;

pub use address_bus::{AddressBus, BusResult};
pub use config::{
    A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard,
    OPEN_BUS_BYTE, OPEN_BUS_WORD,
};
pub use map::{
    BankHandler, BankReadByteDebugFn, BankReadByteFn, BankReadWordDebugFn, BankReadWordFn,
    BankWriteByteDebugFn, BankWriteByteFn, BankWriteWordDebugFn, BankWriteWordFn, MemoryBank,
};
pub use presets::{
    build_bank_map, build_preset_bank_map, get_preset_bank_map, BANK_MAP_BARE, BANK_MAP_EXPANDED,
    BANK_MAP_STANDARD,
};
pub use rtc;
pub use rtc::RtcMsm6242b;

use serde::{Deserialize, Serialize};

pub const MASK_WORD_ADDRESS: u32 = MASK_PHYSICAL_ADDRESS & !1;

pub const MASK_PHYSICAL_ADDRESS: u32 = 0x00FF_FFFF;

/// Fixed factory A500 Kickstart capacity, in bytes.
pub const KICKSTART_ROM_SIZE: usize = 256 * 1024;
const MASK_KICKSTART_ADDRESS: usize = KICKSTART_ROM_SIZE - 1;

/// Maximum size of physical memory regions
pub const CHIP_RAM_SIZE_512K: usize = 512 * 1024;
pub const DEFAULT_BOOT_SSP: u32 = CHIP_RAM_SIZE_512K as u32;
pub const SLOW_RAM_SIZE: usize = 512 * 1024;
pub const MAX_FAST_RAM_SIZE: usize = 4 * 1024 * 1024; // 4MB Fast RAM (Auto-Config expansion at $200000..$5FFFFF)

/// A ROM image or saved ROM buffer has an unsupported length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KickstartSizeError {
    pub actual: usize,
}

impl core::fmt::Display for KickstartSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Kickstart ROM must be exactly 256 KiB ({KICKSTART_ROM_SIZE} bytes); received {} bytes.",
            self.actual
        )
    }
}

impl std::error::Error for KickstartSizeError {}

pub fn validate_kickstart_size(actual: usize) -> Result<(), KickstartSizeError> {
    if actual != KICKSTART_ROM_SIZE {
        return Err(KickstartSizeError { actual });
    }
    Ok(())
}

/// Fixed-size Kickstart bytes in a pure machine snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KickstartBytes(#[serde(deserialize_with = "deserialize_kickstart_rom")] Vec<u8>);

impl KickstartBytes {
    pub fn from_image(bytes: Vec<u8>) -> Result<Self, KickstartSizeError> {
        validate_kickstart_size(bytes.len())?;
        Ok(Self(bytes))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Deterministic boot vectors for headless execution without a supplied ROM.
    pub fn synthetic() -> Self {
        let mut bytes = vec![0xFF; KICKSTART_ROM_SIZE];
        bytes[..4].copy_from_slice(&DEFAULT_BOOT_SSP.to_be_bytes());
        bytes[4..8].copy_from_slice(&0u32.to_be_bytes());
        Self(bytes)
    }
}

fn deserialize_kickstart_rom<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let rom = Vec::<u8>::deserialize(deserializer)?;
    validate_kickstart_size(rom.len()).map_err(serde::de::Error::custom)?;
    Ok(rom)
}

/// Cycle-exact Amiga 500 PhysicalMemory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalMemory<
    Chip = Vec<u8>,
    Slow = Option<Vec<u8>>,
    Fast = Option<Vec<u8>>,
    Rom = KickstartBytes,
> {
    /// Active hardware configuration
    pub config: A500Config,

    /// 256-entry direct bank dispatch table (function pointers to read/write handlers)
    #[serde(with = "config::big_array")]
    pub bank_map: [BankHandler; config::address_map::MEMORY_BANK_COUNT],

    /// Physical Chip RAM buffer (512 KB)
    chip_ram: Chip,

    chip_budget: config::chip_ram::ChipRamBudget,

    /// Slow / Pseudo-fast RAM at $C00000 (512 KB, trapdoor expansion)
    pub slow_ram: Slow,

    /// Fast RAM at $200000 (4 MB)
    pub fast_ram: Fast,

    /// Physical Kickstart ROM buffer (256 KB)
    kickstart_rom: Rom,

    /// Flag indicating whether Agnus/DMA currently blocks the Chip RAM bus
    pub chip_ram_blocked: bool,

    /// Low-memory boot overlay (_OVL) active flag
    pub low_memory_overlay: bool,
}

impl<Chip, Slow, Fast, Rom> PhysicalMemory<Chip, Slow, Fast, Rom> {
    /// Converts snapshot storage representations without altering bank routing or timing.
    pub fn map_storage<C, S, F, R>(
        self,
        chip: impl FnOnce(Chip) -> C,
        slow: impl FnOnce(Slow) -> S,
        fast: impl FnOnce(Fast) -> F,
        rom: impl FnOnce(Rom) -> R,
    ) -> PhysicalMemory<C, S, F, R> {
        PhysicalMemory {
            config: self.config,
            bank_map: self.bank_map,
            chip_ram: chip(self.chip_ram),
            chip_budget: self.chip_budget,
            slow_ram: slow(self.slow_ram),
            fast_ram: fast(self.fast_ram),
            kickstart_rom: rom(self.kickstart_rom),
            chip_ram_blocked: self.chip_ram_blocked,
            low_memory_overlay: self.low_memory_overlay,
        }
    }

    pub fn storage(&self) -> (&Chip, &Slow, &Fast, &Rom) {
        (
            &self.chip_ram,
            &self.slow_ram,
            &self.fast_ram,
            &self.kickstart_rom,
        )
    }
}

impl Default for PhysicalMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicalMemory {
    /// Creates a standard A500 PhysicalMemory using default configuration (Standard 1 MB, PAL)
    pub fn new() -> Self {
        Self::from_config(A500Config::default())
    }

    /// Creates PhysicalMemory configured per the provided A500Config
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
            FastRamSize::Mb4 => Some(vec![0x00; MAX_FAST_RAM_SIZE]),
        };
        let bank_map = presets::build_bank_map(&config);

        let mut bus = Self {
            config,
            bank_map,
            chip_ram: vec![0x00; chip_ram_size],
            chip_budget: config::chip_ram::ChipRamBudget::default(),
            slow_ram,
            fast_ram,
            kickstart_rom: KickstartBytes::synthetic(),
            chip_ram_blocked: false,
            low_memory_overlay: true,
        };
        bus.map_kickstart_to_low_memory();
        bus
    }

    /// Engages low-memory boot overlay (_OVL), routing $000000-$07FFFF accesses to Kickstart ROM
    pub fn map_kickstart_to_low_memory(&mut self) {
        self.low_memory_overlay = true;
        for b in config::address_map::CHIP_BANK_FIRST..=config::address_map::CHIP_BANK_LAST {
            self.bank_map[b as usize] = map::KICKSTART_ROM_HANDLER;
        }
    }

    /// Disengages low-memory boot overlay (_OVL), restoring physical Chip RAM at $000000-$07FFFF
    pub fn map_chip_ram_to_low_memory(&mut self) {
        self.low_memory_overlay = false;
        for b in config::address_map::CHIP_BANK_FIRST..=config::address_map::CHIP_BANK_LAST {
            self.bank_map[b as usize] = map::CHIP_RAM_HANDLER;
        }
    }

    /// Queries whether the low-memory overlay is currently engaged
    #[inline]
    pub fn is_low_memory_overlay_active(&self) -> bool {
        self.low_memory_overlay
    }

    /// Reads an 8-bit byte from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        (self.bank_map[((addr >> 16) & 0xFF) as usize].read_byte)(self, addr)
    }

    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        (self.bank_map[((addr >> 16) & 0xFF) as usize].read_word)(self, addr)
    }

    /// Writes an 8-bit byte to the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let write_fn = self.bank_map[((addr >> 16) & 0xFF) as usize].write_byte;
        write_fn(self, addr, val)
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let write_fn = self.bank_map[((addr >> 16) & 0xFF) as usize].write_word;
        write_fn(self, addr, val)
    }

    /// Direct debug/injection byte block writer into physical memory.
    /// Bypasses bus arbitration locks (such as Chip RAM contention) and side-effects.
    /// ROM images must contain exactly 256 KiB; rejection leaves memory unchanged.
    pub fn write_bytes_debug(
        &mut self,
        addr: u32,
        data: &[u8],
    ) -> Result<usize, KickstartSizeError> {
        let addr = addr & MASK_PHYSICAL_ADDRESS;
        if addr >= config::address_map::KICKSTART_BASE
            || (addr as usize).saturating_add(data.len())
                > config::address_map::KICKSTART_BASE as usize
        {
            validate_kickstart_size(data.len())?;
        }
        for (i, &b) in data.iter().enumerate() {
            self.write_byte_debug(addr.wrapping_add(i as u32), b);
        }
        Ok(data.len())
    }

    /// Read-only view of the fixed-size Kickstart image.
    pub fn kickstart_rom(&self) -> &[u8] {
        &self.kickstart_rom.0
    }

    /// Side-effect-free byte read for debugger inspection and test result assertions
    #[inline(always)]
    pub fn read_byte_debug(&self, addr: u32) -> u8 {
        (self.bank_map[((addr >> 16) & 0xFF) as usize].read_byte_debug)(self, addr)
    }

    /// Side-effect-free word read for disassemblers, debugger inspection, and test result assertions
    #[inline(always)]
    pub fn read_word_debug(&self, addr: u32) -> u16 {
        (self.bank_map[((addr >> 16) & 0xFF) as usize].read_word_debug)(self, addr)
    }

    /// Side-effect-free byte write for debugger modification
    #[inline(always)]
    pub fn write_byte_debug(&mut self, addr: u32, val: u8) {
        let write_fn = self.bank_map[((addr >> 16) & 0xFF) as usize].write_byte_debug;
        write_fn(self, addr, val);
    }

    /// Side-effect-free word write for debugger modification
    #[inline(always)]
    pub fn write_word_debug(&mut self, addr: u32, val: u16) {
        let write_fn = self.bank_map[((addr >> 16) & 0xFF) as usize].write_word_debug;
        write_fn(self, addr, val);
    }

    /// Reset: Wipes all RAM to zero and re-engages Kickstart overlay
    pub fn reset(&mut self) {
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

impl AddressBus for PhysicalMemory {
    #[inline]
    fn begin_cpu_cck(&mut self, _cck: u64) -> u64 {
        let cck = self.memory_cck().wrapping_add(1);
        self.set_cck(cck);
        cck
    }

    #[inline(always)]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        PhysicalMemory::read_byte(self, addr)
    }

    #[inline(always)]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        PhysicalMemory::read_word(self, addr)
    }

    #[inline(always)]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        PhysicalMemory::write_byte(self, addr, val)
    }

    #[inline(always)]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        PhysicalMemory::write_word(self, addr, val)
    }

    #[inline(always)]
    fn read_word_debug(&self, addr: u32) -> u16 {
        PhysicalMemory::read_word_debug(self, addr)
    }
}
