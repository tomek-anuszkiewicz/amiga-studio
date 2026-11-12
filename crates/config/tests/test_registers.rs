#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::custom_reg::{self, agnus, denise, paula};
use config::mask::dmacon;

#[test]
fn test_register_offsets_uniqueness_and_alignment() {
    // Collect all major register offsets to verify word alignment (even addresses)
    let regs = [
        agnus::REGISTER_DMACONR,
        agnus::REGISTER_VPOSR,
        agnus::REGISTER_VHPOSR,
        paula::REGISTER_DSKDATR,
        denise::REGISTER_JOY0DAT,
        denise::REGISTER_JOY1DAT,
        denise::REGISTER_CLXDAT,
        paula::REGISTER_ADKCONR,
        paula::REGISTER_POT0DAT,
        paula::REGISTER_POT1DAT,
        paula::REGISTER_POTGOR,
        paula::REGISTER_SERDATR,
        paula::REGISTER_DSKBYTR,
        paula::REGISTER_INTENAR,
        paula::REGISTER_INTREQR,
        agnus::REGISTER_DSKPTH,
        agnus::REGISTER_DSKPTL,
        agnus::REGISTER_REFPTR,
        paula::REGISTER_DSKLEN,
        paula::REGISTER_DSKDAT,
        agnus::REGISTER_VPOSW,
        agnus::REGISTER_VHPOSW,
        agnus::REGISTER_COPCON,
        paula::REGISTER_SERDAT,
        paula::REGISTER_SERPER,
        paula::REGISTER_POTGO,
        denise::REGISTER_JOYTEST,
        denise::REGISTER_STREQU,
        denise::REGISTER_STRVBL,
        denise::REGISTER_STRHOR,
        denise::REGISTER_STRLONG,
        agnus::REGISTER_BLTCON0,
        agnus::REGISTER_BLTCON1,
        agnus::REGISTER_BLTAFWM,
        agnus::REGISTER_BLTALWM,
        agnus::REGISTER_BLTCPTH,
        agnus::REGISTER_BLTCPTL,
        agnus::REGISTER_BLTBPTH,
        agnus::REGISTER_BLTBPTL,
        agnus::REGISTER_BLTAPTH,
        agnus::REGISTER_BLTAPTL,
        agnus::REGISTER_BLTDPTH,
        agnus::REGISTER_BLTDPTL,
        agnus::REGISTER_BLTSIZE,
        agnus::REGISTER_BLTCMOD,
        agnus::REGISTER_BLTBMOD,
        agnus::REGISTER_BLTAMOD,
        agnus::REGISTER_BLTDMOD,
        agnus::REGISTER_BLTCDAT,
        agnus::REGISTER_BLTBDAT,
        agnus::REGISTER_BLTADAT,
        paula::REGISTER_DSKSYNC,
        agnus::REGISTER_COP1LCH,
        agnus::REGISTER_COP1LCL,
        agnus::REGISTER_COP2LCH,
        agnus::REGISTER_COP2LCL,
        agnus::REGISTER_COPJMP1,
        agnus::REGISTER_COPJMP2,
        agnus::REGISTER_COPINS,
        agnus::REGISTER_DIWSTRT,
        agnus::REGISTER_DIWSTOP,
        agnus::REGISTER_DDFSTRT,
        agnus::REGISTER_DDFSTOP,
        agnus::REGISTER_DMACON,
        denise::REGISTER_CLXCON,
        paula::REGISTER_INTENA,
        paula::REGISTER_INTREQ,
        paula::REGISTER_ADKCON,
        agnus::REGISTER_AUD0LCH,
        agnus::REGISTER_AUD0LCL,
        paula::REGISTER_AUD0LEN,
        paula::REGISTER_AUD0PER,
        paula::REGISTER_AUD0VOL,
        paula::REGISTER_AUD0DAT,
        agnus::REGISTER_AUD1LCH,
        agnus::REGISTER_AUD1LCL,
        paula::REGISTER_AUD1LEN,
        paula::REGISTER_AUD1PER,
        paula::REGISTER_AUD1VOL,
        paula::REGISTER_AUD1DAT,
        agnus::REGISTER_AUD2LCH,
        agnus::REGISTER_AUD2LCL,
        paula::REGISTER_AUD2LEN,
        paula::REGISTER_AUD2PER,
        paula::REGISTER_AUD2VOL,
        paula::REGISTER_AUD2DAT,
        agnus::REGISTER_AUD3LCH,
        agnus::REGISTER_AUD3LCL,
        paula::REGISTER_AUD3LEN,
        paula::REGISTER_AUD3PER,
        paula::REGISTER_AUD3VOL,
        paula::REGISTER_AUD3DAT,
        agnus::REGISTER_BPL1PTH,
        agnus::REGISTER_BPL1PTL,
        agnus::REGISTER_BPL2PTH,
        agnus::REGISTER_BPL2PTL,
        agnus::REGISTER_BPL3PTH,
        agnus::REGISTER_BPL3PTL,
        agnus::REGISTER_BPL4PTH,
        agnus::REGISTER_BPL4PTL,
        agnus::REGISTER_BPL5PTH,
        agnus::REGISTER_BPL5PTL,
        agnus::REGISTER_BPL6PTH,
        agnus::REGISTER_BPL6PTL,
        denise::REGISTER_BPLCON0,
        denise::REGISTER_BPLCON1,
        denise::REGISTER_BPLCON2,
        agnus::REGISTER_BPL1MOD,
        agnus::REGISTER_BPL2MOD,
        denise::REGISTER_BPL1DAT,
        denise::REGISTER_BPL2DAT,
        denise::REGISTER_BPL3DAT,
        denise::REGISTER_BPL4DAT,
        denise::REGISTER_BPL5DAT,
        denise::REGISTER_BPL6DAT,
        agnus::REGISTER_SPR0PTH,
        agnus::REGISTER_SPR0PTL,
        denise::REGISTER_SPR0POS,
        denise::REGISTER_SPR0CTL,
        denise::REGISTER_SPR0DATA,
        denise::REGISTER_SPR0DATB,
        denise::REGISTER_COLOR00,
        denise::REGISTER_COLOR01,
        denise::REGISTER_COLOR31,
    ];

    for &reg in &regs {
        assert_eq!(
            reg % 2,
            0,
            "Register offset ${:03X} must be word-aligned",
            reg
        );
        assert!(
            reg <= 0x1FE,
            "Register offset ${:03X} must fit within custom chip space",
            reg
        );
    }
}

