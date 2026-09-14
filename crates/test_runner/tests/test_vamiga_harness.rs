//! vAmigaTS Test Harness Unit & Integration Tests
//!
//! Validates ADF Sector 2 direct-injection payload extraction,
//! stub ExecBase/GfxBase OS vectors, FrameBuilder viewport extraction,
//! and the golden raw reference differencer.

use config::A500Config;
use machine_loop::A500Machine;
use test_runner::{
    compare_raw_frames, inject_vamiga_test, setup_stub_os_vectors, ADF_PAYLOAD_OFFSET,
    STUB_EXEC_BASE, STUB_GFX_BASE, VAMIGA_ENTRY_POINT, VAMIGA_RAW_BYTE_SIZE, VAMIGA_RAW_HEIGHT,
    VAMIGA_RAW_PIXELS, VAMIGA_RAW_WIDTH, VAMIGA_STACK_POINTER,
};

#[test]
fn test_vamiga_raw_geometry_constants() {
    assert_eq!(VAMIGA_RAW_WIDTH, 716);
    assert_eq!(VAMIGA_RAW_HEIGHT, 285);
    assert_eq!(VAMIGA_RAW_PIXELS, 204_060);
    assert_eq!(VAMIGA_RAW_BYTE_SIZE, 612_180);
    assert_eq!(VAMIGA_RAW_PIXELS * 3, VAMIGA_RAW_BYTE_SIZE);
}

#[test]
fn test_stub_os_vectors_setup() {
    let mut ram = vec![0u8; 0x10000];
    setup_stub_os_vectors(&mut ram);

    // 1. Check ExecBase OpenLibrary at -552 (-0x228) -> $1000 - $228 = $0DD8
    let open_lib = (STUB_EXEC_BASE as usize) - 552;
    // Opcode for `move.l #$2000, d0` is 0x203C 0x0000 0x2000, followed by `rts` (0x4E75)
    assert_eq!(ram[open_lib], 0x20);
    assert_eq!(ram[open_lib + 1], 0x3C);
    assert_eq!(
        u32::from_be_bytes([
            ram[open_lib + 2],
            ram[open_lib + 3],
            ram[open_lib + 4],
            ram[open_lib + 5]
        ]),
        STUB_GFX_BASE
    );
    assert_eq!(ram[open_lib + 6], 0x4E);
    assert_eq!(ram[open_lib + 7], 0x75);

    // 2. Check GfxBase LoadView at -222 (-0xDE) -> $2000 - $DE = $1F22 -> `rts` (0x4E75)
    let load_view = (STUB_GFX_BASE as usize) - 222;
    assert_eq!(ram[load_view], 0x4E);
    assert_eq!(ram[load_view + 1], 0x75);

    // 3. Check GfxBase WaitTOF at -270 (-0x10E) -> $2000 - $10E = $1EF2 -> `rts` (0x4E75)
    let wait_tof = (STUB_GFX_BASE as usize) - 270;
    assert_eq!(ram[wait_tof], 0x4E);
    assert_eq!(ram[wait_tof + 1], 0x75);
}

#[test]
fn test_direct_injection_into_chip_ram() {
    let mut machine = A500Machine::new(A500Config::default());

    // Construct a synthetic ADF with Sector 2 test payload
    let mut synthetic_adf = vec![0u8; 901_120];
    // Put distinctive test payload at offset 0x400
    let payload = [0x48, 0xE7, 0xFF, 0xFE, 0x2C, 0x78, 0x00, 0x04, 0x4E, 0x71];
    synthetic_adf[ADF_PAYLOAD_OFFSET..ADF_PAYLOAD_OFFSET + payload.len()].copy_from_slice(&payload);

    let result = inject_vamiga_test(&mut machine, &synthetic_adf);
    assert!(result.is_ok());

    // Assert Chip RAM at $70000 has the injected payload
    let ep = VAMIGA_ENTRY_POINT as usize;
    assert_eq!(
        &machine.physical_memory.chip_ram[ep..ep + payload.len()],
        &payload
    );

    // Assert Reset vector 0 (SSP) and vector 1 (ExecBase)
    let ssp = u32::from_be_bytes([
        machine.physical_memory.chip_ram[0],
        machine.physical_memory.chip_ram[1],
        machine.physical_memory.chip_ram[2],
        machine.physical_memory.chip_ram[3],
    ]);
    assert_eq!(ssp, VAMIGA_STACK_POINTER);

    let exec_base = u32::from_be_bytes([
        machine.physical_memory.chip_ram[4],
        machine.physical_memory.chip_ram[5],
        machine.physical_memory.chip_ram[6],
        machine.physical_memory.chip_ram[7],
    ]);
    assert_eq!(exec_base, STUB_EXEC_BASE);

    // Assert CPU registers
    assert_eq!(machine.cpu.state.a_long(7), VAMIGA_STACK_POINTER);
    assert_eq!(machine.cpu.state.ssp, VAMIGA_STACK_POINTER);
    assert_eq!(machine.cpu.state.instruction_pc, VAMIGA_ENTRY_POINT);
    assert_eq!(machine.cpu.state.pc, VAMIGA_ENTRY_POINT + 4);
    assert_eq!(machine.cpu.state.sr, 0x2700);
}

