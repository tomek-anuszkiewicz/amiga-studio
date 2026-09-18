//! Amiga 500 MemoryBus Architecture & 2-Phase CCK Bus Arbitration
//!
//! Provides cycle-exact 24-bit physical address decoding, 2-phase CCK arbitration,
//! DMA wait-state stalling, and hardware quirks per Obsidian/Amiga/Design/MemoryBus.md.

pub mod address_bus;
pub mod bus_result;
pub mod map;

pub use address_bus::AddressBus;
pub use bus_result::BusResult;
pub use config::{
    A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard,
};
pub use map::{
    build_bank_map, build_preset_bank_map, get_preset_bank_map, handler_for_bank, BankHandler,
    BankReadByteFn, BankReadWordFn, BankWriteByteFn, BankWriteWordFn, BANK_MAP_BARE,
    BANK_MAP_EXPANDED, BANK_MAP_STANDARD,
};
pub use rtc;
pub use rtc::RtcMsm6242b;

use serde::{Deserialize, Serialize};

/// Maximum size of physical memory regions
pub const CHIP_RAM_SIZE_512K: usize = 512 * 1024;
pub const SLOW_RAM_SIZE: usize = 512 * 1024;
pub const MAX_FAST_RAM_SIZE: usize = 4 * 1024 * 1024; // 4MB Fast RAM (Auto-Config expansion at $200000..$5FFFFF)
pub const KICKSTART_SIZE_256K: usize = 256 * 1024;

