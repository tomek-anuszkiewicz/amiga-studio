//! Complete 24-bit physical memory map, address decoders, and hardware quirks

use super::MemoryBus;

impl MemoryBus {
    /// Reads a 16-bit Big-Endian word from the 24-bit physical address space
    pub(crate) fn read_word_internal(&self, addr: u32) -> u16 {
        let b0 = self.read_byte_internal(addr);
        let b1 = self.read_byte_internal(addr.wrapping_add(1));
        u16::from_be_bytes([b0, b1])
    }

    /// Reads an 8-bit byte from the 24-bit physical address space
    pub(crate) fn read_byte_internal(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FF_FFFF;

        // 1. Boot Overlay or Chip RAM ($000000-$07FFFF or $000000-$0FFFFF)
        if addr < 0x100000 {
            if self.low_memory_overlay && addr < 0x080000 {
                // Low-memory overlay routes accesses to Kickstart ROM
                return self.read_kickstart_byte(addr);
            }
            if (addr as usize) < self.chip_ram.len() {
                return self.chip_ram[addr as usize];
            }
            // Unmapped extended chip RAM on stock 512KB OCS
            return 0xFF;
        }

        // 2. Fast RAM expansion ($200000-$9FFFFF)
        if (0x200000..=0x9FFFFF).contains(&addr) {
            if let Some(fast_ram) = &self.fast_ram {
                let offset = (addr - 0x200000) as usize;
                if offset < fast_ram.len() {
                    return fast_ram[offset];
                }
            }
            return 0xFF; // Floating bus
        }

        // 3. CIA-B ($BFD000-$BFDF00): Even byte addresses (A0 = 0)
        if (0xBFD000..=0xBFDF00).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as usize;
                return self.cia_b_registers[reg];
            }
            return 0xFF; // Odd bytes return open bus $FF
        }

        // 4. CIA-A ($BFE001-$BFEF01): Odd byte addresses (A0 = 1)
        if (0xBFE001..=0xBFEF01).contains(&addr) {
            if (addr & 1) == 1 {
                let reg = ((addr >> 8) & 0x0F) as usize;
                return self.cia_a_registers[reg];
            }
            return 0xFF; // Even bytes return open bus $FF
        }

        // 5. Slow RAM ($C00000-$C7FFFF)
        if (0xC00000..=0xC7FFFF).contains(&addr) {
            if let Some(slow_ram) = &self.slow_ram {
                let offset = (addr - 0xC00000) as usize;
                if offset < slow_ram.len() {
                    return slow_ram[offset];
                }
            }
            return 0xFF;
        }

        // 6. Custom Chip Registers ($DFF000-$DFFFFE)
        if (0xDFF000..=0xDFFFFF).contains(&addr) {
            let word_idx = ((addr & 0x1FE) >> 1) as usize;
            let reg_val = self.custom_registers[word_idx];
            return if (addr & 1) == 0 {
                (reg_val >> 8) as u8
            } else {
                (reg_val & 0xFF) as u8
            };
        }

        // 7. Kickstart ROM ($F80000-$FFFFFF)
        if addr >= 0xF80000 {
            return self.read_kickstart_byte(addr - 0xF80000);
        }

        // Unmapped open bus
        0xFF
    }

    /// Writes a 16-bit Big-Endian word to the 24-bit physical address space
    pub(crate) fn write_word_internal(&mut self, addr: u32, data: u16) {
        let bytes = data.to_be_bytes();
        self.write_byte_internal(addr, bytes[0]);
        self.write_byte_internal(addr.wrapping_add(1), bytes[1]);
    }

    /// Writes an 8-bit byte to the 24-bit physical address space
    pub(crate) fn write_byte_internal(&mut self, addr: u32, val: u8) {
        let addr = addr & 0x00FF_FFFF;

        // 1. Chip RAM ($000000-$07FFFF or $000000-$0FFFFF)
        if addr < 0x100000 {
            if self.low_memory_overlay && addr < 0x080000 {
                // Writes to Kickstart ROM space during overlay are discarded
                return;
            }
            if (addr as usize) < self.chip_ram.len() {
                self.chip_ram[addr as usize] = val;
            }
            return;
        }

        // 2. Fast RAM ($200000-$9FFFFF)
        if (0x200000..=0x9FFFFF).contains(&addr) {
            if let Some(fast_ram) = &mut self.fast_ram {
                let offset = (addr - 0x200000) as usize;
                if offset < fast_ram.len() {
                    fast_ram[offset] = val;
                }
            }
            return;
        }

        // 3. CIA-B ($BFD000-$BFDF00)
        if (0xBFD000..=0xBFDF00).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as usize;
                self.cia_b_registers[reg] = val;
            }
            return;
        }

        // 4. CIA-A ($BFE001-$BFEF01)
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
            return;
        }

        // 5. Slow RAM ($C00000-$C7FFFF)
        if (0xC00000..=0xC7FFFF).contains(&addr) {
            if let Some(slow_ram) = &mut self.slow_ram {
                let offset = (addr - 0xC00000) as usize;
                if offset < slow_ram.len() {
                    slow_ram[offset] = val;
                }
            }
            return;
        }

        // 6. Custom Chip Registers ($DFF000-$DFFFFE)
        if (0xDFF000..=0xDFFFFF).contains(&addr) {
            let word_idx = ((addr & 0x1FE) >> 1) as usize;
            let current = self.custom_registers[word_idx];
            self.custom_registers[word_idx] = if (addr & 1) == 0 {
                ((val as u16) << 8) | (current & 0x00FF)
            } else {
                (current & 0xFF00) | (val as u16)
            };
            return;
        }

        // Writes to ROM or unmapped addresses are silent no-ops
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
