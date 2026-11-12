//! Gary CIA selection, atomic synchronized acceptance, and committed OVL routing.

use crate::MemoryBus;
use cia::Cia;
use config::address_map::{CIA_BANK_FIRST, CIA_BANK_LAST};
use config::{cia_reg, OPEN_BUS_BYTE};
use physical_memory::{BusResult, PhysicalMemory};

/// Full canonical identities for the two independently selected receivers.
struct CiaSelection {
    a: Option<u32>,
    b: Option<u32>,
}

impl CiaSelection {
    const BIT_SELECT_A: u32 = 1 << 12;
    const BIT_SELECT_B: u32 = 1 << 13;
    const MASK_REGISTER: u32 = 0x0F00;
    const REGISTER_UNCONNECTED: u32 = 0x0B00;

    fn decode(addr: u32) -> Self {
        let bank = (addr >> 16) as u8;
        let register = addr & Self::MASK_REGISTER;
        if !(CIA_BANK_FIRST..=CIA_BANK_LAST).contains(&bank)
            || register == Self::REGISTER_UNCONNECTED
        {
            return Self { a: None, b: None };
        }
        Self {
            a: (addr & Self::BIT_SELECT_A == 0).then_some(cia_reg::cia_a::REGISTER_PRA | register),
            b: (addr & Self::BIT_SELECT_B == 0).then_some(cia_reg::cia_b::REGISTER_PRA | register),
        }
    }

    fn read_byte(addr: u32) -> Self {
        let mut selected = Self::decode(addr);
        if addr & 1 == 0 {
            selected.a = None;
        } else {
            selected.b = None;
        }
        selected
    }
}

