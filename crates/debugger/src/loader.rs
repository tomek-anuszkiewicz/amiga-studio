//! Headless Binary Program Loader & Memory Injection
//!
//! Handles injecting raw M68000 binary machine code into physical memory
//! and setting the Program Counter with prefetch priming.

use cpu::Cpu;
use physical_memory::PhysicalMemory;

/// Default target RAM address for synthetic binaries ($001000)
pub const DEFAULT_TARGET_ADDRESS: u32 = 0x001000;

/// Injects a byte slice into physical memory and optionally sets PC & primes prefetch
pub fn inject_binary(
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
    target_addr: u32,
    data: &[u8],
    auto_prime: bool,
) -> Result<usize, physical_memory::KickstartSizeError> {
    // Disengage Kickstart low-memory boot overlay so physical Chip RAM is accessible
    let overlay_was_active = bus.is_low_memory_overlay_active();
    bus.map_chip_ram_to_low_memory();

    let written = match bus.write_bytes_debug(target_addr, data) {
        Ok(written) => written,
        Err(error) => {
            if overlay_was_active {
                bus.map_kickstart_to_low_memory();
            }
            return Err(error);
        }
    };

    if auto_prime {
        // If stack pointer is zero, set default SP to top of 512KB Chip RAM ($080000)
        if cpu.state.a_long(cpu::state::REGISTER_STACK_POINTER) == 0 {
            cpu.state.set_a_long(
                cpu::state::REGISTER_STACK_POINTER,
                physical_memory::DEFAULT_BOOT_SSP,
            );
            cpu.state.set_ssp(physical_memory::DEFAULT_BOOT_SSP);
        }
        cpu.set_pc_and_prime_prefetch(target_addr, bus);
    }

    Ok(written)
}
