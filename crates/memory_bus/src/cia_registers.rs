//! Direct CIA register selection, synchronized bus acceptance, and committed OVL routing.

use crate::MemoryBus;
use cia::Cia;
use config::{cia_reg, OPEN_BUS_BYTE};
use physical_memory::{BusResult, PhysicalMemory};

impl MemoryBus<'_> {
    pub(super) fn read_cia_byte(&mut self, addr: u32) -> BusResult<u8> {
        match addr & Self::CIA_REG_ADDRESS_MASK {
            cia_reg::cia_a::PRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.pra())
            }
            cia_reg::cia_a::PRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.prb())
            }
            cia_reg::cia_a::DDRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.ddra())
            }
            cia_reg::cia_a::DDRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.ddrb())
            }
            cia_reg::cia_a::TALO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.talo())
            }
            cia_reg::cia_a::TAHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.tahi())
            }
            cia_reg::cia_a::TBLO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.tblo())
            }
            cia_reg::cia_a::TBHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.tbhi())
            }
            cia_reg::cia_a::TODLO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.todlo())
            }
            cia_reg::cia_a::TODMID => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.todmid())
            }
            cia_reg::cia_a::TODHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.todhi())
            }
            cia_reg::cia_a::SDR => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.sdr())
            }
            cia_reg::cia_a::ICR => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.icr())
            }
            cia_reg::cia_a::CRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.cra())
            }
            cia_reg::cia_a::CRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_a.crb())
            }
            cia_reg::cia_b::PRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.pra())
            }
            cia_reg::cia_b::PRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.prb())
            }
            cia_reg::cia_b::DDRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.ddra())
            }
            cia_reg::cia_b::DDRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.ddrb())
            }
            cia_reg::cia_b::TALO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.talo())
            }
            cia_reg::cia_b::TAHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.tahi())
            }
            cia_reg::cia_b::TBLO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.tblo())
            }
            cia_reg::cia_b::TBHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.tbhi())
            }
            cia_reg::cia_b::TODLO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.todlo())
            }
            cia_reg::cia_b::TODMID => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.todmid())
            }
            cia_reg::cia_b::TODHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.todhi())
            }
            cia_reg::cia_b::SDR => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.sdr())
            }
            cia_reg::cia_b::ICR => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.icr())
            }
            cia_reg::cia_b::CRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.cra())
            }
            cia_reg::cia_b::CRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                BusResult::Ready(self.cia_b.crb())
            }
            _ => BusResult::Ready(OPEN_BUS_BYTE),
        }
    }

    /// Inspects CIA state without clocking, acknowledging, or scheduling access.
    pub fn read_cia_byte_debug(&self, addr: u32) -> u8 {
        match addr & Self::CIA_REG_ADDRESS_MASK {
            cia_reg::cia_a::PRA => self.cia_a.pra(),
            cia_reg::cia_a::PRB => self.cia_a.prb(),
            cia_reg::cia_a::DDRA => self.cia_a.ddra(),
            cia_reg::cia_a::DDRB => self.cia_a.ddrb(),
            cia_reg::cia_a::TALO => self.cia_a.talo(),
            cia_reg::cia_a::TAHI => self.cia_a.tahi(),
            cia_reg::cia_a::TBLO => self.cia_a.tblo(),
            cia_reg::cia_a::TBHI => self.cia_a.tbhi(),
            cia_reg::cia_a::TODLO => self.cia_a.todlo_debug(),
            cia_reg::cia_a::TODMID => self.cia_a.todmid_debug(),
            cia_reg::cia_a::TODHI => self.cia_a.todhi_debug(),
            cia_reg::cia_a::SDR => self.cia_a.sdr(),
            cia_reg::cia_a::ICR => self.cia_a.icr_debug(),
            cia_reg::cia_a::CRA => self.cia_a.cra(),
            cia_reg::cia_a::CRB => self.cia_a.crb(),
            cia_reg::cia_b::PRA => self.cia_b.pra(),
            cia_reg::cia_b::PRB => self.cia_b.prb(),
            cia_reg::cia_b::DDRA => self.cia_b.ddra(),
            cia_reg::cia_b::DDRB => self.cia_b.ddrb(),
            cia_reg::cia_b::TALO => self.cia_b.talo(),
            cia_reg::cia_b::TAHI => self.cia_b.tahi(),
            cia_reg::cia_b::TBLO => self.cia_b.tblo(),
            cia_reg::cia_b::TBHI => self.cia_b.tbhi(),
            cia_reg::cia_b::TODLO => self.cia_b.todlo_debug(),
            cia_reg::cia_b::TODMID => self.cia_b.todmid_debug(),
            cia_reg::cia_b::TODHI => self.cia_b.todhi_debug(),
            cia_reg::cia_b::SDR => self.cia_b.sdr(),
            cia_reg::cia_b::ICR => self.cia_b.icr_debug(),
            cia_reg::cia_b::CRA => self.cia_b.cra(),
            cia_reg::cia_b::CRB => self.cia_b.crb(),
            _ => OPEN_BUS_BYTE,
        }
    }

    pub(super) fn write_cia_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        match addr & Self::CIA_REG_ADDRESS_MASK {
            cia_reg::cia_a::PRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_pra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::PRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_prb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::DDRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_ddra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::DDRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_ddrb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TALO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_talo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TAHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_tahi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TBLO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_tblo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TBHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_tbhi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TODLO => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_todlo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TODMID => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_todmid(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::TODHI => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_todhi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::SDR => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_sdr(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::ICR => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_icr(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::CRA => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_cra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_a::CRB => {
                if !self.cia_a.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_a.set_crb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::PRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_pra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::PRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_prb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::DDRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_ddra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::DDRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_ddrb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TALO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_talo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TAHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_tahi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TBLO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_tblo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TBHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_tbhi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TODLO => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_todlo(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TODMID => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_todmid(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::TODHI => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_todhi(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::SDR => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_sdr(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::ICR => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_icr(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::CRA => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_cra(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            cia_reg::cia_b::CRB => {
                if !self.cia_b.accept_bus_access() {
                    return BusResult::WaitState;
                }
                if self.cia_b.set_crb(val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            _ => BusResult::Ready(()),
        }
    }

    /// Applies the committed CIA-A PA0 output without another propagation stage.
    pub fn update_ciaa_overlay(mem: &mut PhysicalMemory, cia_a: &Cia) {
        let ovl_high = (cia_a.pra | !cia_a.ddra) & Self::CIAA_PA_OVL != 0;
        if ovl_high {
            mem.map_kickstart_to_low_memory();
        } else {
            mem.map_chip_ram_to_low_memory();
        }
    }
}
