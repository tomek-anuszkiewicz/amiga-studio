//! Live, debug and queued custom-register bus access.

use super::MemoryBus;
use config::{custom_reg, OPEN_BUS_WORD};

impl MemoryBus<'_> {
    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT, DSKBYTR)
    #[inline]
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & config::MASK_CUSTOM_REG_OFFSET;
        match offset {
            // Early DMA reads are not ordinary CPU-readable registers.
            custom_reg::agnus::REGISTER_BLTDDAT | custom_reg::paula::REGISTER_DSKDATR => {
                OPEN_BUS_WORD
            }
            custom_reg::agnus::REGISTER_DMACONR => self.agnus.dmaconr(),
            custom_reg::agnus::REGISTER_VPOSR => self.agnus.vposr(),
            custom_reg::agnus::REGISTER_VHPOSR => self.agnus.vhposr(),
            custom_reg::denise::REGISTER_JOY0DAT => self.denise.joy0dat(),
            custom_reg::denise::REGISTER_JOY1DAT => self.denise.joy1dat(),
            custom_reg::denise::REGISTER_CLXDAT => self.denise.clxdat(),
            custom_reg::paula::REGISTER_ADKCONR => self.paula.adkconr(),
            custom_reg::paula::REGISTER_POT0DAT => self.paula.pot0dat(),
            custom_reg::paula::REGISTER_POT1DAT => self.paula.pot1dat(),
            custom_reg::paula::REGISTER_POTGOR => self.paula.potgor(),
            custom_reg::paula::REGISTER_SERDATR => self.paula.serdatr(),
            custom_reg::paula::REGISTER_DSKBYTR => self.paula.read_dskbytr(),
            custom_reg::paula::REGISTER_INTENAR => self.paula.intenar(),
            custom_reg::paula::REGISTER_INTREQR => self.paula.intreqr(),
            custom_reg::agnus::REGISTER_COPJMP1 => {
                self.agnus.strobe_copjmp1();
                OPEN_BUS_WORD
            }
            custom_reg::agnus::REGISTER_COPJMP2 => {
                self.agnus.strobe_copjmp2();
                OPEN_BUS_WORD
            }
            _ => OPEN_BUS_WORD,
        }
    }

    /// Reads a 16-bit custom register without side-effects for debugging inspection
    #[inline]
    pub fn read_custom_word_debug(&self, offset: u16) -> u16 {
        let offset = offset & config::MASK_CUSTOM_REG_OFFSET;
        match offset {
            custom_reg::agnus::REGISTER_DMACONR => self.agnus.dmaconr_debug(),
            custom_reg::agnus::REGISTER_VPOSR => self.agnus.vposr_debug(),
            custom_reg::agnus::REGISTER_VHPOSR => self.agnus.vhposr_debug(),
            custom_reg::denise::REGISTER_JOY0DAT => self.denise.joy0dat_debug(),
            custom_reg::denise::REGISTER_JOY1DAT => self.denise.joy1dat_debug(),
            custom_reg::denise::REGISTER_CLXDAT => self.denise.clxdat_debug(),
            custom_reg::paula::REGISTER_ADKCONR => self.paula.adkconr_debug(),
            custom_reg::paula::REGISTER_POT0DAT => self.paula.pot0dat_debug(),
            custom_reg::paula::REGISTER_POT1DAT => self.paula.pot1dat_debug(),
            custom_reg::paula::REGISTER_POTGOR => self.paula.potgor_debug(),
            custom_reg::paula::REGISTER_SERDATR => self.paula.serdatr_debug(),
            custom_reg::paula::REGISTER_DSKBYTR => self.paula.peek_dskbytr(),
            custom_reg::paula::REGISTER_INTENAR => self.paula.intenar_debug(),
            custom_reg::paula::REGISTER_INTREQR => self.paula.intreqr_debug(),
            _ => OPEN_BUS_WORD,
        }
    }

    /// Applies address strobes immediately and queues accepted data transfers.
    /// False means no receiver accepted it; the producer must retry.
    pub fn write_custom_word(&mut self, offset: u16, val: u16) -> bool {
        let offset = offset & config::MASK_CUSTOM_REG_OFFSET;
        match offset {
            custom_reg::denise::REGISTER_STREQU => {
                self.denise.strobe_strequ();
                true
            }
            custom_reg::denise::REGISTER_STRVBL => {
                self.denise.strobe_strvbl();
                true
            }
            custom_reg::denise::REGISTER_STRLONG => {
                self.denise.strobe_strlong();
                true
            }
            custom_reg::denise::REGISTER_STRHOR => {
                self.denise.strobe_strhor();
                self.paula.strobe_strhor();
                true
            }
            custom_reg::agnus::REGISTER_DMACON => {
                if !self.agnus.can_accept_register_write()
                    || !self.paula.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_dmacon(val) && self.paula.set_dmacon(val)
            }
            custom_reg::denise::REGISTER_BPLCON0 => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_bplcon0(val) && self.denise.set_bplcon0(val)
            }
            custom_reg::agnus::REGISTER_DIWSTRT => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_diwstrt(val) && self.denise.set_diwstrt(val)
            }
            custom_reg::agnus::REGISTER_DIWSTOP => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_diwstop(val) && self.denise.set_diwstop(val)
            }
            custom_reg::denise::REGISTER_SPR0POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(0, val) && self.denise.set_sprpos(0, val)
            }
            custom_reg::denise::REGISTER_SPR0CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(0, val) && self.denise.set_sprctl(0, val)
            }
            custom_reg::denise::REGISTER_SPR1POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(1, val) && self.denise.set_sprpos(1, val)
            }
            custom_reg::denise::REGISTER_SPR1CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(1, val) && self.denise.set_sprctl(1, val)
            }
            custom_reg::denise::REGISTER_SPR2POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(2, val) && self.denise.set_sprpos(2, val)
            }
            custom_reg::denise::REGISTER_SPR2CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(2, val) && self.denise.set_sprctl(2, val)
            }
            custom_reg::denise::REGISTER_SPR3POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(3, val) && self.denise.set_sprpos(3, val)
            }
            custom_reg::denise::REGISTER_SPR3CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(3, val) && self.denise.set_sprctl(3, val)
            }
            custom_reg::denise::REGISTER_SPR4POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(4, val) && self.denise.set_sprpos(4, val)
            }
            custom_reg::denise::REGISTER_SPR4CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(4, val) && self.denise.set_sprctl(4, val)
            }
            custom_reg::denise::REGISTER_SPR5POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(5, val) && self.denise.set_sprpos(5, val)
            }
            custom_reg::denise::REGISTER_SPR5CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(5, val) && self.denise.set_sprctl(5, val)
            }
            custom_reg::denise::REGISTER_SPR6POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(6, val) && self.denise.set_sprpos(6, val)
            }
            custom_reg::denise::REGISTER_SPR6CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(6, val) && self.denise.set_sprctl(6, val)
            }
            custom_reg::denise::REGISTER_SPR7POS => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprpos(7, val) && self.denise.set_sprpos(7, val)
            }
            custom_reg::denise::REGISTER_SPR7CTL => {
                if !self.agnus.can_accept_register_write()
                    || !self.denise.can_accept_register_write()
                {
                    return false;
                }
                self.agnus.set_sprctl(7, val) && self.denise.set_sprctl(7, val)
            }
            custom_reg::denise::REGISTER_BPLCON1 => self.denise.set_bplcon1(val),
            custom_reg::denise::REGISTER_CLXCON => self.denise.set_clxcon(val),
            custom_reg::denise::REGISTER_BPLCON2 => self.denise.set_bplcon2(val),
            custom_reg::denise::REGISTER_BPL1DAT => self.denise.set_bpldat(0, val),
            custom_reg::denise::REGISTER_BPL2DAT => self.denise.set_bpldat(1, val),
            custom_reg::denise::REGISTER_BPL3DAT => self.denise.set_bpldat(2, val),
            custom_reg::denise::REGISTER_BPL4DAT => self.denise.set_bpldat(3, val),
            custom_reg::denise::REGISTER_BPL5DAT => self.denise.set_bpldat(4, val),
            custom_reg::denise::REGISTER_BPL6DAT => self.denise.set_bpldat(5, val),
            custom_reg::denise::REGISTER_SPR0DATA => self.denise.set_sprdata(0, val),
            custom_reg::denise::REGISTER_SPR0DATB => self.denise.set_sprdatb(0, val),
            custom_reg::denise::REGISTER_SPR1DATA => self.denise.set_sprdata(1, val),
            custom_reg::denise::REGISTER_SPR1DATB => self.denise.set_sprdatb(1, val),
            custom_reg::denise::REGISTER_SPR2DATA => self.denise.set_sprdata(2, val),
            custom_reg::denise::REGISTER_SPR2DATB => self.denise.set_sprdatb(2, val),
            custom_reg::denise::REGISTER_SPR3DATA => self.denise.set_sprdata(3, val),
            custom_reg::denise::REGISTER_SPR3DATB => self.denise.set_sprdatb(3, val),
            custom_reg::denise::REGISTER_SPR4DATA => self.denise.set_sprdata(4, val),
            custom_reg::denise::REGISTER_SPR4DATB => self.denise.set_sprdatb(4, val),
            custom_reg::denise::REGISTER_SPR5DATA => self.denise.set_sprdata(5, val),
            custom_reg::denise::REGISTER_SPR5DATB => self.denise.set_sprdatb(5, val),
            custom_reg::denise::REGISTER_SPR6DATA => self.denise.set_sprdata(6, val),
            custom_reg::denise::REGISTER_SPR6DATB => self.denise.set_sprdatb(6, val),
            custom_reg::denise::REGISTER_SPR7DATA => self.denise.set_sprdata(7, val),
            custom_reg::denise::REGISTER_SPR7DATB => self.denise.set_sprdatb(7, val),
            custom_reg::denise::REGISTER_COLOR00 => self.denise.set_color(0, val),
            custom_reg::denise::REGISTER_COLOR01 => self.denise.set_color(1, val),
            custom_reg::denise::REGISTER_COLOR02 => self.denise.set_color(2, val),
            custom_reg::denise::REGISTER_COLOR03 => self.denise.set_color(3, val),
            custom_reg::denise::REGISTER_COLOR04 => self.denise.set_color(4, val),
            custom_reg::denise::REGISTER_COLOR05 => self.denise.set_color(5, val),
            custom_reg::denise::REGISTER_COLOR06 => self.denise.set_color(6, val),
            custom_reg::denise::REGISTER_COLOR07 => self.denise.set_color(7, val),
            custom_reg::denise::REGISTER_COLOR08 => self.denise.set_color(8, val),
            custom_reg::denise::REGISTER_COLOR09 => self.denise.set_color(9, val),
            custom_reg::denise::REGISTER_COLOR10 => self.denise.set_color(10, val),
            custom_reg::denise::REGISTER_COLOR11 => self.denise.set_color(11, val),
            custom_reg::denise::REGISTER_COLOR12 => self.denise.set_color(12, val),
            custom_reg::denise::REGISTER_COLOR13 => self.denise.set_color(13, val),
            custom_reg::denise::REGISTER_COLOR14 => self.denise.set_color(14, val),
            custom_reg::denise::REGISTER_COLOR15 => self.denise.set_color(15, val),
            custom_reg::denise::REGISTER_COLOR16 => self.denise.set_color(16, val),
            custom_reg::denise::REGISTER_COLOR17 => self.denise.set_color(17, val),
            custom_reg::denise::REGISTER_COLOR18 => self.denise.set_color(18, val),
            custom_reg::denise::REGISTER_COLOR19 => self.denise.set_color(19, val),
            custom_reg::denise::REGISTER_COLOR20 => self.denise.set_color(20, val),
            custom_reg::denise::REGISTER_COLOR21 => self.denise.set_color(21, val),
            custom_reg::denise::REGISTER_COLOR22 => self.denise.set_color(22, val),
            custom_reg::denise::REGISTER_COLOR23 => self.denise.set_color(23, val),
            custom_reg::denise::REGISTER_COLOR24 => self.denise.set_color(24, val),
            custom_reg::denise::REGISTER_COLOR25 => self.denise.set_color(25, val),
            custom_reg::denise::REGISTER_COLOR26 => self.denise.set_color(26, val),
            custom_reg::denise::REGISTER_COLOR27 => self.denise.set_color(27, val),
            custom_reg::denise::REGISTER_COLOR28 => self.denise.set_color(28, val),
            custom_reg::denise::REGISTER_COLOR29 => self.denise.set_color(29, val),
            custom_reg::denise::REGISTER_COLOR30 => self.denise.set_color(30, val),
            custom_reg::denise::REGISTER_COLOR31 => self.denise.set_color(31, val),
            custom_reg::denise::REGISTER_JOYTEST => self.denise.set_joytest(val),
            custom_reg::paula::REGISTER_INTENA => self.paula.set_intena(val),
            custom_reg::paula::REGISTER_INTREQ => self.paula.set_intreq(val),
            custom_reg::paula::REGISTER_ADKCON => self.paula.set_adkcon(val),
            custom_reg::paula::REGISTER_DSKDAT => self.paula.set_dskdat(val),
            custom_reg::paula::REGISTER_DSKLEN => self.paula.set_dsklen(val),
            custom_reg::paula::REGISTER_DSKSYNC => self.paula.set_dsksync(val),
            custom_reg::paula::REGISTER_SERDAT => self.paula.set_serdat(val),
            custom_reg::paula::REGISTER_SERPER => self.paula.set_serper(val),
            custom_reg::paula::REGISTER_POTGO => self.paula.set_potgo(val),
            custom_reg::paula::REGISTER_AUD0LEN => self.paula.set_audlen(0, val),
            custom_reg::paula::REGISTER_AUD0PER => self.paula.set_audper(0, val),
            custom_reg::paula::REGISTER_AUD0VOL => self.paula.set_audvol(0, val),
            custom_reg::paula::REGISTER_AUD0DAT => self.paula.set_auddat(0, val),
            custom_reg::paula::REGISTER_AUD1LEN => self.paula.set_audlen(1, val),
            custom_reg::paula::REGISTER_AUD1PER => self.paula.set_audper(1, val),
            custom_reg::paula::REGISTER_AUD1VOL => self.paula.set_audvol(1, val),
            custom_reg::paula::REGISTER_AUD1DAT => self.paula.set_auddat(1, val),
            custom_reg::paula::REGISTER_AUD2LEN => self.paula.set_audlen(2, val),
            custom_reg::paula::REGISTER_AUD2PER => self.paula.set_audper(2, val),
            custom_reg::paula::REGISTER_AUD2VOL => self.paula.set_audvol(2, val),
            custom_reg::paula::REGISTER_AUD2DAT => self.paula.set_auddat(2, val),
            custom_reg::paula::REGISTER_AUD3LEN => self.paula.set_audlen(3, val),
            custom_reg::paula::REGISTER_AUD3PER => self.paula.set_audper(3, val),
            custom_reg::paula::REGISTER_AUD3VOL => self.paula.set_audvol(3, val),
            custom_reg::paula::REGISTER_AUD3DAT => self.paula.set_auddat(3, val),
            custom_reg::agnus::REGISTER_DSKPTH => self.agnus.set_dskpth(val),
            custom_reg::agnus::REGISTER_DSKPTL => self.agnus.set_dskptl(val),
            custom_reg::agnus::REGISTER_REFPTR => self.agnus.set_refptr(val),
            custom_reg::agnus::REGISTER_VPOSW => self.agnus.set_vposw(val),
            custom_reg::agnus::REGISTER_VHPOSW => self.agnus.set_vhposw(val),
            custom_reg::agnus::REGISTER_COPCON => self.agnus.set_copcon(val),
            custom_reg::agnus::REGISTER_BLTCON0 => self.agnus.set_bltcon0(val),
            custom_reg::agnus::REGISTER_BLTCON1 => self.agnus.set_bltcon1(val),
            custom_reg::agnus::REGISTER_BLTAFWM => self.agnus.set_bltafwm(val),
            custom_reg::agnus::REGISTER_BLTALWM => self.agnus.set_bltalwm(val),
            custom_reg::agnus::REGISTER_BLTCPTH => self.agnus.set_bltcpth(val),
            custom_reg::agnus::REGISTER_BLTCPTL => self.agnus.set_bltcptl(val),
            custom_reg::agnus::REGISTER_BLTBPTH => self.agnus.set_bltbpth(val),
            custom_reg::agnus::REGISTER_BLTBPTL => self.agnus.set_bltbptl(val),
            custom_reg::agnus::REGISTER_BLTAPTH => self.agnus.set_bltapth(val),
            custom_reg::agnus::REGISTER_BLTAPTL => self.agnus.set_bltaptl(val),
            custom_reg::agnus::REGISTER_BLTDPTH => self.agnus.set_bltdpth(val),
            custom_reg::agnus::REGISTER_BLTDPTL => self.agnus.set_bltdptl(val),
            custom_reg::agnus::REGISTER_BLTSIZE => self.agnus.set_bltsize(val),
            custom_reg::agnus::REGISTER_BLTCMOD => self.agnus.set_bltcmod(val),
            custom_reg::agnus::REGISTER_BLTBMOD => self.agnus.set_bltbmod(val),
            custom_reg::agnus::REGISTER_BLTAMOD => self.agnus.set_bltamod(val),
            custom_reg::agnus::REGISTER_BLTDMOD => self.agnus.set_bltdmod(val),
            custom_reg::agnus::REGISTER_BLTCDAT => self.agnus.set_bltcdat(val),
            custom_reg::agnus::REGISTER_BLTBDAT => self.agnus.set_bltbdat(val),
            custom_reg::agnus::REGISTER_BLTADAT => self.agnus.set_bltadat(val),
            custom_reg::agnus::REGISTER_COP1LCH => self.agnus.set_cop1lch(val),
            custom_reg::agnus::REGISTER_COP1LCL => self.agnus.set_cop1lcl(val),
            custom_reg::agnus::REGISTER_COP2LCH => self.agnus.set_cop2lch(val),
            custom_reg::agnus::REGISTER_COP2LCL => self.agnus.set_cop2lcl(val),
            custom_reg::agnus::REGISTER_COPJMP1 => {
                self.agnus.strobe_copjmp1();
                true
            }
            custom_reg::agnus::REGISTER_COPJMP2 => {
                self.agnus.strobe_copjmp2();
                true
            }
            custom_reg::agnus::REGISTER_COPINS => self.agnus.set_copins(val),
            custom_reg::agnus::REGISTER_DDFSTRT => self.agnus.set_ddfstrt(val),
            custom_reg::agnus::REGISTER_DDFSTOP => self.agnus.set_ddfstop(val),
            custom_reg::agnus::REGISTER_AUD0LCH => self.agnus.set_audlch(0, val),
            custom_reg::agnus::REGISTER_AUD0LCL => self.agnus.set_audlcl(0, val),
            custom_reg::agnus::REGISTER_AUD1LCH => self.agnus.set_audlch(1, val),
            custom_reg::agnus::REGISTER_AUD1LCL => self.agnus.set_audlcl(1, val),
            custom_reg::agnus::REGISTER_AUD2LCH => self.agnus.set_audlch(2, val),
            custom_reg::agnus::REGISTER_AUD2LCL => self.agnus.set_audlcl(2, val),
            custom_reg::agnus::REGISTER_AUD3LCH => self.agnus.set_audlch(3, val),
            custom_reg::agnus::REGISTER_AUD3LCL => self.agnus.set_audlcl(3, val),
            custom_reg::agnus::REGISTER_BPL1PTH => self.agnus.set_bplpth(0, val),
            custom_reg::agnus::REGISTER_BPL1PTL => self.agnus.set_bplptl(0, val),
            custom_reg::agnus::REGISTER_BPL2PTH => self.agnus.set_bplpth(1, val),
            custom_reg::agnus::REGISTER_BPL2PTL => self.agnus.set_bplptl(1, val),
            custom_reg::agnus::REGISTER_BPL3PTH => self.agnus.set_bplpth(2, val),
            custom_reg::agnus::REGISTER_BPL3PTL => self.agnus.set_bplptl(2, val),
            custom_reg::agnus::REGISTER_BPL4PTH => self.agnus.set_bplpth(3, val),
            custom_reg::agnus::REGISTER_BPL4PTL => self.agnus.set_bplptl(3, val),
            custom_reg::agnus::REGISTER_BPL5PTH => self.agnus.set_bplpth(4, val),
            custom_reg::agnus::REGISTER_BPL5PTL => self.agnus.set_bplptl(4, val),
            custom_reg::agnus::REGISTER_BPL6PTH => self.agnus.set_bplpth(5, val),
            custom_reg::agnus::REGISTER_BPL6PTL => self.agnus.set_bplptl(5, val),
            custom_reg::agnus::REGISTER_BPL1MOD => self.agnus.set_bpl1mod(val),
            custom_reg::agnus::REGISTER_BPL2MOD => self.agnus.set_bpl2mod(val),
            custom_reg::agnus::REGISTER_SPR0PTH => self.agnus.set_sprpth(0, val),
            custom_reg::agnus::REGISTER_SPR0PTL => self.agnus.set_sprptl(0, val),
            custom_reg::agnus::REGISTER_SPR1PTH => self.agnus.set_sprpth(1, val),
            custom_reg::agnus::REGISTER_SPR1PTL => self.agnus.set_sprptl(1, val),
            custom_reg::agnus::REGISTER_SPR2PTH => self.agnus.set_sprpth(2, val),
            custom_reg::agnus::REGISTER_SPR2PTL => self.agnus.set_sprptl(2, val),
            custom_reg::agnus::REGISTER_SPR3PTH => self.agnus.set_sprpth(3, val),
            custom_reg::agnus::REGISTER_SPR3PTL => self.agnus.set_sprptl(3, val),
            custom_reg::agnus::REGISTER_SPR4PTH => self.agnus.set_sprpth(4, val),
            custom_reg::agnus::REGISTER_SPR4PTL => self.agnus.set_sprptl(4, val),
            custom_reg::agnus::REGISTER_SPR5PTH => self.agnus.set_sprpth(5, val),
            custom_reg::agnus::REGISTER_SPR5PTL => self.agnus.set_sprptl(5, val),
            custom_reg::agnus::REGISTER_SPR6PTH => self.agnus.set_sprpth(6, val),
            custom_reg::agnus::REGISTER_SPR6PTL => self.agnus.set_sprptl(6, val),
            custom_reg::agnus::REGISTER_SPR7PTH => self.agnus.set_sprpth(7, val),
            custom_reg::agnus::REGISTER_SPR7PTL => self.agnus.set_sprptl(7, val),
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
        let offset = (addr & config::MASK_CUSTOM_REG_OFFSET as u32) as u16;
        let word_val = ((val as u16) << 8) | (val as u16);
        self.write_custom_word(offset, word_val)
    }
}
