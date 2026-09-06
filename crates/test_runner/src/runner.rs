//! Test runner for M68000 SingleStepTests

use crate::schema::SingleStepTest;
use flate2::read::GzDecoder;
use m68000::Cpu;
use memory_bus::MemoryBus;
use std::fs::File;
use std::io::{BufReader, Read};

/// Runs a single SingleStepTest against the CPU and MemoryBus
pub fn run_single_test(test: &SingleStepTest) -> Result<(), String> {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.load_test_ram(&test.initial.ram);

    let mut cpu = Cpu::new();
    cpu.state.d = [
        test.initial.d0,
        test.initial.d1,
        test.initial.d2,
        test.initial.d3,
        test.initial.d4,
        test.initial.d5,
        test.initial.d6,
        test.initial.d7,
    ];
    cpu.state.a = [
        test.initial.a0,
        test.initial.a1,
        test.initial.a2,
        test.initial.a3,
        test.initial.a4,
        test.initial.a5,
        test.initial.a6,
    ];
    cpu.state.usp = test.initial.usp;
    cpu.state.ssp = test.initial.ssp;
    cpu.state.sr = test.initial.sr;
    cpu.state.pc = test.initial.pc;
    cpu.state.ir = (test.initial.prefetch[0] & 0xFFFF) as u16;
    cpu.state.prefetch[0] = (test.initial.prefetch[1] & 0xFFFF) as u16;
    cpu.state.prefetch[1] = 0;

    // Execute instruction
    let _ = cpu.step_instruction(&mut bus);

    // Verify Data Registers
    let expected_d = [
        test.final_state.d0,
        test.final_state.d1,
        test.final_state.d2,
        test.final_state.d3,
        test.final_state.d4,
        test.final_state.d5,
        test.final_state.d6,
        test.final_state.d7,
    ];
    for i in 0..8 {
        if cpu.state.d[i] != expected_d[i] {
            return Err(format!(
                "D{} mismatch in '{}': got {:08X}, expected {:08X}",
                i, test.name, cpu.state.d[i], expected_d[i]
            ));
        }
    }

    // Verify Address Registers
    let expected_a = [
        test.final_state.a0,
        test.final_state.a1,
        test.final_state.a2,
        test.final_state.a3,
        test.final_state.a4,
        test.final_state.a5,
        test.final_state.a6,
    ];
    for i in 0..7 {
        if cpu.state.a[i] != expected_a[i] {
            return Err(format!(
                "A{} mismatch in '{}': got {:08X}, expected {:08X}",
                i, test.name, cpu.state.a[i], expected_a[i]
            ));
        }
    }

    // Verify Stack Pointers
    if cpu.state.usp != test.final_state.usp {
        return Err(format!(
            "USP mismatch in '{}': got {:08X}, expected {:08X}",
            test.name, cpu.state.usp, test.final_state.usp
        ));
    }
    if cpu.state.ssp != test.final_state.ssp {
        return Err(format!(
            "SSP mismatch in '{}': got {:08X}, expected {:08X}",
            test.name, cpu.state.ssp, test.final_state.ssp
        ));
    }

    // Verify Status Register (CCR flags)
    if cpu.state.sr != test.final_state.sr {
        return Err(format!(
            "SR mismatch in '{}': got {:04X}, expected {:04X}",
            test.name, cpu.state.sr, test.final_state.sr
        ));
    }

    // Verify Program Counter
    if (cpu.state.pc & 0x00FF_FFFF) != (test.final_state.pc & 0x00FF_FFFF) {
        return Err(format!(
            "PC mismatch in '{}': got {:08X}, expected {:08X}",
            test.name, cpu.state.pc, test.final_state.pc
        ));
    }

    // Verify RAM modifications
    for entry in &test.final_state.ram {
        let addr = entry[0] & 0x00FF_FFFF;
        let expected_byte = (entry[1] & 0xFF) as u8;
        let actual_byte = bus.read_byte_debug(addr);
        if actual_byte != expected_byte {
            return Err(format!(
                "RAM byte mismatch at ${:06X} in '{}': got {:02X}, expected {:02X}",
                addr, test.name, actual_byte, expected_byte
            ));
        }
    }

    Ok(())
}

/// Loads and executes tests from a plain JSON or gzip (.json.gz) test suite file
pub fn run_test_file(
    path: &str,
    limit: Option<usize>,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let tests: Vec<SingleStepTest> = if path.ends_with(".gz") {
        let mut gz = GzDecoder::new(reader);
        let mut json_str = String::new();
        gz.read_to_string(&mut json_str)?;
        serde_json::from_str(&json_str)?
    } else {
        serde_json::from_reader(reader)?
    };

    let count = match limit {
        Some(max) => tests.len().min(max),
        None => tests.len(),
    };

    let mut passed = 0;
    let mut failed = 0;

    for test in tests.iter().take(count) {
        match run_single_test(test) {
            Ok(()) => passed += 1,
            Err(err) => {
                failed += 1;
                eprintln!("Test failure: {}", err);
            }
        }
    }

    Ok((passed, failed))
}