#[test]
fn test_matcher_identical_buffers_pass() {
    let frame = [0x55u8; VAMIGA_RAW_BYTE_SIZE];
    let result = compare_raw_frames(&frame, &frame).expect("Comparison should succeed");
    assert!(result.passed);
    assert_eq!(result.mismatched_pixels, 0);
    assert_eq!(result.total_pixels, VAMIGA_RAW_PIXELS);
    assert!(result.first_mismatch.is_none());
}

#[test]
fn test_matcher_detects_single_pixel_difference() {
    let frame_act = [0x00u8; VAMIGA_RAW_BYTE_SIZE];
    let mut frame_exp = [0x00u8; VAMIGA_RAW_BYTE_SIZE];

    // Alter pixel at (x=10, y=20) -> index = (20 * 716 + 10) * 3
    let target_pixel = 20 * VAMIGA_RAW_WIDTH + 10;
    let byte_offset = target_pixel * 3;
    frame_exp[byte_offset] = 0xFF; // Red
    frame_exp[byte_offset + 1] = 0xAA; // Green
    frame_exp[byte_offset + 2] = 0x55; // Blue

    let result = compare_raw_frames(&frame_act, &frame_exp).expect("Comparison should succeed");
    assert!(!result.passed);
    assert_eq!(result.mismatched_pixels, 1);

    let diff = result
        .first_mismatch
        .expect("Should capture first mismatch");
    assert_eq!(diff.x, 10);
    assert_eq!(diff.y, 20);
    assert_eq!(diff.actual, [0x00, 0x00, 0x00]);
    assert_eq!(diff.expected, [0xFF, 0xAA, 0x55]);
}

#[test]
fn test_matcher_invalid_length_returns_error() {
    let frame_act = [0x00u8; VAMIGA_RAW_BYTE_SIZE];
    let short_exp = vec![0u8; 1000];
    let result = compare_raw_frames(&frame_act, &short_exp);
    assert!(result.is_err());
}

#[test]
fn test_frame_builder_viewport_extraction_coordinates() {
    let mut machine = A500Machine::new(A500Config::default());

    // Top-left pixel of vAmiga viewport: X = 196, Y = 26
    machine.denise.frame_builder.set_pixel(196, 26, 0xFF11_2233);

    // Bottom-right pixel of vAmiga viewport: X = 911, Y = 310
    machine
        .denise
        .frame_builder
        .set_pixel(911, 310, 0xFF44_5566);

    let mut raw_viewport = [0u8; VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut raw_viewport);

    // First pixel in raw_viewport (offset 0..3) corresponds to (196, 26)
    assert_eq!(raw_viewport[0], 0x11);
    assert_eq!(raw_viewport[1], 0x22);
    assert_eq!(raw_viewport[2], 0x33);

    // Last pixel in raw_viewport corresponds to (911, 310)
    let last_offset = (VAMIGA_RAW_PIXELS - 1) * 3;
    assert_eq!(raw_viewport[last_offset], 0x44);
    assert_eq!(raw_viewport[last_offset + 1], 0x55);
    assert_eq!(raw_viewport[last_offset + 2], 0x66);
}
