//! Headless Binary Program Loader & Memory Injection
//!
//! Handles injecting raw M68000 binary machine code into physical memory
//! and setting the Program Counter with prefetch priming.

use m68000::Cpu;
use memory_bus::MemoryBus;

/// Default target RAM address for synthetic binaries ($001000)
pub const DEFAULT_TARGET_ADDRESS: u32 = 0x001000;

/// Injects a byte slice into physical memory and optionally sets PC & primes prefetch
pub fn inject_binary(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    target_addr: u32,
    data: &[u8],
    auto_prime: bool,
) -> usize {
    // Disengage Kickstart low-memory boot overlay so physical Chip RAM is accessible
    bus.map_chip_ram_to_low_memory();

    let mut written = 0;
    for (i, &byte) in data.iter().enumerate() {
        let addr = target_addr.wrapping_add(i as u32);
        bus.write_byte_debug(addr, byte);
        written += 1;
    }

    if auto_prime {
        // If stack pointer is zero, set default SP to top of 512KB Chip RAM ($080000)
        if cpu.state.a_regs()[7] == 0 {
            cpu.state.set_a_long(7, 0x080000);
            cpu.state.ssp = 0x080000;
        }
        cpu.set_pc_and_prime_prefetch(target_addr, bus);
    }

    written
}
