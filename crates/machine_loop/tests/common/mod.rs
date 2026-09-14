//! Machine Loop Test Harness Helper
//!
//! Provides a lightweight, ergonomic harness for constructing synthetic
//! whole-machine integration tests spanning CPU, Agnus, Denise, Paula, and CIAs.

use machine_loop::A500Machine;

/// Fluent test harness wrapping an initialized `A500Machine` in low-memory Chip RAM mode
#[allow(dead_code)]
pub struct MachineHarness {
    pub machine: A500Machine,
}

#[allow(dead_code)]
impl Default for MachineHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl MachineHarness {
    /// Creates a new machine test harness with Chip RAM engaged at $000000 and supervisor mode active
    pub fn new() -> Self {
        Self::new_with_preset(config::A500Preset::Standard1Mb)
    }

    /// Creates a new machine test harness with a specific hardware configuration preset
    pub fn new_with_preset(preset: config::A500Preset) -> Self {
        let config = match preset {
            config::A500Preset::Bare512k => {
                config::A500Config::bare_512k(config::VideoStandard::Pal)
            }
            config::A500Preset::Standard1Mb => {
                config::A500Config::standard_1mb(config::VideoStandard::Pal)
            }
            config::A500Preset::ExpandedPowerUser => {
                config::A500Config::expanded_power_user(config::VideoStandard::Pal)
            }
        };
        let mut machine = A500Machine::new(config);
        machine.physical_memory.map_chip_ram_to_low_memory();

        // Supervisor stack pointer at top of 512KB Chip RAM ($070000)
        machine.cpu.state.ssp = 0x070000;
        machine.cpu.state.write_a(7, 0x070000);
        machine.cpu.state.sr = 0x2000; // Supervisor mode, Interrupt Mask = 0

        Self { machine }
    }

    /// Loads M68000 machine code words into memory and primes the CPU prefetch queue
    pub fn load_cpu_code(&mut self, start_addr: u32, words: &[u16]) -> &mut Self {
        for (i, &w) in words.iter().enumerate() {
            self.machine
                .physical_memory
                .write_word_debug(start_addr + (i as u32) * 2, w);
        }
        self.machine.set_pc_and_prime_prefetch(start_addr);
        self
    }

    /// Loads a Copper instruction list into Chip RAM and sets COP1LC pointer
    pub fn load_copper_list(&mut self, start_addr: u32, words: &[u16]) -> &mut Self {
        for (i, &w) in words.iter().enumerate() {
            self.machine
                .physical_memory
                .write_word_debug(start_addr + (i as u32) * 2, w);
        }
        self.machine.agnus.copper.set_cop1lc(start_addr);
        self
    }

    /// Sets an exception vector in the low-RAM vector table (0..255)
    pub fn set_vector(&mut self, vector_num: u32, handler_addr: u32) -> &mut Self {
        let vector_addr = vector_num * 4;
        self.machine
            .physical_memory
            .write_word_debug(vector_addr, (handler_addr >> 16) as u16);
        self.machine
            .physical_memory
            .write_word_debug(vector_addr + 2, (handler_addr & 0xFFFF) as u16);
        self
    }

    /// Advances the entire machine by `count` Color Clocks
    pub fn step_cck(&mut self, count: u64) -> &mut Self {
        for _ in 0..count {
            self.machine.step_cck();
        }
        self
    }

    /// Advances the machine across `lines` full raster scanlines
    pub fn step_scanlines(&mut self, lines: u32) -> &mut Self {
        for _ in 0..lines {
            let start_vpos = self.machine.agnus.vpos;
            while self.machine.agnus.vpos == start_vpos {
                self.machine.step_cck();
            }
        }
        self
    }

    /// Steps the machine until the vertical beam reaches `target_vpos` or `max_cck` elapsed
    pub fn step_until_vpos(&mut self, target_vpos: u16, max_cck: u64) -> bool {
        for _ in 0..max_cck {
            if self.machine.agnus.vpos == target_vpos {
                return true;
            }
            self.machine.step_cck();
        }
        false
    }

    /// Steps the machine until CPU reaches `target_pc` or `max_cck` elapsed
    pub fn step_until_pc(&mut self, target_pc: u32, max_cck: u64) -> bool {
        for _ in 0..max_cck {
            if self.machine.cpu.state.instruction_pc == target_pc {
                return true;
            }
            self.machine.step_cck();
        }
        false
    }
}
