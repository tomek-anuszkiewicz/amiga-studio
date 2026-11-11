//! Direct custom-register write routing.

use super::MemoryBus;
use config::custom_reg;

impl MemoryBus<'_> {
    /// Queues one accepted custom transfer on every modeled receiver.
    /// False means no receiver accepted it; the producer must retry.
    pub fn write_custom_word(&mut self, offset: u16, val: u16) -> bool {
        let offset = offset & Self::CUSTOM_REG_OFFSET_MASK;
        match offset {
            custom_reg::denise::STREQU => self.denise.set_strequ(),
            custom_reg::denise::STRVBL => self.denise.set_strvbl(),
            custom_reg::denise::STRLONG => self.denise.set_strlong(),
            custom_reg::denise::STRHOR => {
                if !self.denise.can_accept_register_write()
                    || !self.paula.can_accept_register_write()
                {
                    return false;
                }
                self.denise.set_strhor() && self.paula.set_strhor()
            }
            custom_reg::agnus::DMACON => {
                if !self.agnus.can_accept_register_write()
                    || !self.paula.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_dmacon(val) && self.paula.set_dmacon(val)
            }
            custom_reg::denise::BPLCON0 => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_bplcon0(val) && self.denise.set_bplcon0(val)
            }
            custom_reg::agnus::DIWSTRT => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_diwstrt(val) && self.denise.set_diwstrt(val)
            }
            custom_reg::agnus::DIWSTOP => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_diwstop(val) && self.denise.set_diwstop(val)
            }
            custom_reg::denise::SPR0POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(0, val) && self.denise.set_sprpos(0, val)
            }
            custom_reg::denise::SPR0CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(0, val) && self.denise.set_sprctl(0, val)
            }
            custom_reg::denise::SPR1POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(1, val) && self.denise.set_sprpos(1, val)
            }
            custom_reg::denise::SPR1CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(1, val) && self.denise.set_sprctl(1, val)
            }
            custom_reg::denise::SPR2POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(2, val) && self.denise.set_sprpos(2, val)
            }
            custom_reg::denise::SPR2CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(2, val) && self.denise.set_sprctl(2, val)
            }
            custom_reg::denise::SPR3POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(3, val) && self.denise.set_sprpos(3, val)
            }
            custom_reg::denise::SPR3CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(3, val) && self.denise.set_sprctl(3, val)
            }
            custom_reg::denise::SPR4POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(4, val) && self.denise.set_sprpos(4, val)
            }
            custom_reg::denise::SPR4CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(4, val) && self.denise.set_sprctl(4, val)
            }
            custom_reg::denise::SPR5POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(5, val) && self.denise.set_sprpos(5, val)
            }
            custom_reg::denise::SPR5CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(5, val) && self.denise.set_sprctl(5, val)
            }
            custom_reg::denise::SPR6POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(6, val) && self.denise.set_sprpos(6, val)
            }
            custom_reg::denise::SPR6CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(6, val) && self.denise.set_sprctl(6, val)
            }
            custom_reg::denise::SPR7POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(7, val) && self.denise.set_sprpos(7, val)
            }
            custom_reg::denise::SPR7CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(7, val) && self.denise.set_sprctl(7, val)
            }
            custom_reg::denise::BPLCON1 => self.denise.set_bplcon1(val),
            custom_reg::denise::CLXCON => self.denise.set_clxcon(val),
            custom_reg::denise::BPLCON2 => self.denise.set_bplcon2(val),
            custom_reg::denise::BPL1DAT => self.denise.set_bpldat(0, val),
            custom_reg::denise::BPL2DAT => self.denise.set_bpldat(1, val),
            custom_reg::denise::BPL3DAT => self.denise.set_bpldat(2, val),
            custom_reg::denise::BPL4DAT => self.denise.set_bpldat(3, val),
            custom_reg::denise::BPL5DAT => self.denise.set_bpldat(4, val),
            custom_reg::denise::BPL6DAT => self.denise.set_bpldat(5, val),
            custom_reg::denise::SPR0DATA => self.denise.set_sprdata(0, val),
            custom_reg::denise::SPR0DATB => self.denise.set_sprdatb(0, val),
            custom_reg::denise::SPR1DATA => self.denise.set_sprdata(1, val),
            custom_reg::denise::SPR1DATB => self.denise.set_sprdatb(1, val),
            custom_reg::denise::SPR2DATA => self.denise.set_sprdata(2, val),
            custom_reg::denise::SPR2DATB => self.denise.set_sprdatb(2, val),
            custom_reg::denise::SPR3DATA => self.denise.set_sprdata(3, val),
            custom_reg::denise::SPR3DATB => self.denise.set_sprdatb(3, val),
            custom_reg::denise::SPR4DATA => self.denise.set_sprdata(4, val),
            custom_reg::denise::SPR4DATB => self.denise.set_sprdatb(4, val),
            custom_reg::denise::SPR5DATA => self.denise.set_sprdata(5, val),
            custom_reg::denise::SPR5DATB => self.denise.set_sprdatb(5, val),
            custom_reg::denise::SPR6DATA => self.denise.set_sprdata(6, val),
            custom_reg::denise::SPR6DATB => self.denise.set_sprdatb(6, val),
            custom_reg::denise::SPR7DATA => self.denise.set_sprdata(7, val),
            custom_reg::denise::SPR7DATB => self.denise.set_sprdatb(7, val),
            custom_reg::denise::COLOR00 => self.denise.set_color(0, val),
            custom_reg::denise::COLOR01 => self.denise.set_color(1, val),
            custom_reg::denise::COLOR02 => self.denise.set_color(2, val),
            custom_reg::denise::COLOR03 => self.denise.set_color(3, val),
            custom_reg::denise::COLOR04 => self.denise.set_color(4, val),
            custom_reg::denise::COLOR05 => self.denise.set_color(5, val),
            custom_reg::denise::COLOR06 => self.denise.set_color(6, val),
            custom_reg::denise::COLOR07 => self.denise.set_color(7, val),
            custom_reg::denise::COLOR08 => self.denise.set_color(8, val),
            custom_reg::denise::COLOR09 => self.denise.set_color(9, val),
            custom_reg::denise::COLOR10 => self.denise.set_color(10, val),
            custom_reg::denise::COLOR11 => self.denise.set_color(11, val),
            custom_reg::denise::COLOR12 => self.denise.set_color(12, val),
            custom_reg::denise::COLOR13 => self.denise.set_color(13, val),
            custom_reg::denise::COLOR14 => self.denise.set_color(14, val),
            custom_reg::denise::COLOR15 => self.denise.set_color(15, val),
            custom_reg::denise::COLOR16 => self.denise.set_color(16, val),
            custom_reg::denise::COLOR17 => self.denise.set_color(17, val),
            custom_reg::denise::COLOR18 => self.denise.set_color(18, val),
            custom_reg::denise::COLOR19 => self.denise.set_color(19, val),
            custom_reg::denise::COLOR20 => self.denise.set_color(20, val),
            custom_reg::denise::COLOR21 => self.denise.set_color(21, val),
            custom_reg::denise::COLOR22 => self.denise.set_color(22, val),
            custom_reg::denise::COLOR23 => self.denise.set_color(23, val),
            custom_reg::denise::COLOR24 => self.denise.set_color(24, val),
            custom_reg::denise::COLOR25 => self.denise.set_color(25, val),
            custom_reg::denise::COLOR26 => self.denise.set_color(26, val),
            custom_reg::denise::COLOR27 => self.denise.set_color(27, val),
            custom_reg::denise::COLOR28 => self.denise.set_color(28, val),
            custom_reg::denise::COLOR29 => self.denise.set_color(29, val),
            custom_reg::denise::COLOR30 => self.denise.set_color(30, val),
            custom_reg::denise::COLOR31 => self.denise.set_color(31, val),
            custom_reg::denise::JOYTEST => self.denise.set_joytest(val),
            custom_reg::paula::INTENA => self.paula.set_intena(val),
            custom_reg::paula::INTREQ => self.paula.set_intreq(val),
            custom_reg::paula::ADKCON => self.paula.set_adkcon(val),
            custom_reg::paula::DSKDAT => self.paula.set_dskdat(val),
            custom_reg::paula::DSKLEN => self.paula.set_dsklen(val),
            custom_reg::paula::DSKSYNC => self.paula.set_dsksync(val),
            custom_reg::paula::SERDAT => self.paula.set_serdat(val),
            custom_reg::paula::SERPER => self.paula.set_serper(val),
            custom_reg::paula::POTGO => self.paula.set_potgo(val),
            custom_reg::paula::AUD0LEN => self.paula.set_audlen(0, val),
            custom_reg::paula::AUD0PER => self.paula.set_audper(0, val),
            custom_reg::paula::AUD0VOL => self.paula.set_audvol(0, val),
            custom_reg::paula::AUD0DAT => self.paula.set_auddat(0, val),
            custom_reg::paula::AUD1LEN => self.paula.set_audlen(1, val),
            custom_reg::paula::AUD1PER => self.paula.set_audper(1, val),
            custom_reg::paula::AUD1VOL => self.paula.set_audvol(1, val),
            custom_reg::paula::AUD1DAT => self.paula.set_auddat(1, val),
            custom_reg::paula::AUD2LEN => self.paula.set_audlen(2, val),
            custom_reg::paula::AUD2PER => self.paula.set_audper(2, val),
            custom_reg::paula::AUD2VOL => self.paula.set_audvol(2, val),
            custom_reg::paula::AUD2DAT => self.paula.set_auddat(2, val),
            custom_reg::paula::AUD3LEN => self.paula.set_audlen(3, val),
            custom_reg::paula::AUD3PER => self.paula.set_audper(3, val),
            custom_reg::paula::AUD3VOL => self.paula.set_audvol(3, val),
            custom_reg::paula::AUD3DAT => self.paula.set_auddat(3, val),
            custom_reg::agnus::DSKPTH => self.agnus.set_dskpth(val),
            custom_reg::agnus::DSKPTL => self.agnus.set_dskptl(val),
            custom_reg::agnus::REFPTR => self.agnus.set_refptr(val),
            custom_reg::agnus::VPOSW => self.agnus.set_vposw(val),
            custom_reg::agnus::VHPOSW => self.agnus.set_vhposw(val),
            custom_reg::agnus::COPCON => self.agnus.set_copcon(val),
            custom_reg::agnus::BLTCON0 => self.agnus.set_bltcon0(val),
            custom_reg::agnus::BLTCON1 => self.agnus.set_bltcon1(val),
            custom_reg::agnus::BLTAFWM => self.agnus.set_bltafwm(val),
            custom_reg::agnus::BLTALWM => self.agnus.set_bltalwm(val),
            custom_reg::agnus::BLTCPTH => self.agnus.set_bltcpth(val),
            custom_reg::agnus::BLTCPTL => self.agnus.set_bltcptl(val),
            custom_reg::agnus::BLTBPTH => self.agnus.set_bltbpth(val),
            custom_reg::agnus::BLTBPTL => self.agnus.set_bltbptl(val),
            custom_reg::agnus::BLTAPTH => self.agnus.set_bltapth(val),
            custom_reg::agnus::BLTAPTL => self.agnus.set_bltaptl(val),
            custom_reg::agnus::BLTDPTH => self.agnus.set_bltdpth(val),
            custom_reg::agnus::BLTDPTL => self.agnus.set_bltdptl(val),
            custom_reg::agnus::BLTSIZE => self.agnus.set_bltsize(val),
            custom_reg::agnus::BLTCMOD => self.agnus.set_bltcmod(val),
            custom_reg::agnus::BLTBMOD => self.agnus.set_bltbmod(val),
            custom_reg::agnus::BLTAMOD => self.agnus.set_bltamod(val),
            custom_reg::agnus::BLTDMOD => self.agnus.set_bltdmod(val),
            custom_reg::agnus::BLTCDAT => self.agnus.set_bltcdat(val),
            custom_reg::agnus::BLTBDAT => self.agnus.set_bltbdat(val),
            custom_reg::agnus::BLTADAT => self.agnus.set_bltadat(val),
            custom_reg::agnus::COP1LCH => self.agnus.set_cop1lch(val),
            custom_reg::agnus::COP1LCL => self.agnus.set_cop1lcl(val),
            custom_reg::agnus::COP2LCH => self.agnus.set_cop2lch(val),
            custom_reg::agnus::COP2LCL => self.agnus.set_cop2lcl(val),
            custom_reg::agnus::COPJMP1 => self.agnus.set_copjmp1(),
            custom_reg::agnus::COPJMP2 => self.agnus.set_copjmp2(),
            custom_reg::agnus::COPINS => self.agnus.set_copins(val),
            custom_reg::agnus::DDFSTRT => self.agnus.set_ddfstrt(val),
            custom_reg::agnus::DDFSTOP => self.agnus.set_ddfstop(val),
            custom_reg::agnus::AUD0LCH => self.agnus.set_audlch(0, val),
            custom_reg::agnus::AUD0LCL => self.agnus.set_audlcl(0, val),
            custom_reg::agnus::AUD1LCH => self.agnus.set_audlch(1, val),
            custom_reg::agnus::AUD1LCL => self.agnus.set_audlcl(1, val),
            custom_reg::agnus::AUD2LCH => self.agnus.set_audlch(2, val),
            custom_reg::agnus::AUD2LCL => self.agnus.set_audlcl(2, val),
            custom_reg::agnus::AUD3LCH => self.agnus.set_audlch(3, val),
            custom_reg::agnus::AUD3LCL => self.agnus.set_audlcl(3, val),
            custom_reg::agnus::BPL1PTH => self.agnus.set_bplpth(0, val),
            custom_reg::agnus::BPL1PTL => self.agnus.set_bplptl(0, val),
            custom_reg::agnus::BPL2PTH => self.agnus.set_bplpth(1, val),
            custom_reg::agnus::BPL2PTL => self.agnus.set_bplptl(1, val),
            custom_reg::agnus::BPL3PTH => self.agnus.set_bplpth(2, val),
            custom_reg::agnus::BPL3PTL => self.agnus.set_bplptl(2, val),
            custom_reg::agnus::BPL4PTH => self.agnus.set_bplpth(3, val),
            custom_reg::agnus::BPL4PTL => self.agnus.set_bplptl(3, val),
            custom_reg::agnus::BPL5PTH => self.agnus.set_bplpth(4, val),
            custom_reg::agnus::BPL5PTL => self.agnus.set_bplptl(4, val),
            custom_reg::agnus::BPL6PTH => self.agnus.set_bplpth(5, val),
            custom_reg::agnus::BPL6PTL => self.agnus.set_bplptl(5, val),
            custom_reg::agnus::BPL1MOD => self.agnus.set_bpl1mod(val),
            custom_reg::agnus::BPL2MOD => self.agnus.set_bpl2mod(val),
            custom_reg::agnus::SPR0PTH => self.agnus.set_sprpth(0, val),
            custom_reg::agnus::SPR0PTL => self.agnus.set_sprptl(0, val),
            custom_reg::agnus::SPR1PTH => self.agnus.set_sprpth(1, val),
            custom_reg::agnus::SPR1PTL => self.agnus.set_sprptl(1, val),
            custom_reg::agnus::SPR2PTH => self.agnus.set_sprpth(2, val),
            custom_reg::agnus::SPR2PTL => self.agnus.set_sprptl(2, val),
            custom_reg::agnus::SPR3PTH => self.agnus.set_sprpth(3, val),
            custom_reg::agnus::SPR3PTL => self.agnus.set_sprptl(3, val),
            custom_reg::agnus::SPR4PTH => self.agnus.set_sprpth(4, val),
            custom_reg::agnus::SPR4PTL => self.agnus.set_sprptl(4, val),
            custom_reg::agnus::SPR5PTH => self.agnus.set_sprpth(5, val),
            custom_reg::agnus::SPR5PTL => self.agnus.set_sprptl(5, val),
            custom_reg::agnus::SPR6PTH => self.agnus.set_sprpth(6, val),
            custom_reg::agnus::SPR6PTL => self.agnus.set_sprptl(6, val),
            custom_reg::agnus::SPR7PTH => self.agnus.set_sprpth(7, val),
            custom_reg::agnus::SPR7PTL => self.agnus.set_sprptl(7, val),
            _ => true,
        }
    }

    /// Writes an 8-bit byte to custom register space ($DF0000..$DFFFFF).
    ///
    /// Per M68000 bus reality and custom chip architecture, A0 is disconnected from
    /// custom chips and UDS/LDS are not gated; the byte is duplicated on both byte lanes
    /// (D15..D8 and D7..D0) and written as a 16-bit word `(val << 8) | val`.
    #[inline(always)]
    pub fn write_custom_byte(&mut self, addr: u32, val: u8) -> bool {
        let offset = (addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16;
        let word_val = ((val as u16) << 8) | (val as u16);
        self.write_custom_word(offset, word_val)
    }
}
