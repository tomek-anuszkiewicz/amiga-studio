#![allow(clippy::unwrap_used)]

#[path = "../src/bus_guard.rs"]
mod bus_guard;

use bus_guard::{BusTiming, GuardedBus};
use physical_memory::{AddressBus, BusResult};

#[derive(Default)]
struct Bus {
    calls: usize,
    wait: bool,
    fixed_cck: Option<u64>,
}

impl AddressBus for Bus {
    fn begin_cpu_cck(&mut self, cck: u64) -> u64 {
        self.fixed_cck.unwrap_or(cck)
    }
    fn read_byte(&mut self, _: u32) -> BusResult<u8> {
        self.calls += 1;
        if self.wait {
            BusResult::WaitState
        } else {
            BusResult::Ready(0x71)
        }
    }
    fn read_word(&mut self, _: u32) -> BusResult<u16> {
        self.calls += 1;
        if self.wait {
            BusResult::WaitState
        } else {
            BusResult::Ready(0x4e71)
        }
    }
    fn write_byte(&mut self, _: u32, _: u8) -> BusResult<()> {
        self.calls += 1;
        if self.wait {
            BusResult::WaitState
        } else {
            BusResult::Ready(())
        }
    }
    fn write_word(&mut self, _: u32, _: u16) -> BusResult<()> {
        self.write_byte(0, 0)
    }
    fn read_word_debug(&self, _: u32) -> u16 {
        0x4e71
    }
}

#[test]
fn duplicate_attempts_including_waits_never_dispatch_twice_on_any_route() {
    for address in [0x1000, 0x200000, 0xfc0000, 0xdff000] {
        for wait in [false, true] {
            for first_write in [false, true] {
                let mut bus = Bus {
                    wait,
                    ..Bus::default()
                };
                let timing = GuardedBus::new(&mut bus, BusTiming::default(), 1, 0x1000).finish();
                let mut guarded = GuardedBus::new(&mut bus, timing, 2, 0x1000);
                assert_eq!(guarded.read_word_debug(address), 0x4e71);
                let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if first_write {
                        guarded.write_byte(address, 1);
                    } else {
                        guarded.read_byte(address);
                    }
                    guarded.read_word(address);
                }));
                assert!(failure.is_err());
                drop(guarded);
                assert_eq!(bus.calls, 1);
            }
        }
    }
}

#[test]
fn shortened_cycles_fail_and_legal_write_read_adjacency_survives_restore() {
    for rejected_write in [false, true] {
        let mut bus = Bus::default();
        let mut first = GuardedBus::new(&mut bus, BusTiming::default(), 1, 0x1000);
        first.read_word(0x1000);
        let timing = first.finish();
        let mut second =
            GuardedBus::new(&mut bus, timing, if rejected_write { 3 } else { 2 }, 0x1000);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if rejected_write {
                second.write_word(0x2000, 1);
            } else {
                second.read_word(0x1002);
            }
        }))
        .is_err());
        drop(second);
        assert_eq!(bus.calls, 1);
    }
    let mut bus = Bus::default();
    let setup = GuardedBus::new(&mut bus, BusTiming::default(), 1, 0x1000).finish();
    let mut write = GuardedBus::new(&mut bus, setup, 2, 0x1000);
    write.write_tas_byte(0x2000, 0x80);
    let saved = serde_json::to_string(&write.finish()).unwrap();
    let timing = serde_json::from_str(&saved).unwrap();
    let mut read = GuardedBus::new(&mut bus, timing, 3, 0x1000);
    assert_eq!(read.read_word(0x1000), BusResult::Ready(0x4e71));
    let timing = read.finish();
    let mut next_word = GuardedBus::new(&mut bus, timing, 5, 0x1000);
    next_word.read_word(0x1002);
    assert_eq!(next_word.finish().next_cpu_cck(), 1);
    assert_eq!(bus.calls, 3);
}

