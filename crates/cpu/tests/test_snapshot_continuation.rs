#![allow(clippy::unwrap_used)]

use cpu::Cpu;
use physical_memory::PhysicalMemory;

// Continuation is a relative invariant: restoring at T must preserve every future
// CPU value and memory write, independently of absolute instruction timing.
#[test]
fn redirected_branch_survives_every_cck_snapshot() {
    let mut memory = PhysicalMemory::new();
    memory.map_chip_ram_to_low_memory();
    memory.write_bytes_debug(0x1000, &[0x66, 0xfe]).unwrap();
    let mut cpu = Cpu::new();
    cpu.state.set_ssp(0x70000);
    cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);
    for boundary in 0..12 {
        let saved = serde_json::to_string(&cpu.state).unwrap();
        let mut restored = Cpu::new();
        restored.restore_state(serde_json::from_str(&saved).unwrap());
        let mut expected = cpu.clone();
        let mut expected_memory = memory.clone();
        let mut restored_memory = memory.clone();
        for offset in 0..12 {
            assert_eq!(
                expected.step_cck(&mut expected_memory),
                restored.step_cck(&mut restored_memory),
                "boundary {boundary}, offset {offset}"
            );
            assert_eq!(
                expected.state, restored.state,
                "boundary {boundary}, offset {offset}"
            );
            assert_eq!(expected_memory, restored_memory);
        }
        cpu.step_cck(&mut memory);
    }
}

#[test]
fn branches_exceptions_early_prefetch_and_partial_transfers_continue_exactly() {
    // Each case crosses dispatch, internal redirection, bus phases and retirement.
    for (program, d0, sr) in [
        (vec![0x66fe], 0, 0x2704),         // short branch not taken
        (vec![0x50c8, 0xfffc], 0, 0x2700), // DBT condition true
        (vec![0x6600, 0xfffc], 0, 0x2700),
        (vec![0x6600, 0xfffc], 0, 0x2704),
        (vec![0x51c8, 0xfffc], 0, 0x2700),
        (vec![0x51c8, 0xfffc], 2, 0x2700),
        (vec![0x50c0], 0, 0x2700),
        (vec![0x51c0], 0, 0x2700),
        (vec![0x80fc, 0], 10, 0x2700),              // DIVU zero
        (vec![0x4181], 0xffff, 0x2700),             // CHK negative
        (vec![0x4181], 10, 0x2700),                 // CHK upper bound (D1=1)
        (vec![0x4e76], 0, 0x2702),                  // TRAPV
        (vec![0x4e72, 0x2700], 0, 0),               // privilege exception
        (vec![0x3080], 0x1234, 0x2700),             // MOVE.W D0,(A0), early prefetch
        (vec![0x48d0, 0x0003], 0x12345678, 0x2700), // MOVEM
        (vec![0x3010], 0, 0x2700),                  // odd operand -> Address Error
        (vec![0x4e71], 0, 0xa700),                  // trace
    ] {
        let mut bus = TracedBus::new();
        let bytes: Vec<_> = program
            .iter()
            .flat_map(|word: &u16| word.to_be_bytes())
            .collect();
        bus.memory.write_bytes_debug(0x1000, &bytes).unwrap();
        // Every exception vector leads to a stable branch loop in RAM.
        for vector in 2..48 {
            bus.memory
                .write_bytes_debug(vector * 4, &0x2000u32.to_be_bytes())
                .unwrap();
        }
        bus.memory.write_bytes_debug(0x2000, &[0x60, 0xfe]).unwrap();
        let mut cpu = Cpu::new();
        cpu.state.set_ssp(0x70000);
        cpu.state.set_usp(0x68000);
        cpu.state.set_d_long(0, d0);
        cpu.state.set_d_long(1, 1);
        cpu.state
            .set_a_long(0, if program == [0x3010] { 0x3001 } else { 0x3000 });
        cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
        cpu.state.set_sr(sr);
        for boundary in 0..36 {
            let saved = serde_json::to_string(&cpu.state).unwrap();
            let mut restored = Cpu::new();
            restored.restore_state(serde_json::from_str(&saved).unwrap());
            let mut reference = cpu.clone();
            let mut reference_bus = bus.clone();
            let mut restored_bus = bus.clone();
            reference_bus.trace.clear();
            restored_bus.trace.clear();
            for offset in 0..28 {
                assert_eq!(
                    reference.step_cck(&mut reference_bus),
                    restored.step_cck(&mut restored_bus),
                    "program {program:x?}, boundary {boundary}, offset {offset}"
                );
                assert_eq!(
                    reference.state, restored.state,
                    "program {program:x?}, boundary {boundary}, offset {offset}"
                );
                assert_eq!(
                    reference_bus.trace, restored_bus.trace,
                    "timed transfers diverged"
                );
            }
            assert_eq!(reference_bus.memory, restored_bus.memory);
            cpu.step_cck(&mut bus);
        }
    }
}

