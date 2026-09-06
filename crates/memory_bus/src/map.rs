//! Complete 24-bit physical memory map, address decoders, and hardware quirks
//!
//! Provides single-instruction O(1) memory bank dispatch using a 256-entry direct lookup table.

use super::{MemoryBank, MemoryBus};
use config::{A500Config, A500Preset};

/// Precalculates the 256-entry 64 KB memory bank dispatch table for a given preset at compile time
pub const fn build_preset_bank_map(preset: A500Preset) -> [MemoryBank; 256] {
    let mut map = [MemoryBank::OpenBus; 256];

    // 1. Chip RAM: 512 KB occupies banks 0x00..=0x07 (8 banks of 64 KB)
    let mut b = 0x00;
    while b <= 0x07 {
        map[b] = MemoryBank::ChipRam;
        b += 1;
    }

    // 2. Fast RAM: 4 MB occupies banks 0x20..=0x5F (64 banks of 64 KB)
    if matches!(preset, A500Preset::ExpandedPowerUser) {
        let mut b = 0x20;
        while b <= 0x5F {
            map[b] = MemoryBank::FastRam;
            b += 1;
        }
    }

    // 3. CIA registers: bank 0xBF ($BF0000-$BFFFFF)
    map[0xBF] = MemoryBank::Cia;

    // 4. Slow RAM: 512 KB occupies banks 0xC0..=0xC7 (8 banks of 64 KB)
    if matches!(preset, A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser) {
        let mut b = 0xC0;
        while b <= 0xC7 {
            map[b] = MemoryBank::SlowRam;
            b += 1;
        }
    }

    // 5. RTC: bank 0xDC (at $DC0000..=$DC003F)
    if matches!(preset, A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser) {
        map[0xDC] = MemoryBank::Rtc;
    }

    // 6. Custom chip registers: bank 0xDF (at $DFF000..=$DFFFFE)
    map[0xDF] = MemoryBank::CustomChips;

    // 7. Kickstart ROM: 512 KB occupies banks 0xF8..=0xFF (8 banks of 64 KB)
    let mut b = 0xF8;
    while b <= 0xFF {
        map[b] = MemoryBank::KickstartRom;
        b += 1;
    }

    map
}

/// Static compile-time bank dispatch table for Preset 1 (Bare Stock 512 KB)
pub static BANK_MAP_BARE: [MemoryBank; 256] = build_preset_bank_map(A500Preset::Bare512k);

/// Static compile-time bank dispatch table for Preset 2 (Standard 1 MB + RTC)
pub static BANK_MAP_STANDARD: [MemoryBank; 256] = build_preset_bank_map(A500Preset::Standard1Mb);

/// Static compile-time bank dispatch table for Preset 3 (Expanded Power User: 1 MB + 4 MB Fast + RTC)
pub static BANK_MAP_EXPANDED: [MemoryBank; 256] = build_preset_bank_map(A500Preset::ExpandedPowerUser);

/// Returns a reference to the static compile-time bank dispatch table for the given preset
#[inline(always)]
pub const fn get_preset_bank_map(preset: A500Preset) -> &'static [MemoryBank; 256] {
    match preset {
        A500Preset::Bare512k => &BANK_MAP_BARE,
        A500Preset::Standard1Mb => &BANK_MAP_STANDARD,
        A500Preset::ExpandedPowerUser => &BANK_MAP_EXPANDED,
    }
}

/// Returns the 256-entry 64 KB memory bank dispatch table based on active configuration
#[inline(always)]
pub fn build_bank_map(config: &A500Config) -> [MemoryBank; 256] {
    *get_preset_bank_map(config.active_preset())
}

impl MemoryBus {
    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space
    #[inline(always)]
    pub(crate) fn read_word_internal(&self, addr: u32) -> u16 {
        let b0 = self.read_byte_internal(addr);
        let b1 = self.read_byte_internal(addr.wrapping_add(1));
        u16::from_be_bytes([b0, b1])
    }

    /// Reads an 8-bit byte from the 24-bit physical address space using the 256-entry bank table
    #[inline(always)]
    pub(crate) fn read_byte_internal(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        let bank = self.bank_map[bank_idx];

        match bank {
            MemoryBank::ChipRam => {
                if self.low_memory_overlay && addr < 0x080000 {
                    return self.read_kickstart_byte(addr);
                }
                if (addr as usize) < self.chip_ram.len() {
                    self.chip_ram[addr as usize]
                } else {
                    0xFF
                }
            }
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &self.fast_ram {
                    let offset = (addr - 0x200000) as usize;
                    if offset < fast_ram.len() {
                        return fast_ram[offset];
                    }
                }
                0xFF
            }
            MemoryBank::Cia => {
                // CIA-B ($BFD000-$BFDF00): Even byte addresses (A0 = 0)
                if (0xBFD000..=0xBFDF00).contains(&addr) {
                    if (addr & 1) == 0 {
                        let reg = ((addr >> 8) & 0x0F) as usize;
                        return self.cia_b_registers[reg];
                    }
                    return 0xFF;
                }
                // CIA-A ($BFE001-$BFEF01): Odd byte addresses (A0 = 1)
                if (0xBFE001..=0xBFEF01).contains(&addr) {
                    if (addr & 1) == 1 {
                        let reg = ((addr >> 8) & 0x0F) as usize;
                        return self.cia_a_registers[reg];
                    }
                    return 0xFF;
                }
                0xFF
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &self.slow_ram {
                    let offset = (addr - 0xC00000) as usize;
                    if offset < slow_ram.len() {
                        return slow_ram[offset];
                    }
                }
                0xFF
            }
            MemoryBank::Rtc => {
                if (0xDC0000..=0xDC003F).contains(&addr) {
                    let reg = ((addr >> 2) & 0x0F) as usize;
                    self.rtc_registers[reg] & 0x0F
                } else {
                    0xFF
                }
            }
            MemoryBank::CustomChips => {
                if (0xDFF000..=0xDFFFFF).contains(&addr) {
                    let word_idx = ((addr & 0x1FE) >> 1) as usize;
                    let reg_val = self.custom_registers[word_idx];
                    if (addr & 1) == 0 {
                        (reg_val >> 8) as u8
                    } else {
                        (reg_val & 0xFF) as u8
                    }
                } else {
                    0xFF
                }
            }
            MemoryBank::KickstartRom => self.read_kickstart_byte(addr - 0xF80000),
            MemoryBank::OpenBus => 0xFF,
        }
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space
    #[inline(always)]
    pub(crate) fn write_word_internal(&mut self, addr: u32, data: u16) {
        let bytes = data.to_be_bytes();
        self.write_byte_internal(addr, bytes[0]);
        self.write_byte_internal(addr.wrapping_add(1), bytes[1]);
    }

