//! Complete 24-bit physical memory map, address decoders, and hardware quirks
//!
//! Provides single-instruction O(1) memory bank dispatch using a 256-entry table
//! of direct function pointers to bank read/write handler methods.

use super::{MemoryBank, PhysicalMemory};
use config::{A500Config, A500Preset};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Function pointer signature for an 8-bit memory bank read handler
pub type BankReadByteFn = fn(&PhysicalMemory, u32) -> u8;

/// Function pointer signature for an 8-bit memory bank write handler
pub type BankWriteByteFn = fn(&mut PhysicalMemory, u32, u8);

/// Function pointer signature for a 16-bit memory bank read handler
pub type BankReadWordFn = fn(&PhysicalMemory, u32) -> u16;

/// Function pointer signature for a 16-bit memory bank write handler
pub type BankWriteWordFn = fn(&mut PhysicalMemory, u32, u16);

/// Memory bank handler containing method pointers for direct dispatch
#[derive(Clone, Copy)]
pub struct BankHandler {
    /// Associated memory bank classification
    pub bank: MemoryBank,
    /// Flag indicating if this memory bank is subject to Agnus/DMA bus contention
    pub is_contended: bool,
    /// Direct read byte handler method pointer
    pub read_byte: BankReadByteFn,
    /// Direct write byte handler method pointer
    pub write_byte: BankWriteByteFn,
    /// Direct read word handler method pointer
    pub read_word: BankReadWordFn,
    /// Direct write word handler method pointer
    pub write_word: BankWriteWordFn,
}

impl PartialEq for BankHandler {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.bank == other.bank
    }
}

impl Eq for BankHandler {}

impl std::fmt::Debug for BankHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BankHandler({:?})", self.bank)
    }
}

impl Default for BankHandler {
    fn default() -> Self {
        OPEN_BUS_HANDLER
    }
}

impl PartialEq<MemoryBank> for BankHandler {
    #[inline(always)]
    fn eq(&self, other: &MemoryBank) -> bool {
        self.bank == *other
    }
}

impl PartialEq<BankHandler> for MemoryBank {
    #[inline(always)]
    fn eq(&self, other: &BankHandler) -> bool {
        *self == other.bank
    }
}

impl Serialize for BankHandler {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.bank.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BankHandler {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bank = MemoryBank::deserialize(deserializer)?;
        Ok(handler_for_bank(bank))
    }
}

// =============================================================================
// Individual Memory Bank Handler Implementations
// =============================================================================

/// Read handler for Chip RAM ($000000-$07FFFF)
#[inline(always)]
pub fn read_chip_ram(bus: &PhysicalMemory, addr: u32) -> u8 {
    bus.chip_ram[addr as usize]
}

/// Read word handler for Chip RAM ($000000-$07FFFF)
#[inline(always)]
pub fn read_chip_ram_word(bus: &PhysicalMemory, addr: u32) -> u16 {
    let idx = addr as usize;
    u16::from_be_bytes([bus.chip_ram[idx], bus.chip_ram[idx + 1]])
}

/// Write handler for Chip RAM ($000000-$07FFFF)
#[inline(always)]
pub fn write_chip_ram(bus: &mut PhysicalMemory, addr: u32, val: u8) {
    bus.chip_ram[addr as usize] = val;
}

/// Write word handler for Chip RAM ($000000-$07FFFF)
#[inline(always)]
pub fn write_chip_ram_word(bus: &mut PhysicalMemory, addr: u32, val: u16) {
    let idx = addr as usize;
    let bytes = val.to_be_bytes();
    bus.chip_ram[idx] = bytes[0];
    bus.chip_ram[idx + 1] = bytes[1];
}

/// Read handler for Auto-Config Fast RAM ($200000-$9FFFFF)
pub fn read_fast_ram(bus: &PhysicalMemory, addr: u32) -> u8 {
    if let Some(fast_ram) = &bus.fast_ram {
        let offset = (addr - 0x200000) as usize;
        fast_ram[offset]
    } else {
        0xFF
    }
}

/// Read word handler for Auto-Config Fast RAM ($200000-$9FFFFF)
pub fn read_fast_ram_word(bus: &PhysicalMemory, addr: u32) -> u16 {
    if let Some(fast_ram) = &bus.fast_ram {
        let offset = (addr - 0x200000) as usize;
        u16::from_be_bytes([fast_ram[offset], fast_ram[offset + 1]])
    } else {
        0xFFFF
    }
}

