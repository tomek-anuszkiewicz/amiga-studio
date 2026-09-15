//! vAmigaTS Direct-Injection Payload Extractor & OS Stub Initializer
//!
//! Slices executable test payloads directly from Sector 2 ($000400) of test ADF images
//! into Chip RAM at $00070000 and installs lightweight stub ExecBase and GfxBase vector tables,
//! bypassing floppy motor emulation, MFM track decoding, and Kickstart bootstrap.

use machine_loop::A500Machine;

/// Standard entry point for vAmigaTS test payloads in Chip RAM
pub const VAMIGA_ENTRY_POINT: u32 = 0x0007_0000;

/// Standard Supervisor Stack Pointer for vAmigaTS execution
pub const VAMIGA_STACK_POINTER: u32 = 0x0007_FF00;

/// Byte offset of the compiled machine code payload in standard vAmigaTS ADF images (Sector 2)
pub const ADF_PAYLOAD_OFFSET: usize = 0x000400;

/// Maximum payload size extracted from ADF (up to ~62 KB)
pub const MAX_PAYLOAD_SIZE: usize = 0x00F800;

/// Address of synthetic ExecBase structure in low memory
pub const STUB_EXEC_BASE: u32 = 0x0000_1000;

/// Address of synthetic GfxBase structure in low memory
pub const STUB_GFX_BASE: u32 = 0x0000_2000;

/// Writes a 16-bit big-endian word to Chip RAM at `addr`
#[inline(always)]
fn write_be_u16(ram: &mut [u8], addr: usize, val: u16) {
    if addr + 1 < ram.len() {
        ram[addr] = (val >> 8) as u8;
        ram[addr + 1] = (val & 0xFF) as u8;
    }
}

/// Writes a 32-bit big-endian longword to Chip RAM at `addr`
#[inline(always)]
fn write_be_u32(ram: &mut [u8], addr: usize, val: u32) {
    if addr + 3 < ram.len() {
        ram[addr] = (val >> 24) as u8;
        ram[addr + 1] = ((val >> 16) & 0xFF) as u8;
        ram[addr + 2] = ((val >> 8) & 0xFF) as u8;
        ram[addr + 3] = (val & 0xFF) as u8;
    }
}

/// Configures zero-allocation stub vector tables for ExecBase and GfxBase
/// enabling `ministartup.s` to resolve libraries and proceed to MAIN without Kickstart ROM.
pub fn setup_stub_os_vectors(ram: &mut [u8]) {
    // 1. ExecBase OpenLibrary at -552 (-0x228) -> returns GfxBase (STUB_GFX_BASE)
    // Code: move.l #STUB_GFX_BASE, d0 (0x203C, 0x0000, 0x2000) ; rts (0x4E75)
    let open_lib_addr = (STUB_EXEC_BASE as usize).wrapping_sub(552);
    write_be_u16(ram, open_lib_addr, 0x203C);
    write_be_u32(ram, open_lib_addr + 2, STUB_GFX_BASE);
    write_be_u16(ram, open_lib_addr + 6, 0x4E75);

    // 2. ExecBase CloseLibrary at -414 (-0x19E) -> returns 0 in D0
    // Code: moveq #0, d0 (0x7000) ; rts (0x4E75)
    let close_lib_addr = (STUB_EXEC_BASE as usize).wrapping_sub(414);
    write_be_u16(ram, close_lib_addr, 0x7000);
    write_be_u16(ram, close_lib_addr + 2, 0x4E75);

    // 3. ExecBase SuperVisor at -30 (-0x1E) -> executes (a5)
    // Code: jsr (a5) (0x4E95) ; rts (0x4E75)
    let supervisor_addr = (STUB_EXEC_BASE as usize).wrapping_sub(30);
    write_be_u16(ram, supervisor_addr, 0x4E95);
    write_be_u16(ram, supervisor_addr + 2, 0x4E75);

    // 4. ExecBase AttnFlags at +296 (0x128) -> 0 (Standard 68000 CPU)
    write_be_u16(ram, STUB_EXEC_BASE as usize + 296, 0x0000);

    // 5. GfxBase LoadView at -222 (-0xDE) -> rts (0x4E75)
    let load_view_addr = (STUB_GFX_BASE as usize).wrapping_sub(222);
    write_be_u16(ram, load_view_addr, 0x4E75);

    // 6. GfxBase WaitTOF at -270 (-0x10E) -> rts (0x4E75)
    let wait_tof_addr = (STUB_GFX_BASE as usize).wrapping_sub(270);
    write_be_u16(ram, wait_tof_addr, 0x4E75);

    // 7. GfxBase View and Copper fields (34 -> OldView, $26 -> OldCop1, $32 -> OldCop2)
    write_be_u32(ram, STUB_GFX_BASE as usize + 34, 0);
    write_be_u32(ram, STUB_GFX_BASE as usize + 0x26, 0);
    write_be_u32(ram, STUB_GFX_BASE as usize + 0x32, 0);
}

/// Injects a vAmigaTS test ADF directly into the machine's Chip RAM and initializes execution state.
pub fn inject_vamiga_test(machine: &mut A500Machine, adf_bytes: &[u8]) -> Result<(), String> {
    if adf_bytes.len() < ADF_PAYLOAD_OFFSET + 512 {
        return Err("ADF file too short to contain Sector 2 payload".to_string());
    }

    // 1. Cold reset and map Chip RAM to low memory
    machine.reset_cold();
    machine.physical_memory.map_chip_ram_to_low_memory();

    // 2. Slice Sector 2 payload ($000400) directly into Chip RAM at $00070000
    let payload_end = (ADF_PAYLOAD_OFFSET + MAX_PAYLOAD_SIZE).min(adf_bytes.len());
    let payload = &adf_bytes[ADF_PAYLOAD_OFFSET..payload_end];

    let dest_start = VAMIGA_ENTRY_POINT as usize;
    let dest_end = dest_start + payload.len();
    if dest_end > machine.physical_memory.chip_ram.len() {
        return Err("Chip RAM too small for test payload".to_string());
    }
    machine.physical_memory.chip_ram[dest_start..dest_end].copy_from_slice(payload);

    // 3. Set Reset Vector 0 ($0, SSP) and Vector 1 ($4, ExecBase)
    write_be_u32(
        &mut machine.physical_memory.chip_ram,
        0,
        VAMIGA_STACK_POINTER,
    );
    write_be_u32(&mut machine.physical_memory.chip_ram, 4, STUB_EXEC_BASE);

    // 4. Install synthetic ExecBase & GfxBase vector tables
    setup_stub_os_vectors(&mut machine.physical_memory.chip_ram);

    // 5. Initialize CPU register state and prime prefetch pipeline
    machine.cpu.state.clear_registers();
    machine.cpu.state.ssp = VAMIGA_STACK_POINTER;
    machine.cpu.state.set_a_long(7, VAMIGA_STACK_POINTER);
    machine.cpu.state.sr = 0x2000; // Supervisor mode, IPL 0 (interrupts enabled)
                                   // Emulate Kickstart OS state: Master Interrupts (INTEN) enabled
    machine.paula.intena = 0x4000;
    machine.set_pc_and_prime_prefetch(VAMIGA_ENTRY_POINT);

    Ok(())
}