    /// Writes an 8-bit byte to the 24-bit physical address space using the 256-entry bank table
    #[inline(always)]
    pub(crate) fn write_byte_internal(&mut self, addr: u32, val: u8) {
        let addr = addr & 0x00FF_FFFF;
        let bank_idx = (addr >> 16) as usize;
        let bank = self.bank_map[bank_idx];

        match bank {
            MemoryBank::ChipRam => {
                if self.low_memory_overlay && addr < 0x080000 {
                    // Writes to Kickstart ROM space during overlay are discarded
                    return;
                }
                if (addr as usize) < self.chip_ram.len() {
                    self.chip_ram[addr as usize] = val;
                }
            }
            MemoryBank::FastRam => {
                if let Some(fast_ram) = &mut self.fast_ram {
                    let offset = (addr - 0x200000) as usize;
                    if offset < fast_ram.len() {
                        fast_ram[offset] = val;
                    }
                }
            }
            MemoryBank::Cia => {
                // CIA-B ($BFD000-$BFDF00)
                if (0xBFD000..=0xBFDF00).contains(&addr) {
                    if (addr & 1) == 0 {
                        let reg = ((addr >> 8) & 0x0F) as usize;
                        self.cia_b_registers[reg] = val;
                    }
                    return;
                }
                // CIA-A ($BFE001-$BFEF01)
                if (0xBFE001..=0xBFEF01).contains(&addr) {
                    if (addr & 1) == 1 {
                        let reg = ((addr >> 8) & 0x0F) as usize;
                        self.cia_a_registers[reg] = val;
                        // CIA-A bit 0 of Port A ($BFE001) controls the low-memory overlay (_OVL)
                        if reg == 0 {
                            if (val & 0x01) == 0 {
                                self.map_kickstart_to_low_memory();
                            } else {
                                self.map_chip_ram_to_low_memory();
                            }
                        }
                    }
                }
            }
            MemoryBank::SlowRam => {
                if let Some(slow_ram) = &mut self.slow_ram {
                    let offset = (addr - 0xC00000) as usize;
                    if offset < slow_ram.len() {
                        slow_ram[offset] = val;
                    }
                }
            }
            MemoryBank::Rtc => {
                if (0xDC0000..=0xDC003F).contains(&addr) {
                    let reg = ((addr >> 2) & 0x0F) as usize;
                    self.rtc_registers[reg] = val & 0x0F;
                }
            }
            MemoryBank::CustomChips => {
                if (0xDFF000..=0xDFFFFF).contains(&addr) {
                    let word_idx = ((addr & 0x1FE) >> 1) as usize;
                    let current = self.custom_registers[word_idx];
                    self.custom_registers[word_idx] = if (addr & 1) == 0 {
                        ((val as u16) << 8) | (current & 0x00FF)
                    } else {
                        (current & 0xFF00) | (val as u16)
                    };
                }
            }
            MemoryBank::KickstartRom | MemoryBank::OpenBus => {
                // Writes to ROM or unmapped open bus are silent no-ops
            }
        }
    }

    /// Read byte from Kickstart ROM (handling 256KB and 512KB mirroring)
    #[inline]
    fn read_kickstart_byte(&self, offset: u32) -> u8 {
        if self.kickstart_rom.is_empty() {
            return 0xFF;
        }
        let rom_len = self.kickstart_rom.len();
        let mask = (rom_len - 1) as u32;
        let idx = (offset & mask) as usize;
        if idx < rom_len {
            self.kickstart_rom[idx]
        } else {
            0xFF
        }
    }

    /// Handles the Amiga TAS unbroken RMW hardware bug:
    /// In Chip RAM and Slow RAM, Gary / Agnus fails to latch the write phase, dropping the write.
    /// In Fast RAM, the write phase succeeds.
    pub fn write_tas_byte(&mut self, addr: u32, data: u8) {
        let addr = addr & 0x00FF_FFFF;
        // Check if target is Chip RAM or Slow RAM
        if addr < 0x100000 || (0xC00000..=0xC7FFFF).contains(&addr) {
            // Hardware bug: Gary drops the write phase. Memory is unmodified.
            return;
        }
        // In Fast RAM, write succeeds
        self.write_byte_internal(addr, data);
    }
}