/// Write handler for Auto-Config Fast RAM ($200000-$9FFFFF)
pub fn write_fast_ram(bus: &mut PhysicalMemory, addr: u32, val: u8) {
    if let Some(fast_ram) = &mut bus.fast_ram {
        let offset = (addr - 0x200000) as usize;
        fast_ram[offset] = val;
    }
}

/// Write word handler for Auto-Config Fast RAM ($200000-$9FFFFF)
pub fn write_fast_ram_word(bus: &mut PhysicalMemory, addr: u32, val: u16) {
    if let Some(fast_ram) = &mut bus.fast_ram {
        let offset = (addr - 0x200000) as usize;
        let bytes = val.to_be_bytes();
        fast_ram[offset] = bytes[0];
        fast_ram[offset + 1] = bytes[1];
    }
}

/// Read handler for Slow / Trapdoor RAM ($C00000-$C7FFFF)
pub fn read_slow_ram(bus: &PhysicalMemory, addr: u32) -> u8 {
    if let Some(slow_ram) = &bus.slow_ram {
        let offset = (addr - 0xC00000) as usize;
        slow_ram[offset]
    } else {
        0xFF
    }
}

/// Read word handler for Slow / Trapdoor RAM ($C00000-$C7FFFF)
pub fn read_slow_ram_word(bus: &PhysicalMemory, addr: u32) -> u16 {
    if let Some(slow_ram) = &bus.slow_ram {
        let offset = (addr - 0xC00000) as usize;
        u16::from_be_bytes([slow_ram[offset], slow_ram[offset + 1]])
    } else {
        0xFFFF
    }
}

/// Write handler for Slow / Trapdoor RAM ($C00000-$C7FFFF)
pub fn write_slow_ram(bus: &mut PhysicalMemory, addr: u32, val: u8) {
    if let Some(slow_ram) = &mut bus.slow_ram {
        let offset = (addr - 0xC00000) as usize;
        slow_ram[offset] = val;
    }
}

/// Write word handler for Slow / Trapdoor RAM ($C00000-$C7FFFF)
pub fn write_slow_ram_word(bus: &mut PhysicalMemory, addr: u32, val: u16) {
    if let Some(slow_ram) = &mut bus.slow_ram {
        let offset = (addr - 0xC00000) as usize;
        let bytes = val.to_be_bytes();
        slow_ram[offset] = bytes[0];
        slow_ram[offset + 1] = bytes[1];
    }
}

/// Read handler for Kickstart ROM ($F80000-$FFFFFF, mirrored at $000000 during boot overlay)
pub fn read_kickstart_rom(bus: &PhysicalMemory, addr: u32) -> u8 {
    bus.read_kickstart_byte(addr)
}

/// Read word handler for Kickstart ROM ($F80000-$FFFFFF, mirrored at $000000 during boot overlay)
pub fn read_kickstart_rom_word(bus: &PhysicalMemory, addr: u32) -> u16 {
    bus.read_kickstart_word(addr)
}

/// Write handler for Kickstart ROM (ROM writes are silent no-ops)
pub fn write_kickstart_rom(_bus: &mut PhysicalMemory, _addr: u32, _val: u8) {}

/// Write word handler for Kickstart ROM (ROM writes are silent no-ops)
pub fn write_kickstart_rom_word(_bus: &mut PhysicalMemory, _addr: u32, _val: u16) {}

/// Read handler for unmapped Open Bus (returns floating bus byte, defaults to $FF)
pub fn read_open_bus(bus: &PhysicalMemory, _addr: u32) -> u8 {
    bus.unmapped_byte
}

/// Read word handler for unmapped Open Bus (returns floating bus word, defaults to $FFFF)
pub fn read_open_bus_word(bus: &PhysicalMemory, _addr: u32) -> u16 {
    let b = bus.unmapped_byte as u16;
    (b << 8) | b
}

/// Write handler for unmapped Open Bus (writes are silent no-ops)
pub fn write_open_bus(_bus: &mut PhysicalMemory, _addr: u32, _val: u8) {}

/// Write word handler for unmapped Open Bus (writes are silent no-ops)
pub fn write_open_bus_word(_bus: &mut PhysicalMemory, _addr: u32, _val: u16) {}

