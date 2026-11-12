#![allow(dead_code)]

use machine_loop::A500Machine;
use physical_memory::{AddressBus, BusResult};

/// Retries one stable write until accepted, leaving it pending before the falling edge.
pub fn accept_write(machine: &mut A500Machine, addr: u32, value: u8) -> BusResult<()> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(()) = machine.memory_bus().write_byte(addr, value) {
            return result;
        }
        machine.step_cck();
    }
    panic!("CIA write did not synchronize");
}

/// Completes a write and its following falling-edge capture through the real machine.
pub fn write_byte(machine: &mut A500Machine, addr: u32, value: u8) -> BusResult<()> {
    let result = accept_write(machine, addr, value);
    machine.step_cck();
    result
}

/// Starts with released control lines, then enables CIA-B port B output drivers.
pub fn configure_drive_control_outputs(machine: &mut A500Machine) {
    write_byte(machine, config::cia_reg::cia_b::REGISTER_PRB, 0xff);
    write_byte(machine, config::cia_reg::cia_b::REGISTER_DDRB, 0xff);
}

/// Retries a stable live read; a refused access must have no read side effects.
pub fn read_byte(machine: &mut A500Machine, addr: u32) -> BusResult<u8> {
    for _ in 0..12 {
        if let result @ BusResult::Ready(_) = machine.memory_bus().read_byte(addr) {
            return result;
        }
        machine.step_cck();
    }
    panic!("CIA read did not synchronize");
}