/// Classification of a 64 KB physical memory bank
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryBank {
    /// Chip RAM ($000000-$07FFFF)
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
        let bank_map = map::build_bank_map(&config);

        let mut bus = Self {
            config,
            bank_map,
            chip_ram: vec![0x00; chip_ram_size],
            slow_ram,
            fast_ram,
            kickstart_rom: vec![0xFF; KICKSTART_SIZE_256K],
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

    /// Checks whether an address targets Chip RAM or contention-affected Slow RAM
    #[inline(always)]
    pub fn is_chip_ram_target(&self, addr: u32) -> bool {
        let bank_idx = ((addr >> 16) & 0xFF) as usize;
        matches!(
            self.bank_map[bank_idx].bank,
            MemoryBank::ChipRam | MemoryBank::SlowRam
        )
    }

    /// Reads an 8-bit byte from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_byte(&self, addr: u32) -> BusResult<u8> {
        let addr = addr & 0x00FF_FFFF;
        (self.bank_map[(addr >> 16) as usize].read_byte)(self, addr)
    }

    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn read_word(&self, addr: u32) -> BusResult<u16> {
        let addr = addr & 0x00FF_FFFF;
        (self.bank_map[(addr >> 16) as usize].read_word)(self, addr)
    }

    /// Writes an 8-bit byte to the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & 0x00FF_FFFF;
        let write_fn = self.bank_map[(addr >> 16) as usize].write_byte;
        write_fn(self, addr, val)
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space, delegating directly to the bank handler.
    #[inline(always)]
    pub fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let addr = addr & 0x00FF_FFFF;
        let write_fn = self.bank_map[(addr >> 16) as usize].write_word;
        write_fn(self, addr, val)
    }

    /// Injects or writes an arbitrary contiguous byte block into physical memory regions
    /// (Chip RAM, Fast RAM, Slow RAM, or Kickstart ROM) using direct slice operations.
    /// Returns the number of bytes written.
    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) -> usize {
        if data.is_empty() {
            return 0;
        }
        let addr = addr & 0x00FF_FFFF;

        // 1. Chip RAM ($000000..$07FFFF, or up to configured chip_ram size)
        if (addr as usize) < self.chip_ram.len() {
            let start = addr as usize;
            let end = (start + data.len()).min(self.chip_ram.len());
            let len = end - start;
            self.chip_ram[start..end].copy_from_slice(&data[..len]);
            return len;
        }

        // 2. Auto-Config Fast RAM ($200000..$9FFFFF)
        if (0x0020_0000..0x00A0_0000).contains(&addr) {
            if let Some(ref mut fast_ram) = self.fast_ram {
                let offset = (addr - 0x0020_0000) as usize;
                if offset < fast_ram.len() {
                    let end = (offset + data.len()).min(fast_ram.len());
                    let len = end - offset;
                    fast_ram[offset..end].copy_from_slice(&data[..len]);
                    return len;
                }
            }
            return 0;
        }

        // 3. Slow / Trapdoor RAM ($C00000..$C7FFFF)
        if (0x00C0_0000..0x00C8_0000).contains(&addr) {
            if let Some(ref mut slow_ram) = self.slow_ram {
                let offset = (addr - 0x00C0_0000) as usize;
                if offset < slow_ram.len() {
                    let end = (offset + data.len()).min(slow_ram.len());
                    let len = end - offset;
                    slow_ram[offset..end].copy_from_slice(&data[..len]);
                    return len;
                }
            }
            return 0;
        }

        // 4. Kickstart ROM ($F80000..$FFFFFF)
        if addr >= 0x00F8_0000 {
            if (addr == 0x00F8_0000 || addr == 0x00FC_0000) && data.len() >= 256 * 1024 {
                self.kickstart_rom = data.to_vec();
                return data.len();
            } else {
                if self.kickstart_rom.is_empty() {
                    self.kickstart_rom = vec![0xFF; KICKSTART_SIZE_256K];
                }
                let mask = self.kickstart_rom.len() - 1;
                let start = (addr as usize) & mask;
                if start + data.len() <= self.kickstart_rom.len() {
                    self.kickstart_rom[start..start + data.len()].copy_from_slice(data);
                    return data.len();
                } else {
                    for (i, &b) in data.iter().enumerate() {
                        let idx = (start + i) & mask;
                        self.kickstart_rom[idx] = b;
                    }
                    return data.len();
                }
            }
        }

        0
    }

    /// Side-effect-free byte read for debugger inspection and test result assertions
    #[inline(always)]
    pub fn read_byte_debug(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        match self.bank_map[bank_idx].bank {
            MemoryBank::ChipRam => self.chip_ram[addr as usize],
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &self.fast_ram {
                    fast_ram[(addr - 0x200000) as usize]
                } else {
                    self.unmapped_byte
                }
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &self.slow_ram {
                    slow_ram[(addr - 0xC00000) as usize]
                } else {
                    self.unmapped_byte
                }
            }
            MemoryBank::KickstartRom => {
                let mask = self.kickstart_rom.len() - 1;
                self.kickstart_rom[(addr as usize) & mask]
            }
            MemoryBank::OpenBus | MemoryBank::Cia | MemoryBank::Rtc | MemoryBank::CustomChips => {
                self.unmapped_byte
            }
        }
    }

    /// Side-effect-free word read for disassemblers, debugger inspection, and test result assertions
    #[inline(always)]
    pub fn read_word_debug(&self, addr: u32) -> u16 {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        match self.bank_map[bank_idx].bank {
            MemoryBank::ChipRam => {
                let idx = addr as usize;
                u16::from_be_bytes([self.chip_ram[idx], self.chip_ram[idx + 1]])
            }
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &self.fast_ram {
                    let offset = (addr - 0x200000) as usize;
                    u16::from_be_bytes([fast_ram[offset], fast_ram[offset + 1]])
                } else {
                    let b = self.unmapped_byte as u16;
                    (b << 8) | b
                }
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &self.slow_ram {
                    let offset = (addr - 0xC00000) as usize;
                    u16::from_be_bytes([slow_ram[offset], slow_ram[offset + 1]])
                } else {
                    let b = self.unmapped_byte as u16;
                    (b << 8) | b
                }
            }
            MemoryBank::KickstartRom => {
                let mask = self.kickstart_rom.len() - 1;
                let idx = (addr as usize) & mask;
                if idx + 1 < self.kickstart_rom.len() {
                    u16::from_be_bytes([self.kickstart_rom[idx], self.kickstart_rom[idx + 1]])
                } else {
                    u16::from_be_bytes([self.kickstart_rom[idx], self.kickstart_rom[0]])
                }
            }
            MemoryBank::OpenBus | MemoryBank::Cia | MemoryBank::Rtc | MemoryBank::CustomChips => {
                let b = self.unmapped_byte as u16;
                (b << 8) | b
            }
        }
    }

    /// Side-effect-free byte write for debugger modification
    #[inline(always)]
    pub fn write_byte_debug(&mut self, addr: u32, val: u8) {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        match self.bank_map[bank_idx].bank {
            MemoryBank::ChipRam => self.chip_ram[addr as usize] = val,
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &mut self.fast_ram {
                    fast_ram[(addr - 0x200000) as usize] = val;
                }
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &mut self.slow_ram {
                    slow_ram[(addr - 0xC00000) as usize] = val;
                }
            }
            MemoryBank::KickstartRom
            | MemoryBank::OpenBus
            | MemoryBank::Cia
            | MemoryBank::Rtc
            | MemoryBank::CustomChips => {}
        }
    }

    /// Side-effect-free word write for debugger modification
    #[inline(always)]
    pub fn write_word_debug(&mut self, addr: u32, val: u16) {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        let bytes = val.to_be_bytes();
        match self.bank_map[bank_idx].bank {
            MemoryBank::ChipRam => {
                let idx = addr as usize;
                self.chip_ram[idx] = bytes[0];
                self.chip_ram[idx + 1] = bytes[1];
            }
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &mut self.fast_ram {
                    let offset = (addr - 0x200000) as usize;
                    fast_ram[offset] = bytes[0];
                    fast_ram[offset + 1] = bytes[1];
                }
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &mut self.slow_ram {
                    let offset = (addr - 0xC00000) as usize;
                    slow_ram[offset] = bytes[0];
                    slow_ram[offset + 1] = bytes[1];
                }
            }
            MemoryBank::KickstartRom
            | MemoryBank::OpenBus
            | MemoryBank::Cia
            | MemoryBank::Rtc
            | MemoryBank::CustomChips => {}
        }
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