// =============================================================================
// Static Handler Definitions
// =============================================================================

pub const CHIP_RAM_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::ChipRam,
    is_contended: true,
    read_byte: read_chip_ram,
    write_byte: write_chip_ram,
    read_word: read_chip_ram_word,
    write_word: write_chip_ram_word,
};

pub const FAST_RAM_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::FastRam,
    is_contended: false,
    read_byte: read_fast_ram,
    write_byte: write_fast_ram,
    read_word: read_fast_ram_word,
    write_word: write_fast_ram_word,
};

pub const CIA_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::Cia,
    is_contended: false,
    read_byte: read_open_bus,
    write_byte: write_open_bus,
    read_word: read_open_bus_word,
    write_word: write_open_bus_word,
};

pub const SLOW_RAM_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::SlowRam,
    is_contended: true,
    read_byte: read_slow_ram,
    write_byte: write_slow_ram,
    read_word: read_slow_ram_word,
    write_word: write_slow_ram_word,
};

pub const RTC_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::Rtc,
    is_contended: false,
    read_byte: read_open_bus,
    write_byte: write_open_bus,
    read_word: read_open_bus_word,
    write_word: write_open_bus_word,
};

pub const CUSTOM_CHIPS_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::CustomChips,
    is_contended: false,
    read_byte: read_open_bus,
    write_byte: write_open_bus,
    read_word: read_open_bus_word,
    write_word: write_open_bus_word,
};

pub const KICKSTART_ROM_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::KickstartRom,
    is_contended: false,
    read_byte: read_kickstart_rom,
    write_byte: write_kickstart_rom,
    read_word: read_kickstart_rom_word,
    write_word: write_kickstart_rom_word,
};

pub const OPEN_BUS_HANDLER: BankHandler = BankHandler {
    bank: MemoryBank::OpenBus,
    is_contended: false,
    read_byte: read_open_bus,
    write_byte: write_open_bus,
    read_word: read_open_bus_word,
    write_word: write_open_bus_word,
};

/// Maps a `MemoryBank` classification to its direct method handler
pub const fn handler_for_bank(bank: MemoryBank) -> BankHandler {
    match bank {
        MemoryBank::ChipRam => CHIP_RAM_HANDLER,
        MemoryBank::FastRam => FAST_RAM_HANDLER,
        MemoryBank::Cia => CIA_HANDLER,
        MemoryBank::SlowRam => SLOW_RAM_HANDLER,
        MemoryBank::Rtc => RTC_HANDLER,
        MemoryBank::CustomChips => CUSTOM_CHIPS_HANDLER,
        MemoryBank::KickstartRom => KICKSTART_ROM_HANDLER,
        MemoryBank::OpenBus => OPEN_BUS_HANDLER,
    }
}

// =============================================================================
// 256-Entry Bank Dispatch Table Construction
// =============================================================================

/// Precalculates the 256-entry 64 KB memory bank dispatch table for a given preset at compile time
pub const fn build_preset_bank_map(preset: A500Preset) -> [BankHandler; 256] {
    let mut map = [OPEN_BUS_HANDLER; 256];

    // 1. Chip RAM: 512 KB occupies banks 0x00..=0x07 (8 banks of 64 KB)
    let mut b = 0x00;
    while b <= 0x07 {
        map[b] = CHIP_RAM_HANDLER;
        b += 1;
    }

    // 2. Fast RAM: 4 MB occupies banks 0x20..=0x5F (64 banks of 64 KB)
    if matches!(preset, A500Preset::ExpandedPowerUser) {
        let mut b = 0x20;
        while b <= 0x5F {
            map[b] = FAST_RAM_HANDLER;
            b += 1;
        }
    }

    // 3. CIA registers: bank 0xBF ($BF0000-$BFFFFF)
    map[0xBF] = CIA_HANDLER;

    // 4. Slow RAM: 512 KB occupies banks 0xC0..=0xC7 (8 banks of 64 KB)
    if matches!(
        preset,
        A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser
    ) {
        let mut b = 0xC0;
        while b <= 0xC7 {
            map[b] = SLOW_RAM_HANDLER;
            b += 1;
        }
    }

    // 5. RTC: bank 0xDC (at $DC0000..=$DC003F)
    if matches!(
        preset,
        A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser
    ) {
        map[0xDC] = RTC_HANDLER;
    }

    // 6. Custom chip registers: bank 0xDF (at $DFF000..=$DFFFFE)
    map[0xDF] = CUSTOM_CHIPS_HANDLER;

    // 7. Kickstart ROM: 512 KB occupies banks 0xF8..=0xFF (8 banks of 64 KB)
    let mut b = 0xF8;
    while b <= 0xFF {
        map[b] = KICKSTART_ROM_HANDLER;
        b += 1;
    }

    map
}