#[test]
fn waiting_transaction_keeps_address_size_direction_and_value() {
    for changed in 0..4 {
        let mut bus = Bus {
            wait: true,
            ..Bus::default()
        };
        let setup = GuardedBus::new(&mut bus, BusTiming::default(), 1, 0);
        setup.require_instruction_completion();
        let setup = setup.finish();
        let mut write = GuardedBus::new(&mut bus, setup, 2, 0);
        assert_eq!(write.write_word(0x1000, 0x1234), BusResult::WaitState);
        let saved = serde_json::to_string(&write.finish()).unwrap();
        let mut retry = GuardedBus::new(&mut bus, serde_json::from_str(&saved).unwrap(), 3, 0);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match changed {
                0 => {
                    retry.write_word(0x1002, 0x1234);
                }
                1 => {
                    retry.write_word(0x1000, 0x5678);
                }
                2 => {
                    retry.write_byte(0x1000, 0x34);
                }
                _ => {
                    retry.read_word(0x1000);
                }
            }
        }))
        .is_err());
        drop(retry);
        assert_eq!(bus.calls, 1);
    }
}

#[test]
fn normalized_cycle_starts_preserve_all_direction_boundaries_and_wrap() {
    for origin in [2, u64::MAX - 1] {
        for (first_write, second_write, distance) in [
            (false, false, 2),
            (true, true, 2),
            (true, false, 1),
            (false, true, 3),
        ] {
            for extra in [0, 17] {
                let mut bus = Bus::default();
                let setup =
                    GuardedBus::new(&mut bus, BusTiming::default(), origin.wrapping_sub(1), 0)
                        .finish();
                let mut first = GuardedBus::new(&mut bus, setup, origin, 0);
                if first_write {
                    assert_eq!(first.write_word(0x1000, 0x1234), BusResult::Ready(()));
                } else {
                    assert_eq!(first.read_word(0x1000), BusResult::Ready(0x4e71));
                }
                let saved = serde_json::to_string(&first.finish()).unwrap();
                let mut second = GuardedBus::new(
                    &mut bus,
                    serde_json::from_str(&saved).unwrap(),
                    origin.wrapping_add(distance + extra),
                    0,
                );
                if second_write {
                    assert_eq!(second.write_word(0x1002, 0x5678), BusResult::Ready(()));
                } else {
                    assert_eq!(second.read_word(0x1002), BusResult::Ready(0x4e71));
                }
                drop(second);
                assert_eq!(bus.calls, 2);
            }
        }
    }
}

#[test]
fn first_write_requires_setup_and_accepted_retry_starts_spacing_at_acceptance() {
    for origin in [0, 7, u64::MAX] {
        let mut bus = Bus::default();
        let mut first = GuardedBus::new(&mut bus, BusTiming::default(), origin, 0);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            first.write_word(0x1000, 0x1234);
        }))
        .is_err());
        drop(first);
        assert_eq!(bus.calls, 0);
    }
    for write in [false, true] {
        let mut bus = Bus {
            wait: true,
            ..Bus::default()
        };
        let mut timing = GuardedBus::new(&mut bus, BusTiming::default(), 1, 0).finish();
        for cck in 2..=4 {
            let mut retry = GuardedBus::new(&mut bus, timing, cck, 0);
            let waiting = if write {
                retry.write_word(0x1000, 0x1234).is_wait()
            } else {
                retry.read_word(0x1000).is_wait()
            };
            assert!(waiting);
            timing = retry.finish();
        }
        bus.wait = false;
        let mut accepted = GuardedBus::new(&mut bus, timing, 5, 0);
        if write {
            assert_eq!(accepted.write_word(0x1000, 0x1234), BusResult::Ready(()));
            accepted.require_instruction_completion();
        } else {
            assert_eq!(accepted.read_word(0x1000), BusResult::Ready(0x4e71));
        }
        let saved = serde_json::to_string(&accepted.finish()).unwrap();
        let next = if write { 6 } else { 7 };
        let mut read = GuardedBus::new(&mut bus, serde_json::from_str(&saved).unwrap(), next, 0);
        read.require_instruction_completion();
        assert_eq!(read.read_word(0x1002), BusResult::Ready(0x4e71));
        drop(read);
        assert_eq!(bus.calls, 5);
    }
}

#[test]
fn instruction_stepping_and_statistical_clock_reset_keep_bus_phases() {
    let mut bus = Bus::default();
    let mut cpu = cpu::Cpu::new();
    cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
    assert_eq!(cpu.step_instruction(&mut bus), 4);
    cpu.reset_cycle_counter();
    assert_eq!(cpu.step_instruction(&mut bus), 4);
    assert_eq!(bus.calls, 2);
}

