//! Motherboard wiring and phases; receivers retain their own incoming inputs.

use crate::{A500Machine, MemoryBus};
use config::mask::game_port::{BIT_POT_OUTPUT_ENABLE_SHIFT, MASK_POT_DATA};

const MASK_CIAA_PA_BOARD_CONTROL: u8 = 0x03;
const BIT_CIAB_PA_SP: u8 = 0x01;
const BIT_CIAB_PA_CNT: u8 = 0x02;

impl A500Machine {
    /// Routes same-CCK outputs and captures pins for sampling at the next CCK start.
    /// Also initializes incoming inputs during construction and reset.
    pub(super) fn route_signal_pins_end_cck(&mut self) {
        // Committed PA0 reaches OVL in this CCK, before CPU bus access.
        if self.cia_a.poll_pra_output().is_some() {
            MemoryBus::update_ciaa_overlay(&mut self.physical_memory, &self.cia_a);
        }

        let host_drives = self.cia_a.is_sdr_output();
        let host_data_high =
            self.keyboard.pins.data_high && (!host_drives || self.cia_a.pins.sp_output_high);
        // No parallel endpoint drives these nets. PA0/SP and PA1/CNT share wiring.
        let mut ciab_serial_levels = self.cia_b.port_a_output_pins();
        if self.cia_b.is_sdr_output() {
            if !self.cia_b.pins.sp_output_high {
                ciab_serial_levels &= !BIT_CIAB_PA_SP;
            }
            if !self.cia_b.pins.cnt_output_high {
                ciab_serial_levels &= !BIT_CIAB_PA_CNT;
            }
        }
        // Disconnected CIA-A PB/FLAG and CIA-B PA/PB inputs retain reset levels.
        // FIRE is sampled from the current device at the next input phase.
        self.cia_a.pins.incoming_port_a = Some(
            self.floppy.sample_ciaa_port_a_inputs()
                | game_ports::MASK_CIAA_PRA_FIRE
                | MASK_CIAA_PA_BOARD_CONTROL,
        );
        self.cia_a.pins.incoming = Some(cia::CiaInputs {
            sp_high: host_data_high,
            cnt_high: self.keyboard.pins.clock_high
                && (!host_drives || self.cia_a.pins.cnt_output_high),
            tod_high: self.agnus.pins.vsync_high,
        });
        self.cia_b.pins.incoming_flag_high = Some(self.floppy.pins.index_high);
        self.cia_b.pins.incoming_port_a = Some(ciab_serial_levels);
        self.cia_b.pins.incoming = Some(cia::CiaInputs {
            sp_high: ciab_serial_levels & BIT_CIAB_PA_SP != 0,
            cnt_high: ciab_serial_levels & BIT_CIAB_PA_CNT != 0,
            tod_high: self.agnus.pins.hsync_high,
        });
        self.paula.pins.incoming = Some(paula::PaulaInputs {
            int2_high: !self.cia_a.irq_pending(),
            int3_high: self.agnus.pins.int3_high,
            int6_high: !self.cia_b.irq_pending(),
            disk_read_high: self.floppy.pins.read_data_high,
            pot0dat: self.game_ports.pot0dat(),
            pot1dat: self.game_ports.pot1dat(),
            rga_strobe: self.agnus.pins.rga_strobe,
        });
        self.floppy.pins.incoming = Some(floppy::FloppyInputs {
            control: self.cia_b.port_b_output_pins(),
            write_data_high: self.paula.pins.disk_write_high,
            // Gary/U36 buffering yields an active-low connector write gate.
            write_enable_high: !self.paula.pins.disk_write_enable,
        });
        self.denise.pins.incoming = Some(denise::DeniseInputs {
            rga_strobe: self.agnus.pins.rga_strobe,
        });
        self.game_ports.pins.incoming = Some(game_ports::GamePortsInputs {
            fire_output_mask: self.cia_a.ddra & game_ports::MASK_CIAA_PRA_FIRE,
            fire_output_high: (self.cia_a.pra | !self.cia_a.ddra) & game_ports::MASK_CIAA_PRA_FIRE,
            pot_output_mask: (self.paula.potgo >> BIT_POT_OUTPUT_ENABLE_SHIFT) & MASK_POT_DATA,
            pot_output_high: self.paula.potgo & MASK_POT_DATA,
        });
        self.agnus.pins.incoming_dmal_high = Some(self.paula.pins.dmal_output_high);
        self.keyboard.pins.incoming_host_data_high = Some(host_data_high);
        self.keyboard_reset_asserted = self.keyboard.pins.reset_line_asserted;
        let _ = self.cia_b.poll_prb_output();
    }

    /// Samples incoming inputs at the start of the subsystem CCK, before any engine advances.
    /// Independent external levels are sampled directly in this phase, even while the CPU waits.
    pub(super) fn sample_signal_pins_start_cck(&mut self) {
        self.agnus.sample_input_pins();
        self.denise.sample_input_pins();
        self.paula.sample_input_pins();
        self.cia_a.sample_input_pins();
        self.cia_b.sample_input_pins();
        self.floppy.sample_input_pins();
        self.keyboard.sample_input_pins();
        self.game_ports.sample_input_pins();
        self.game_ports.step_cck();

        // Independent external levels already denote arrival at receiver pins.
        self.cia_a.set_input_pins_a(
            self.game_ports.sample_ciaa_port_a_inputs(),
            game_ports::MASK_CIAA_PRA_FIRE,
        );
        self.denise
            .receive_game_port_pins(self.game_ports.pins.direction_pins);
        self.paula.receive_pot_inputs(self.game_ports.potgor());
    }

    /// Delivers Paula IPL before CPU sampling in the same CCK.
    pub(super) fn deliver_cpu_interrupt_pins(&mut self) {
        self.cpu.state.set_ipl(self.paula.pending_interrupt_level());
    }

    /// CPU RESET reaches the reset network after that CPU phase, once per assertion.
    pub(super) fn deliver_cpu_reset_pin(&mut self, previously_asserted: bool) {
        if !previously_asserted && self.cpu.state.is_reset_line_asserted() {
            self.reset_external_devices();
        }
    }

    /// Routes the retained Agnus RGA/data output, acknowledging only acceptance.
    pub(super) fn deliver_dma_register(&mut self) {
        if let Some((reg, word)) = self.agnus.pending_dma_write {
            if self.memory_bus().write_custom_word(reg, word) {
                self.agnus.pending_dma_write = None;
            }
        }
    }
}