impl MemoryBus<'_> {
    fn accept_cia_access(&mut self, selected: &CiaSelection) -> bool {
        // Prepare both independently: short circuiting would delay the second chip.
        let ready_a = selected.a.is_none() || self.cia_a.synchronize_bus_access();
        let ready_b = selected.b.is_none() || self.cia_b.synchronize_bus_access();
        if !ready_a || !ready_b {
            return false;
        }
        if selected.a.is_some() {
            self.cia_a.finish_bus_access();
        }
        if selected.b.is_some() {
            self.cia_b.finish_bus_access();
        }
        true
    }

    pub(super) fn read_cia_byte(&mut self, addr: u32) -> BusResult<u8> {
        let selected = CiaSelection::read_byte(addr);
        if !self.accept_cia_access(&selected) {
            return BusResult::WaitState;
        }
        BusResult::Ready(match selected.a.or(selected.b) {
            Some(canonical) => self.read_cia_register(canonical),
            None => OPEN_BUS_BYTE,
        })
    }

    pub(super) fn read_cia_word(&mut self, addr: u32) -> BusResult<u16> {
        let selected = CiaSelection::decode(addr);
        if !self.accept_cia_access(&selected) {
            return BusResult::WaitState;
        }
        let high = selected
            .b
            .map_or(OPEN_BUS_BYTE, |reg| self.read_cia_register(reg));
        let low = selected
            .a
            .map_or(OPEN_BUS_BYTE, |reg| self.read_cia_register(reg));
        BusResult::Ready(u16::from_be_bytes([high, low]))
    }

    pub(super) fn read_cia_word_debug(&self, addr: u32) -> u16 {
        let selected = CiaSelection::decode(addr);
        let high = selected
            .b
            .map_or(OPEN_BUS_BYTE, |reg| self.read_cia_register_debug(reg));
        let low = selected
            .a
            .map_or(OPEN_BUS_BYTE, |reg| self.read_cia_register_debug(reg));
        u16::from_be_bytes([high, low])
    }

    pub(super) fn write_cia_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        self.write_cia_word(addr, u16::from_be_bytes([val; 2]))
    }

    pub(super) fn write_cia_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let selected = CiaSelection::decode(addr);
        if !self.accept_cia_access(&selected) {
            return BusResult::WaitState;
        }
        let [high, low] = val.to_be_bytes();
        if let Some(reg) = selected.b {
            self.write_cia_register(reg, high);
        }
        if let Some(reg) = selected.a {
            self.write_cia_register(reg, low);
        }
        BusResult::Ready(())
    }

    fn read_cia_register(&mut self, canonical: u32) -> u8 {
        match canonical {
            cia_reg::cia_a::REGISTER_PRA => self.cia_a.pra(),
            cia_reg::cia_a::REGISTER_PRB => self.cia_a.prb(),
            cia_reg::cia_a::REGISTER_DDRA => self.cia_a.ddra(),
            cia_reg::cia_a::REGISTER_DDRB => self.cia_a.ddrb(),
            cia_reg::cia_a::REGISTER_TALO => self.cia_a.talo(),
            cia_reg::cia_a::REGISTER_TAHI => self.cia_a.tahi(),
            cia_reg::cia_a::REGISTER_TBLO => self.cia_a.tblo(),
            cia_reg::cia_a::REGISTER_TBHI => self.cia_a.tbhi(),
            cia_reg::cia_a::REGISTER_TODLO => self.cia_a.todlo(),
            cia_reg::cia_a::REGISTER_TODMID => self.cia_a.todmid(),
            cia_reg::cia_a::REGISTER_TODHI => self.cia_a.todhi(),
            cia_reg::cia_a::REGISTER_SDR => self.cia_a.sdr(),
            cia_reg::cia_a::REGISTER_ICR => self.cia_a.icr(),
            cia_reg::cia_a::REGISTER_CRA => self.cia_a.cra(),
            cia_reg::cia_a::REGISTER_CRB => self.cia_a.crb(),
            cia_reg::cia_b::REGISTER_PRA => self.cia_b.pra(),
            cia_reg::cia_b::REGISTER_PRB => self.cia_b.prb(),
            cia_reg::cia_b::REGISTER_DDRA => self.cia_b.ddra(),
            cia_reg::cia_b::REGISTER_DDRB => self.cia_b.ddrb(),
            cia_reg::cia_b::REGISTER_TALO => self.cia_b.talo(),
            cia_reg::cia_b::REGISTER_TAHI => self.cia_b.tahi(),
            cia_reg::cia_b::REGISTER_TBLO => self.cia_b.tblo(),
            cia_reg::cia_b::REGISTER_TBHI => self.cia_b.tbhi(),
            cia_reg::cia_b::REGISTER_TODLO => self.cia_b.todlo(),
            cia_reg::cia_b::REGISTER_TODMID => self.cia_b.todmid(),
            cia_reg::cia_b::REGISTER_TODHI => self.cia_b.todhi(),
            cia_reg::cia_b::REGISTER_SDR => self.cia_b.sdr(),
            cia_reg::cia_b::REGISTER_ICR => self.cia_b.icr(),
            cia_reg::cia_b::REGISTER_CRA => self.cia_b.cra(),
            cia_reg::cia_b::REGISTER_CRB => self.cia_b.crb(),
            _ => OPEN_BUS_BYTE,
        }
    }

    fn read_cia_register_debug(&self, canonical: u32) -> u8 {
        match canonical {
            cia_reg::cia_a::REGISTER_PRA => self.cia_a.pra(),
            cia_reg::cia_a::REGISTER_PRB => self.cia_a.prb(),
            cia_reg::cia_a::REGISTER_DDRA => self.cia_a.ddra(),
            cia_reg::cia_a::REGISTER_DDRB => self.cia_a.ddrb(),
            cia_reg::cia_a::REGISTER_TALO => self.cia_a.talo(),
            cia_reg::cia_a::REGISTER_TAHI => self.cia_a.tahi(),
            cia_reg::cia_a::REGISTER_TBLO => self.cia_a.tblo(),
            cia_reg::cia_a::REGISTER_TBHI => self.cia_a.tbhi(),
            cia_reg::cia_a::REGISTER_TODLO => self.cia_a.todlo_debug(),
            cia_reg::cia_a::REGISTER_TODMID => self.cia_a.todmid_debug(),
            cia_reg::cia_a::REGISTER_TODHI => self.cia_a.todhi_debug(),
            cia_reg::cia_a::REGISTER_SDR => self.cia_a.sdr(),
            cia_reg::cia_a::REGISTER_ICR => self.cia_a.icr_debug(),
            cia_reg::cia_a::REGISTER_CRA => self.cia_a.cra(),
            cia_reg::cia_a::REGISTER_CRB => self.cia_a.crb(),
            cia_reg::cia_b::REGISTER_PRA => self.cia_b.pra(),
            cia_reg::cia_b::REGISTER_PRB => self.cia_b.prb(),
            cia_reg::cia_b::REGISTER_DDRA => self.cia_b.ddra(),
            cia_reg::cia_b::REGISTER_DDRB => self.cia_b.ddrb(),
            cia_reg::cia_b::REGISTER_TALO => self.cia_b.talo(),
            cia_reg::cia_b::REGISTER_TAHI => self.cia_b.tahi(),
            cia_reg::cia_b::REGISTER_TBLO => self.cia_b.tblo(),
            cia_reg::cia_b::REGISTER_TBHI => self.cia_b.tbhi(),
            cia_reg::cia_b::REGISTER_TODLO => self.cia_b.todlo_debug(),
            cia_reg::cia_b::REGISTER_TODMID => self.cia_b.todmid_debug(),
            cia_reg::cia_b::REGISTER_TODHI => self.cia_b.todhi_debug(),
            cia_reg::cia_b::REGISTER_SDR => self.cia_b.sdr(),
            cia_reg::cia_b::REGISTER_ICR => self.cia_b.icr_debug(),
            cia_reg::cia_b::REGISTER_CRA => self.cia_b.cra(),
            cia_reg::cia_b::REGISTER_CRB => self.cia_b.crb(),
            _ => OPEN_BUS_BYTE,
        }
    }

    fn write_cia_register(&mut self, canonical: u32, val: u8) {
        match canonical {
            cia_reg::cia_a::REGISTER_PRA => {
                self.cia_a.set_pra(val);
            }
            cia_reg::cia_a::REGISTER_PRB => {
                self.cia_a.set_prb(val);
            }
            cia_reg::cia_a::REGISTER_DDRA => {
                self.cia_a.set_ddra(val);
            }
            cia_reg::cia_a::REGISTER_DDRB => {
                self.cia_a.set_ddrb(val);
            }
            cia_reg::cia_a::REGISTER_TALO => {
                self.cia_a.set_talo(val);
            }
            cia_reg::cia_a::REGISTER_TAHI => {
                self.cia_a.set_tahi(val);
            }
            cia_reg::cia_a::REGISTER_TBLO => {
                self.cia_a.set_tblo(val);
            }
            cia_reg::cia_a::REGISTER_TBHI => {
                self.cia_a.set_tbhi(val);
            }
            cia_reg::cia_a::REGISTER_TODLO => {
                self.cia_a.set_todlo(val);
            }
            cia_reg::cia_a::REGISTER_TODMID => {
                self.cia_a.set_todmid(val);
            }
            cia_reg::cia_a::REGISTER_TODHI => {
                self.cia_a.set_todhi(val);
            }
            cia_reg::cia_a::REGISTER_SDR => {
                self.cia_a.set_sdr(val);
            }
            cia_reg::cia_a::REGISTER_ICR => {
                self.cia_a.set_icr(val);
            }
            cia_reg::cia_a::REGISTER_CRA => {
                self.cia_a.set_cra(val);
            }
            cia_reg::cia_a::REGISTER_CRB => {
                self.cia_a.set_crb(val);
            }
            cia_reg::cia_b::REGISTER_PRA => {
                self.cia_b.set_pra(val);
            }
            cia_reg::cia_b::REGISTER_PRB => {
                self.cia_b.set_prb(val);
            }
            cia_reg::cia_b::REGISTER_DDRA => {
                self.cia_b.set_ddra(val);
            }
            cia_reg::cia_b::REGISTER_DDRB => {
                self.cia_b.set_ddrb(val);
            }
            cia_reg::cia_b::REGISTER_TALO => {
                self.cia_b.set_talo(val);
            }
            cia_reg::cia_b::REGISTER_TAHI => {
                self.cia_b.set_tahi(val);
            }
            cia_reg::cia_b::REGISTER_TBLO => {
                self.cia_b.set_tblo(val);
            }
            cia_reg::cia_b::REGISTER_TBHI => {
                self.cia_b.set_tbhi(val);
            }
            cia_reg::cia_b::REGISTER_TODLO => {
                self.cia_b.set_todlo(val);
            }
            cia_reg::cia_b::REGISTER_TODMID => {
                self.cia_b.set_todmid(val);
            }
            cia_reg::cia_b::REGISTER_TODHI => {
                self.cia_b.set_todhi(val);
            }
            cia_reg::cia_b::REGISTER_SDR => {
                self.cia_b.set_sdr(val);
            }
            cia_reg::cia_b::REGISTER_ICR => {
                self.cia_b.set_icr(val);
            }
            cia_reg::cia_b::REGISTER_CRA => {
                self.cia_b.set_cra(val);
            }
            cia_reg::cia_b::REGISTER_CRB => {
                self.cia_b.set_crb(val);
            }
            _ => {}
        }
    }

    /// Applies the committed CIA-A PA0 output without another propagation stage.
    pub fn update_ciaa_overlay(mem: &mut PhysicalMemory, cia_a: &Cia) {
        let ovl_high = (cia_a.pra | !cia_a.ddra) & Self::BIT_CIAA_PA_OVL != 0;
        if ovl_high {
            mem.map_kickstart_to_low_memory();
        } else {
            mem.map_chip_ram_to_low_memory();
        }
    }
}