#[test]
fn both_cpu_engines_stop_inside_a_duplicate_bus_callback() {
    fn duplicate(_: &mut cpu::Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
        let _ = bus.read_word(0x1000);
        let _ = bus.read_word(0x1002);
        BusResult::Ready(())
    }
    fn later(state: &mut cpu::CpuState, _: u8, _: u8) {
        state.set_d_long(0, 99);
    }
    static STEPS: [cpu::MicroStep; 2] =
        [cpu::MicroStep::cck(duplicate), cpu::MicroStep::alu(later)];
    for instruction in [false, true] {
        for wait in [false, true] {
            let mut cpu = cpu::Cpu::new();
            cpu.state.micro_mut().current_steps = &STEPS;
            let mut bus = Bus {
                wait,
                ..Bus::default()
            };
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if instruction {
                    cpu.step_instruction(&mut bus);
                } else {
                    cpu.step_cck(&mut bus);
                }
            }))
            .unwrap_err();
            let message = failure.downcast_ref::<String>().unwrap();
            assert!(message.contains("CPU bus timing/access invariant"));
            assert_eq!(bus.calls, 1);
            assert_eq!(cpu.state.d_long(0), 0);
            assert!(!cpu.state.is_halted());
        }
    }
}

#[test]
fn read_completion_cannot_be_omitted_at_instruction_retirement() {
    fn read(_: &mut cpu::Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
        match bus.read_word(0x1000) {
            BusResult::Ready(_) => BusResult::Ready(()),
            BusResult::WaitState => BusResult::WaitState,
        }
    }
    fn discard_refusal(_: &mut cpu::Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
        let _ = bus.read_word(0x1000);
        BusResult::Ready(())
    }
    static SHORT: [cpu::MicroStep; 1] = [cpu::MicroStep::cck(read)];
    static DISCARDED: [cpu::MicroStep; 1] = [cpu::MicroStep::cck(discard_refusal)];
    static COMPLETE: [cpu::MicroStep; 2] = [cpu::MicroStep::cck(read), cpu::MicroStep::cck_idle()];
    for instruction in [false, true] {
        for (steps, wait) in [(&SHORT[..], false), (&DISCARDED[..], true)] {
            let mut cpu = cpu::Cpu::new();
            let mut bus = Bus {
                wait,
                ..Bus::default()
            };
            cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
            cpu.state.micro_mut().current_steps = steps;
            let pc = cpu.state.instruction_pc();
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if instruction {
                    cpu.step_instruction(&mut bus);
                } else {
                    cpu.step_cck(&mut bus);
                }
            }));
            assert!(
                failure.is_err(),
                "A read instruction retired without its completion CCK"
            );
            assert_eq!(cpu.state.instruction_pc(), pc);
            assert_eq!(bus.calls, 1);
        }
        // A separate healthy fixture completes in exactly two CCKs.
        let mut cpu = cpu::Cpu::new();
        let mut bus = Bus::default();
        cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
        cpu.state.micro_mut().current_steps = &COMPLETE;
        if instruction {
            assert_eq!(cpu.step_instruction(&mut bus), 4);
        } else {
            assert!(!cpu.step_cck(&mut bus));
            assert!(cpu.step_cck(&mut bus));
        }
        assert_eq!(bus.calls, 1);
        assert_eq!(cpu.cycle_counter(), 4);
    }
}

#[test]
fn repeated_clock_fails_in_both_cpu_engines_before_a_second_phase() {
    for instruction in [false, true] {
        let mut cpu = cpu::Cpu::new();
        let mut bus = Bus {
            fixed_cck: Some(7),
            ..Bus::default()
        };
        cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
        let pc = cpu.state.instruction_pc();
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if instruction {
                cpu.step_instruction(&mut bus);
            } else {
                assert!(!cpu.step_cck(&mut bus));
                cpu.step_cck(&mut bus);
            }
        }))
        .unwrap_err();
        assert!(failure
            .downcast_ref::<String>()
            .unwrap()
            .contains("repeated CCK=7"));
        assert_eq!(bus.calls, 1);
        assert_eq!(cpu.cycle_counter(), 2);
        assert_eq!(cpu.state.instruction_pc(), pc);
        assert!(!cpu.state.is_halted());
    }
}
