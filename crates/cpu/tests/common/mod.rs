//! Scripted refusals and transfer observations for ordinary CPU tests.
#![allow(dead_code)]

use cpu::{Cpu, CpuState};
use physical_memory::{AddressBus, BusResult, PhysicalMemory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    ReadByte(u32),
    ReadWord(u32),
    WriteByte(u32, u8),
    WriteWord(u32, u16),
}

#[derive(Debug)]
pub struct Attempt {
    pub access: Access,
    pub ready: bool,
}

pub struct RecordingBus {
    pub memory: PhysicalMemory,
    pub attempts: Vec<Attempt>,
    pub refusals: u64,
    script: Vec<(Access, usize)>,
    completed: usize,
    remaining: usize,
}

impl RecordingBus {
    pub fn new(memory: PhysicalMemory, script: Vec<(Access, usize)>) -> Self {
        let remaining = script.first().map_or(0, |entry| entry.1);
        Self {
            memory,
            attempts: Vec::new(),
            refusals: 0,
            script,
            completed: 0,
            remaining,
        }
    }

    fn accept(&mut self, access: Access) -> bool {
        let expected = self
            .script
            .get(self.completed)
            .expect("unexpected bus access");
        assert_eq!(access, expected.0, "transfer identity/order changed");
        let ready = self.remaining == 0;
        self.attempts.push(Attempt { access, ready });
        if ready {
            self.completed += 1;
            self.remaining = self.script.get(self.completed).map_or(0, |entry| entry.1);
        } else {
            self.remaining -= 1;
            self.refusals += 1;
        }
        ready
    }

    pub fn assert_complete(&self) {
        assert_eq!(
            self.completed,
            self.script.len(),
            "unconsumed transfer script"
        );
        assert_eq!(
            self.refusals,
            self.script.iter().map(|entry| entry.1 as u64).sum()
        );
        assert_eq!(
            self.attempts.iter().filter(|attempt| attempt.ready).count(),
            self.script.len()
        );
    }
}

impl AddressBus for RecordingBus {
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        if self.accept(Access::ReadByte(addr)) {
            self.memory.read_byte(addr)
        } else {
            BusResult::WaitState
        }
    }

    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        if self.accept(Access::ReadWord(addr)) {
            self.memory.read_word(addr)
        } else {
            BusResult::WaitState
        }
    }

    fn write_byte(&mut self, addr: u32, data: u8) -> BusResult<()> {
        if self.accept(Access::WriteByte(addr, data)) {
            self.memory.write_byte(addr, data)
        } else {
            BusResult::WaitState
        }
    }

    fn write_word(&mut self, addr: u32, data: u16) -> BusResult<()> {
        if self.accept(Access::WriteWord(addr, data)) {
            self.memory.write_word(addr, data)
        } else {
            BusResult::WaitState
        }
    }

    fn read_word_debug(&self, addr: u32) -> u16 {
        self.memory.read_word_debug(addr)
    }
}

pub fn without_clocks(state: &CpuState) -> CpuState {
    let mut snapshot = state.clone();
    snapshot.set_cycle_counter(0);
    snapshot
}

/// Expected transfers/times come from each fixture, never from a discovery run.
pub fn run_instruction(cpu: &mut Cpu, bus: &mut RecordingBus, base_clocks: u64) {
    run_next_instruction(cpu, bus, base_clocks);
    bus.assert_complete();
}

/// Check one instruction inside a longer, independently specified bus script.
pub fn run_next_instruction(cpu: &mut Cpu, bus: &mut RecordingBus, base_clocks: u64) {
    let start = cpu.cycle_counter();
    let start_refusals = bus.refusals;
    let mut waiting: Option<(usize, CpuState)> = None;
    for _ in 0..2048 {
        let before = cpu.cycle_counter();
        let state_before = without_clocks(&cpu.state);
        let active_step = state_before
            .micro()
            .current_steps
            .get(state_before.micro().micro_step as usize)
            .copied();
        let refusals = bus.refusals;
        let finished = cpu.step_cck(bus);
        assert_eq!(cpu.cycle_counter(), before + 2);
        if bus.refusals != refusals {
            assert!(!finished, "instruction retired during refusal");
            let snapshot = without_clocks(&cpu.state);
            if let Some(step) = active_step {
                if step.alu_fn.is_none() && step.bus_fn.is_some() {
                    let mut expected = state_before;
                    if expected.micro().clocks_remaining == 0 {
                        expected.micro_mut().clocks_remaining = step.base_clocks as u16;
                    }
                    // MOVEM can select its pending dummy read on the first refusal.
                    // Register/operand/address state must still remain unchanged.
                    let dummy_read_pending = (16 << 1) | (1 << 7);
                    if step.base_clocks == 0 && snapshot.micro().movem_state == dummy_read_pending {
                        expected.micro_mut().movem_state = dummy_read_pending;
                    }
                    assert_eq!(snapshot, expected, "refused transfer changed active state");
                }
            }
            if let Some((position, previous)) = &waiting {
                if *position == bus.completed {
                    assert!(std::ptr::eq(
                        snapshot.micro().current_steps,
                        previous.micro().current_steps
                    ));
                    assert_eq!(&snapshot, previous, "repeated wait changed active state");
                }
            }
            waiting = Some((bus.completed, snapshot));
        } else {
            waiting = None;
        }
        if finished {
            assert_eq!(
                cpu.cycle_counter() - start,
                base_clocks + 2 * (bus.refusals - start_refusals)
            );
            return;
        }
        assert!(!cpu.state.is_halted() && !cpu.state.is_stopped());
    }
    panic!("instruction did not complete within the fixture limit");
}

pub fn retire_to(cpu: &mut Cpu, bus: &mut dyn AddressBus, next_pc: u32) {
    for _ in 0..2048 {
        if cpu.step_cck(bus) {
            assert_eq!(cpu.state.instruction_pc(), next_pc);
            return;
        }
        assert!(!cpu.state.is_halted(), "unexpected halt");
    }
    panic!("instruction did not retire within 2048 CCKs");
}
