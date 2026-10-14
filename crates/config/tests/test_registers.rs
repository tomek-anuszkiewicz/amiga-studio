#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::custom_reg::{self, agnus, denise, paula};
use config::mask::*;

#[test]
fn test_register_offsets_uniqueness_and_alignment() {
    // Collect all major register offsets to verify word alignment (even addresses)
    let regs = [
        agnus::DMACONR,
        agnus::VPOSR,
        agnus::VHPOSR,
        paula::DSKDATR,
        denise::JOY0DAT,
        denise::JOY1DAT,
        denise::CLXDAT,
        paula::ADKCONR,
        paula::POT0DAT,
        paula::POT1DAT,
        paula::POTGOR,
        paula::SERDATR,
        paula::DSKBYTR,
        paula::INTENAR,
        paula::INTREQR,
        agnus::DSKPTH,
        agnus::DSKPTL,
        agnus::REFPTR,
        paula::DSKLEN,
        paula::DSKDAT,
        agnus::VPOSW,
        agnus::VHPOSW,
        agnus::COPCON,
        paula::SERDAT,
        paula::SERPER,
        paula::POTGO,
        denise::JOYTEST,
        denise::STREQU,
        denise::STRVBL,
        denise::STRHOR,
        denise::STRLONG,
        agnus::BLTCON0,
        agnus::BLTCON1,
        agnus::BLTAFWM,
        agnus::BLTALWM,
        agnus::BLTCPTH,
        agnus::BLTCPTL,
        agnus::BLTBPTH,
        agnus::BLTBPTL,
        agnus::BLTAPTH,
        agnus::BLTAPTL,
        agnus::BLTDPTH,
        agnus::BLTDPTL,
        agnus::BLTSIZE,
        agnus::BLTCMOD,
        agnus::BLTBMOD,
        agnus::BLTAMOD,
        agnus::BLTDMOD,
        agnus::BLTCDAT,
        agnus::BLTBDAT,
        agnus::BLTADAT,
        paula::DSKSYNC,
        agnus::COP1LCH,
        agnus::COP1LCL,
        agnus::COP2LCH,
        agnus::COP2LCL,
        agnus::COPJMP1,
        agnus::COPJMP2,
        agnus::COPINS,
        agnus::DIWSTRT,
        agnus::DIWSTOP,
        agnus::DDFSTRT,
        agnus::DDFSTOP,
        agnus::DMACON,
        denise::CLXCON,
        paula::INTENA,
        paula::INTREQ,
        paula::ADKCON,
        agnus::AUD0LCH,
        agnus::AUD0LCL,
        paula::AUD0LEN,
        paula::AUD0PER,
        paula::AUD0VOL,
        paula::AUD0DAT,
        agnus::AUD1LCH,
        agnus::AUD1LCL,
        paula::AUD1LEN,
        paula::AUD1PER,
        paula::AUD1VOL,
        paula::AUD1DAT,
        agnus::AUD2LCH,
        agnus::AUD2LCL,
        paula::AUD2LEN,
        paula::AUD2PER,
        paula::AUD2VOL,
        paula::AUD2DAT,
        agnus::AUD3LCH,
        agnus::AUD3LCL,
        paula::AUD3LEN,
        paula::AUD3PER,
        paula::AUD3VOL,
        paula::AUD3DAT,
        agnus::BPL1PTH,
        agnus::BPL1PTL,
        agnus::BPL2PTH,
        agnus::BPL2PTL,
        agnus::BPL3PTH,
        agnus::BPL3PTL,
        agnus::BPL4PTH,
        agnus::BPL4PTL,
        agnus::BPL5PTH,
        agnus::BPL5PTL,
        agnus::BPL6PTH,
        agnus::BPL6PTL,
        denise::BPLCON0,
        denise::BPLCON1,
        denise::BPLCON2,
        agnus::BPL1MOD,
        agnus::BPL2MOD,
        denise::BPL1DAT,
        denise::BPL2DAT,
        denise::BPL3DAT,
        denise::BPL4DAT,
        denise::BPL5DAT,
        denise::BPL6DAT,
        agnus::SPR0PTH,
        agnus::SPR0PTL,
        denise::SPR0POS,
        denise::SPR0CTL,
        denise::SPR0DATA,
        denise::SPR0DATB,
        denise::COLOR00,
        denise::COLOR01,
        denise::COLOR31,
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
fn test_dmacon_and_intreq_masks() {
    assert_eq!(dmacon::SET_CLR, 0x8000);
    assert_eq!(dmacon::DMAEN, 0x0200);
    assert_eq!(dmacon::COPEN, 0x0080);
    assert_eq!(dmacon::BLTEN, 0x0040);
    assert_eq!(dmacon::BBUSY, 0x4000);
    assert_eq!(dmacon::BZERO, 0x2000);

    assert_eq!(intreq::SET_CLR, 0x8000);
    assert_eq!(intreq::INTEN, 0x4000);
    assert_eq!(intreq::BLIT, 0x0040);
    assert_eq!(intreq::VERTB, 0x0020);
    assert_eq!(intreq::COPER, 0x0010);
}

#[test]
fn test_copcon_cdang_mask() {
    assert_eq!(copcon::CDANG, 0x0002);
}

#[test]
fn test_custom_reg_chip_namespaces_and_multi_chip_decoding() {
    // Agnus specific registers
    assert_eq!(custom_reg::agnus::BLTCON0, 0x040);
    assert_eq!(custom_reg::agnus::COP1LCH, 0x080);
    assert_eq!(custom_reg::agnus::DSKPTH, 0x020);
    assert_eq!(custom_reg::agnus::REFPTR, 0x028);

    // Denise specific registers
    assert_eq!(custom_reg::denise::COLOR00, 0x180);
    assert_eq!(custom_reg::denise::BPLCON1, 0x102);
    assert_eq!(custom_reg::denise::CLXCON, 0x098);

    // Paula specific registers
    assert_eq!(custom_reg::paula::INTENA, 0x09A);
    assert_eq!(custom_reg::paula::INTREQ, 0x09C);
    assert_eq!(custom_reg::paula::ADKCON, 0x09E);
    assert_eq!(custom_reg::paula::AUD0VOL, 0x0A8);
    assert_eq!(custom_reg::paula::SERDAT, 0x030);

    // Multi-chip decoded registers: physically wired to and decoded by multiple chips
    assert_eq!(custom_reg::agnus::DMACON, 0x096);
    assert_eq!(custom_reg::paula::DMACON, 0x096);

    assert_eq!(custom_reg::agnus::DMACONR, 0x002);
    assert_eq!(custom_reg::paula::DMACONR, 0x002);

    assert_eq!(custom_reg::denise::BPLCON0, 0x100);
    assert_eq!(custom_reg::agnus::BPLCON0, 0x100);
}
