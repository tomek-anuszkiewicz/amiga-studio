//! Direct state injection and inspection methods for unit testing, test runners, and debuggers

use super::MemoryBus;

impl MemoryBus {
    /// Loads a sequence of [address, byte] tuples into physical memory without triggering bus cycles or latches
    pub fn load_test_ram(&mut self, entries: &[[u32; 2]]) {
        for entry in entries {
            let addr = entry[0] & 0x00FF_FFFF;
            let val = (entry[1] & 0xFF) as u8;

            // In SingleStepTests, memory is loaded directly regardless of overlay
            if (addr as usize) < self.chip_ram.len() {
                self.chip_ram[addr as usize] = val;
            } else if (0xC00000..=0xC7FFFF).contains(&addr) {
                let bank_idx = (addr >> 16) as usize;
                self.bank_map[bank_idx] = super::map::SLOW_RAM_HANDLER;
                if let Some(slow_ram) = &mut self.slow_ram {
                    let offset = (addr - 0xC00000) as usize;
                    if offset < slow_ram.len() {
                        slow_ram[offset] = val;
                    }
                }
            } else if (0x200000..=0x9FFFFF).contains(&addr) {
                if self.fast_ram.is_none() {
                    self.fast_ram = Some(vec![0x00; 2 * 1024 * 1024]);
                }
                let bank_idx = (addr >> 16) as usize;
                self.bank_map[bank_idx] = super::map::FAST_RAM_HANDLER;
                if let Some(fast_ram) = &mut self.fast_ram {
                    let offset = (addr - 0x200000) as usize;
                    if offset < fast_ram.len() {
                        fast_ram[offset] = val;
                    }
                }
            } else {
                // If test writes to higher address, expand Chip RAM buffer for testing if within 16MB
                if (addr as usize) >= self.chip_ram.len() && (addr as usize) < 0x200000 {
                    self.chip_ram.resize((addr as usize) + 1, 0x00);
                    let bank_idx = (addr >> 16) as usize;
                    self.bank_map[bank_idx] = super::map::CHIP_RAM_HANDLER;
                    self.chip_ram[addr as usize] = val;
                }
            }
        }
    }

    /// Injects Kickstart ROM image bytes
    pub fn inject_kickstart_rom(&mut self, rom: &[u8]) {
        self.kickstart_rom = rom.to_vec();
    }

    /// Side-effect-free byte read for debugger inspection and test result assertions
    #[inline]
    pub fn read_byte_debug(&self, addr: u32) -> u8 {
        self.read_byte_internal(addr)
    }

    /// Side-effect-free word read for disassemblers, debugger inspection, and test result assertions
    #[inline]
    pub fn read_word_debug(&self, addr: u32) -> u16 {
        self.read_word_internal(addr)
    }

    /// Side-effect-free byte write for debugger modification
    #[inline]
    pub fn write_byte_debug(&mut self, addr: u32, val: u8) {
        self.write_byte_internal(addr, val);
    }

    /// Side-effect-free word write for debugger modification
    #[inline]
    pub fn write_word_debug(&mut self, addr: u32, val: u16) {
        self.write_word_internal(addr, val);
    }
}