#[derive(Clone)]
struct TracedBus {
    memory: PhysicalMemory,
    cck: u64,
    trace: Vec<(u64, u32, bool, u16, bool)>,
}

#[test]
fn accepted_interrupt_entry_restores_with_waits_and_exact_future_transfers() {
    let mut bus = TracedBus::new();
    bus.memory
        .write_bytes_debug(0x1000, &[0x4e, 0x71, 0x60, 0xfe])
        .unwrap();
    bus.memory
        .write_bytes_debug(28 * 4, &0x2000u32.to_be_bytes())
        .unwrap();
    bus.memory.write_bytes_debug(0x2000, &[0x60, 0xfe]).unwrap();
    let mut cpu = Cpu::new();
    cpu.state.set_ssp(0x70000);
    cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
    cpu.state.set_sr(0x2000);
    cpu.state.set_ipl(4);
    assert!(cpu.check_and_trigger_interrupt());
    assert!(serde_json::to_string(&cpu.state)
        .unwrap()
        .contains("\"Interrupt\""));
    for boundary in 0..48 {
        let encoded = serde_json::to_string(&cpu.state).unwrap();
        let mut restored = Cpu::new();
        restored.restore_state(serde_json::from_str(&encoded).unwrap());
        let mut reference = cpu.clone();
        let mut reference_bus = bus.clone();
        let mut restored_bus = bus.clone();
        reference_bus.trace.clear();
        restored_bus.trace.clear();
        for offset in 0..28 {
            assert_eq!(
                reference.step_cck(&mut reference_bus),
                restored.step_cck(&mut restored_bus)
            );
            assert_eq!(
                reference.state, restored.state,
                "IRQ boundary {boundary}, offset {offset}"
            );
            assert_eq!(reference_bus.trace, restored_bus.trace);
        }
        assert_eq!(reference_bus.memory, restored_bus.memory);
        cpu.step_cck(&mut bus);
    }
}
impl TracedBus {
    fn new() -> Self {
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        Self {
            memory,
            cck: 0,
            trace: Vec::new(),
        }
    }
    fn wait(&self) -> bool {
        self.cck % 7 == 0 || self.cck % 7 == 1
    }
}
impl physical_memory::AddressBus for TracedBus {
    fn begin_cpu_cck(&mut self, cck: u64) -> u64 {
        self.cck = cck;
        cck
    }
    fn read_byte(&mut self, addr: u32) -> physical_memory::BusResult<u8> {
        let value = self.memory.read_byte_debug(addr);
        let wait = self.wait();
        self.trace
            .push((self.cck, addr, false, u16::from(value), wait));
        if wait {
            physical_memory::BusResult::WaitState
        } else {
            physical_memory::BusResult::Ready(value)
        }
    }
    fn read_word(&mut self, addr: u32) -> physical_memory::BusResult<u16> {
        let value = self.memory.read_word_debug(addr);
        let wait = self.wait();
        self.trace.push((self.cck, addr, false, value, wait));
        if wait {
            physical_memory::BusResult::WaitState
        } else {
            physical_memory::BusResult::Ready(value)
        }
    }
    fn write_byte(&mut self, addr: u32, value: u8) -> physical_memory::BusResult<()> {
        let wait = self.wait();
        self.trace
            .push((self.cck, addr, true, u16::from(value), wait));
        if wait {
            physical_memory::BusResult::WaitState
        } else {
            self.memory.write_byte_debug(addr, value);
            physical_memory::BusResult::Ready(())
        }
    }
    fn write_word(&mut self, addr: u32, value: u16) -> physical_memory::BusResult<()> {
        let wait = self.wait();
        self.trace.push((self.cck, addr, true, value, wait));
        if wait {
            physical_memory::BusResult::WaitState
        } else {
            self.memory.write_word_debug(addr, value);
            physical_memory::BusResult::Ready(())
        }
    }
    fn read_word_debug(&self, addr: u32) -> u16 {
        self.memory.read_word_debug(addr)
    }
}