/// Static compile-time bank dispatch table for Preset 1 (Bare Stock 512 KB)
pub static BANK_MAP_BARE: [BankHandler; 256] = build_preset_bank_map(A500Preset::Bare512k);

/// Static compile-time bank dispatch table for Preset 2 (Standard 1 MB + RTC)
pub static BANK_MAP_STANDARD: [BankHandler; 256] = build_preset_bank_map(A500Preset::Standard1Mb);

/// Static compile-time bank dispatch table for Preset 3 (Expanded Power User: 1 MB + 4 MB Fast + RTC)
pub static BANK_MAP_EXPANDED: [BankHandler; 256] =
    build_preset_bank_map(A500Preset::ExpandedPowerUser);

/// Returns a reference to the static compile-time bank dispatch table for the given preset
#[inline(always)]
pub const fn get_preset_bank_map(preset: A500Preset) -> &'static [BankHandler; 256] {
    match preset {
        A500Preset::Bare512k => &BANK_MAP_BARE,
        A500Preset::Standard1Mb => &BANK_MAP_STANDARD,
        A500Preset::ExpandedPowerUser => &BANK_MAP_EXPANDED,
    }
}

/// Returns the 256-entry 64 KB memory bank dispatch table based on active configuration
#[inline(always)]
pub fn build_bank_map(config: &A500Config) -> [BankHandler; 256] {
    *get_preset_bank_map(config.active_preset())
}

// =============================================================================
// PhysicalMemory Method Implementations
// =============================================================================

impl PhysicalMemory {
    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space
    #[inline(always)]
    pub(crate) fn read_word_internal(&self, addr: u32) -> u16 {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        (self.bank_map[bank_idx].read_word)(self, addr)
    }

    /// Reads an 8-bit byte using direct function pointer dispatch from the 256-entry bank table
    #[inline(always)]
    pub(crate) fn read_byte_internal(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        (self.bank_map[bank_idx].read_byte)(self, addr)
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space
    #[inline(always)]
    pub(crate) fn write_word_internal(&mut self, addr: u32, data: u16) {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        (self.bank_map[bank_idx].write_word)(self, addr, data);
    }

    /// Read 16-bit word from Kickstart ROM (256 KB, mirrored across $F80000..$FFFFFF)
    #[inline]
    pub(crate) fn read_kickstart_word(&self, offset: u32) -> u16 {
        if self.kickstart_rom.is_empty() {
            return 0xFFFF;
        }
        let rom_len = self.kickstart_rom.len();
        let mask = (rom_len - 1) as u32;
        let idx = (offset & mask) as usize;
        if idx + 1 < rom_len {
            u16::from_be_bytes([self.kickstart_rom[idx], self.kickstart_rom[idx + 1]])
        } else {
            let b0 = self.read_kickstart_byte(offset);
            let b1 = self.read_kickstart_byte(offset.wrapping_add(1));
            u16::from_be_bytes([b0, b1])
        }
    }

    /// Writes an 8-bit byte using direct function pointer dispatch from the 256-entry bank table
    #[inline(always)]
    pub(crate) fn write_byte_internal(&mut self, addr: u32, val: u8) {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        (self.bank_map[bank_idx].write_byte)(self, addr, val);
    }

    /// Read byte from Kickstart ROM (256 KB, mirrored across $F80000..$FFFFFF)
    #[inline]
    pub(crate) fn read_kickstart_byte(&self, offset: u32) -> u8 {
        if self.kickstart_rom.is_empty() {
            return 0xFF;
        }
        let rom_len = self.kickstart_rom.len();
        let mask = (rom_len - 1) as u32;
        let idx = (offset & mask) as usize;
        self.kickstart_rom[idx]
    }
}
