#![allow(dead_code)]

#[path = "../benches/bench_instructions/builder.rs"]
mod builder;
#[path = "../benches/bench_instructions/catalog.rs"]
mod catalog;
#[path = "../benches/bench_instructions/catalog_data_a.rs"]
mod catalog_data_a;
#[path = "../benches/bench_instructions/catalog_data_b.rs"]
mod catalog_data_b;
#[path = "../benches/bench_instructions/prng.rs"]
mod prng;

use builder::BenchmarkProgramBuilder;
use cpu::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn signed_divide_valid_repeats_without_overflow() {
    let spec = *catalog::find_spec_by_id("ARITH-19").expect("DIVS benchmark exists");
    let program = BenchmarkProgramBuilder::new(spec).with_unroll(700).build();
    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();
    program.inject_into(&mut cpu, &mut bus);

    for operation in 0..700 {
        assert_eq!(cpu.state.instruction_pc, program.entry_pc + 2 * operation);
        assert!(cpu.step_instruction(&mut bus) > 0);
        assert_eq!(
            cpu.state.sr() & 0x0002,
            0,
            "operation {operation} overflowed"
        );
    }
    assert_ne!(cpu.state.d_long(0) & 0xffff, 0);
    assert!(cpu.step_instruction(&mut bus) > 0);
    assert_eq!(cpu.state.instruction_pc, program.exit_pc);
}
