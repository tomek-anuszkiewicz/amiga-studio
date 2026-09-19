//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod address_bus;
pub mod map;
pub mod presets;

pub use address_bus::{AddressBus, BusResult};
pub use config::{
    A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard,
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

/// Maximum size of physical memory regions
const CHIP_RAM_SIZE_512K: usize = 512 * 1024;
const SLOW_RAM_SIZE: usize = 512 * 1024;
pub const MAX_FAST_RAM_SIZE: usize = 4 * 1024 * 1024; // 4MB Fast RAM (Auto-Config expansion at $200000..$5FFFFF)
const KICKSTART_SIZE_256K: usize = 256 * 1024;

/// Cycle-exact Amiga 500 PhysicalMemory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalMemory {
    /// Active hardware configuration
    pub config: A500Config,

    /// 256-entry direct bank dispatch table (function pointers to read/write handlers)
    #[serde(with = "config::big_array")]
    pub bank_map: [BankHandler; 256],

    /// Physical Chip RAM buffer (512 KB)
    pub chip_ram: Vec<u8>,

    /// Slow / Pseudo-fast RAM at $C00000 (512 KB, trapdoor expansion)
    pub slow_ram: Option<Vec<u8>>,

    /// Fast RAM at $200000 (4 MB)
    pub fast_ram: Option<Vec<u8>>,

    /// Physical Kickstart ROM buffer (256 KB)
    pub kickstart_rom: Vec<u8>,

    /// Flag indicating whether Agnus/DMA currently blocks the Chip RAM bus
    pub chip_ram_blocked: bool,

    /// Low-memory boot overlay (_OVL) active flag
    pub low_memory_overlay: bool,

    /// Default byte value returned when reading unpopulated memory or unmapped open bus space.
    /// In real Amiga hardware execution, this is 0xFF (floating open bus with pull-up resistors).
    #[serde(default = "default_unmapped_byte")]
    pub unmapped_byte: u8,
}

#[inline(always)]
fn default_unmapped_byte() -> u8 {
    0xFF
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

        let mut kickstart_rom = vec![0xFF; KICKSTART_SIZE_256K];
        // Synthetic default boot vectors for unpopulated Kickstart ROM mode (headless testing / developer startup):
        // Vector 0 ($000000): Default SSP = $00080000 (top of standard 512KB Chip RAM)
        kickstart_rom[0..4].copy_from_slice(&0x00080000u32.to_be_bytes());
        // Vector 1 ($000004): Default PC = $00000000 (base of Chip RAM / synthetic test code)
        kickstart_rom[4..8].copy_from_slice(&0x00000000u32.to_be_bytes());

        let mut bus = Self {
            config,
            bank_map,
            chip_ram: vec![0x00; chip_ram_size],
            slow_ram,
            fast_ram,
            kickstart_rom,
            chip_ram_blocked: false,
            low_memory_overlay: true,
            unmapped_byte: 0xFF,
        };
        bus.map_kickstart_to_low_memory();
        bus
    }

    /// Engages low-memory boot overlay (_OVL), routing $000000-$07FFFF accesses to Kickstart ROM
    pub fn map_kickstart_to_low_memory(&mut self) {
        self.low_memory_overlay = true;
        for b in 0x00..=0x07 {
            self.bank_map[b] = map::KICKSTART_ROM_HANDLER;
        }
    }

    /// Disengages low-memory boot overlay (_OVL), restoring physical Chip RAM at $000000-$07FFFF
    pub fn map_chip_ram_to_low_memory(&mut self) {
        self.low_memory_overlay = false;
        for b in 0x00..=0x07 {
            self.bank_map[b] = map::CHIP_RAM_HANDLER;
        }
    }

    /// Queries whether the low-memory overlay is currently engaged
    #[inline]
    pub fn is_low_memory_overlay_active(&self) -> bool {
        self.low_memory_overlay
    }

    /// Reads an 8-bit byte from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_byte(&self, addr: u32) -> BusResult<u8> {
        (self.bank_map[((addr >> 16) & 0xFF) as usize].read_byte)(self, addr)
    }

    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_word(&self, addr: u32) -> BusResult<u16> {
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
    /// Returns the number of bytes written.
    pub fn write_bytes_debug(&mut self, addr: u32, data: &[u8]) -> usize {
        for (i, &b) in data.iter().enumerate() {
            self.write_byte_debug(addr.wrapping_add(i as u32), b);
        }
        data.len()
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