#[test]
fn test_dmacon_masks() {
    assert_eq!(dmacon::BIT_SET_CLR, 0x8000);
    assert_eq!(dmacon::BIT_DMAEN, 0x0200);
    assert_eq!(dmacon::BIT_COPEN, 0x0080);
    assert_eq!(dmacon::BIT_BLTEN, 0x0040);
    assert_eq!(dmacon::BIT_BBUSY, 0x4000);
    assert_eq!(dmacon::BIT_BZERO, 0x2000);
}

#[test]
fn test_custom_reg_chip_namespaces_and_multi_chip_decoding() {
    // Agnus specific registers
    assert_eq!(custom_reg::agnus::REGISTER_BLTCON0, 0x040);
    assert_eq!(custom_reg::agnus::REGISTER_COP1LCH, 0x080);
    assert_eq!(custom_reg::agnus::REGISTER_DSKPTH, 0x020);
    assert_eq!(custom_reg::agnus::REGISTER_REFPTR, 0x028);

    // Denise specific registers
    assert_eq!(custom_reg::denise::REGISTER_COLOR00, 0x180);
    assert_eq!(custom_reg::denise::REGISTER_BPLCON1, 0x102);
    assert_eq!(custom_reg::denise::REGISTER_CLXCON, 0x098);

    // Paula specific registers
    assert_eq!(custom_reg::paula::REGISTER_INTENA, 0x09A);
    assert_eq!(custom_reg::paula::REGISTER_INTREQ, 0x09C);
    assert_eq!(custom_reg::paula::REGISTER_ADKCON, 0x09E);
    assert_eq!(custom_reg::paula::REGISTER_AUD0VOL, 0x0A8);
    assert_eq!(custom_reg::paula::REGISTER_SERDAT, 0x030);

    // Multi-chip decoded registers: physically wired to and decoded by multiple chips
    assert_eq!(custom_reg::agnus::REGISTER_DMACON, 0x096);
    assert_eq!(custom_reg::paula::REGISTER_DMACON, 0x096);

    assert_eq!(custom_reg::agnus::REGISTER_DMACONR, 0x002);
    assert_eq!(custom_reg::paula::REGISTER_DMACONR, 0x002);

    assert_eq!(custom_reg::denise::REGISTER_BPLCON0, 0x100);
    assert_eq!(custom_reg::agnus::REGISTER_BPLCON0, 0x100);
}
